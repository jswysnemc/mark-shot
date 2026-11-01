#include "platform/wayland/wlroots_screencopy_capture_stream.h"

#include <QPainter>
#include <QRasterWindow>
#include <QTimer>
#include <QtTest/QtTest>

using namespace markshot::recording;

class RecordingTestSurface final : public QRasterWindow {
protected:
    /** 绘制不断变化的测试色块，event 为重绘事件，无返回值。 */
    void paintEvent(QPaintEvent *event) override
    {
        Q_UNUSED(event)
        QPainter painter(this);
        painter.fillRect(QRect(QPoint(), size()), QColor((++m_frame * 17) % 256, 80, 120));
    }

private:
    int m_frame = 0;
};

class RecordingWlrootsCaptureTest final : public QObject {
    Q_OBJECT

private slots:
    /** 拒绝在非隔离桌面执行采集测试，无参数和返回值。 */
    void initTestCase()
    {
        const QString runtime = qEnvironmentVariable("MARK_SHOT_RECORDING_TEST_RUNTIME");
        QVERIFY2(!runtime.isEmpty(), "Run through recording_wayland_session_test.py");
        QVERIFY(qEnvironmentVariable("WAYLAND_DISPLAY").startsWith(runtime + QLatin1Char('/')));
        QCOMPARE(QGuiApplication::platformName(), QStringLiteral("wayland"));
    }

    /** 动态画面不能突破采集频率，背压恢复后继续出帧且停止后不再回调，无参数和返回值。 */
    void respectsCadenceAndBackpressure()
    {
        RecordingTestSurface surface;
        surface.resize(320, 240);
        surface.show();
        QVERIFY(QTest::qWaitForWindowExposed(&surface));
        QTimer animation;
        animation.setTimerType(Qt::PreciseTimer);
        connect(&animation, &QTimer::timeout, &surface, [&surface] { surface.update(); });
        animation.start(4);

        RecordingOptions options;
        options.scope = RecordingScope::Display;
        options.fps = 12;
        auto stream = createWlrootsScreencopyCaptureStream(options);
        QVERIFY(stream != nullptr);
        QString failure;
        int count = 0;
        bool valid = true;
        qint64 lastTimestamp = -1;
        connect(stream.get(), &RecordingCaptureStream::failed, this,
                [&failure](const QString &error) { failure = error; });
        connect(stream.get(), &RecordingCaptureStream::frameReady, this,
                [&](const RecordingFrameSample &sample) {
                    valid = valid && sample.hasFrameData() && sample.timestampMs > lastTimestamp;
                    lastTimestamp = sample.timestampMs;
                    ++count;
                });
        QString error;
        QVERIFY2(stream->start(&error), qPrintable(error));
        QTRY_VERIFY_WITH_TIMEOUT(count > 0 || !failure.isEmpty(), 5000);
        QVERIFY2(failure.isEmpty(), qPrintable(failure));

        // 1. 【录制测试】【采集限帧】只保留计数，不让测试本身持有共享内存帧
        const int before = count;
        QElapsedTimer elapsed;
        elapsed.start();
        QTest::qWait(2000);
        const double seconds = elapsed.elapsed() / 1000.0;
        const int captured = count - before;
        qInfo("【录制测试】【采集限帧】frames=%d seconds=%.3f target_fps=%d",
              captured, seconds, options.fps);
        QVERIFY2(captured <= seconds * options.fps + 2, "Capture exceeded target FPS");
        QVERIFY2(captured >= options.fps, "Capture stalled while the test surface was changing");

        // 2. 【录制测试】【采集背压】允许已提交的一帧完成，随后暂停请求并验证恢复
        stream->setBackpressureActive(true);
        QTest::qWait(200);
        const int pressured = count;
        QTest::qWait(250);
        QCOMPARE(count, pressured);
        stream->setBackpressureActive(false);
        QTRY_VERIFY_WITH_TIMEOUT(count > pressured, 2000);
        QVERIFY2(failure.isEmpty(), qPrintable(failure));
        QVERIFY(valid);

        // 3. 【录制测试】【采集停止】归还在途帧后停止，定时器不得继续发送请求
        stream->setBackpressureActive(true);
        QTest::qWait(200);
        elapsed.restart();
        stream->stop();
        QVERIFY(elapsed.elapsed() < 1000);
        const int stopped = count;
        QTest::qWait(200);
        QCOMPARE(count, stopped);
        QVERIFY2(failure.isEmpty(), qPrintable(failure));
    }

    /** 停止会话不能等待调用栈持有的帧，映射像素必须在 Wayland 连接关闭后仍可读取，无参数和返回值。 */
    void retainedPixelsSurviveStreamStop()
    {
        RecordingTestSurface surface;
        surface.resize(320, 240);
        surface.show();
        QVERIFY(QTest::qWaitForWindowExposed(&surface));
        RecordingOptions options;
        options.scope = RecordingScope::Display;
        auto stream = createWlrootsScreencopyCaptureStream(options);
        QVERIFY(stream != nullptr);
        RecordingFrameSample retained;
        QString failure;
        connect(stream.get(), &RecordingCaptureStream::frameReady, this,
                [&](const RecordingFrameSample &sample) { retained = sample; });
        connect(stream.get(), &RecordingCaptureStream::failed, this,
                [&](const QString &error) { failure = error; });
        QString error;
        QVERIFY2(stream->start(&error), qPrintable(error));
        QTRY_VERIFY_WITH_TIMEOUT(retained.hasFrameData() || !failure.isEmpty(), 5000);
        QVERIFY2(failure.isEmpty(), qPrintable(failure));
        QVERIFY(retained.bgra.mappedOwner != nullptr);
        RecordingFrameSample firstSession = retained;
        const std::weak_ptr<const void> firstLease = firstSession.bgra.mappedOwner;
        const QByteArray firstPixel(retained.bgra.constData(), 4);
        QElapsedTimer elapsed;
        elapsed.start();
        stream->stop();
        const qint64 stopMs = elapsed.elapsed();

        // 1. 【录制测试】【映射寿命】旧帧仍被借出时重启同一个适配器，新会话不能复用旧映射
        retained = {};
        QVERIFY2(stream->start(&error), qPrintable(error));
        QTRY_VERIFY_WITH_TIMEOUT(retained.hasFrameData() || !failure.isEmpty(), 5000);
        QVERIFY2(failure.isEmpty(), qPrintable(failure));
        QVERIFY(retained.bgra.constData() != firstSession.bgra.constData());
        stream->stop();
        stream.reset();
        QCOMPARE(QByteArray(firstSession.bgra.constData(), 4), firstPixel);
        firstSession = {};
        QVERIFY(firstLease.expired());
        retained = {};
        qInfo("【录制测试】【映射寿命】stop_ms=%lld", static_cast<long long>(stopMs));
        QVERIFY2(stopMs < 1000, "Stopping waited for a frame retained by the caller");
    }
};

QTEST_MAIN(RecordingWlrootsCaptureTest)
#include "recording_wlroots_capture_test.moc"
