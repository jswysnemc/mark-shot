#include "recording/recording_frame_grabber.h"

#include <QSignalSpy>
#include <QTimer>
#include <QtTest/QtTest>

#include <memory>

namespace {

enum class FakeStreamBehavior {
    FailBeforeFirstFrame,
    ProduceFrame,
    ProduceFrameThenFail,
};

class FakeCaptureStream final : public markshot::recording::RecordingCaptureStream {
public:
    /**
     * 创建测试采集流。
     * @param behavior 启动后的信号行为。
     * @param parent 父对象。
     */
    explicit FakeCaptureStream(FakeStreamBehavior behavior, QObject *parent = nullptr)
        : RecordingCaptureStream(parent)
        , m_behavior(behavior)
    {
    }

    /**
     * 启动测试采集流并在事件循环中发送预设信号。
     * @param error 输出错误信息。
     * @return 始终返回 true，模拟门户已经完成同步启动。
     */
    bool start(QString *error) override
    {
        if (error) {
            error->clear();
        }

        if (m_behavior == FakeStreamBehavior::FailBeforeFirstFrame) {
            QTimer::singleShot(0, this, [this] {
                emit failed(QStringLiteral("pipewire failed before first frame"));
            });
            return true;
        }

        QTimer::singleShot(0, this, [this] {
            markshot::recording::RecordingFrameSample sample;
            sample.image = QImage(2, 2, QImage::Format_ARGB32_Premultiplied);
            sample.sequence = 1;
            emit frameReady(sample);
        });
        if (m_behavior == FakeStreamBehavior::ProduceFrameThenFail) {
            QTimer::singleShot(0, this, [this] {
                emit failed(QStringLiteral("capture failed after first frame"));
            });
        }
        return true;
    }

    /**
     * 停止测试采集流。
     * @return 无返回值。
     */
    void stop() override
    {
    }

    /**
     * 接收背压状态。
     * @param active 是否暂停采集。
     * @return 无返回值。
     */
    void setBackpressureActive(bool active) override
    {
        m_backpressureActive = active;
    }

private:
    FakeStreamBehavior m_behavior;
    bool m_backpressureActive = false;
};

}  // namespace

class RecordingFrameGrabberTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 注册录制帧信号参数类型。
     * @return 无返回值。
     */
    void initTestCase()
    {
        qRegisterMetaType<markshot::recording::RecordingFrameSample>();
    }

    /**
     * 验证 PipeWire 在首帧前异步失败时自动回退到 Polling。
     * @return 无返回值。
     */
    void asynchronousFailureBeforeFirstFrameFallsBack()
    {
        QVector<markshot::recording::RecordingCaptureBackend> attemptedBackends;
        auto factory = [&attemptedBackends](markshot::recording::RecordingCaptureBackend backend,
                                            const markshot::recording::RecordingOptions &,
                                            QObject *parent)
            -> std::unique_ptr<markshot::recording::RecordingCaptureStream> {
            attemptedBackends.append(backend);
            if (backend == markshot::recording::RecordingCaptureBackend::PipeWire) {
                return std::make_unique<FakeCaptureStream>(
                    FakeStreamBehavior::FailBeforeFirstFrame,
                    parent);
            }
            if (backend == markshot::recording::RecordingCaptureBackend::Polling) {
                return std::make_unique<FakeCaptureStream>(FakeStreamBehavior::ProduceFrame,
                                                           parent);
            }
            return nullptr;
        };

        markshot::recording::RecordingOptions options;
        options.captureBackend = markshot::recording::RecordingCaptureBackend::PipeWire;
        markshot::recording::RecordingFrameGrabber grabber(options, factory);
        QSignalSpy frameSpy(&grabber,
                            &markshot::recording::RecordingFrameGrabber::frameReady);
        QSignalSpy failedSpy(&grabber,
                             &markshot::recording::RecordingFrameGrabber::failed);

        grabber.start();

        QTRY_VERIFY_WITH_TIMEOUT(frameSpy.count() > 0 || failedSpy.count() > 0, 250);
        QCOMPARE(frameSpy.count(), 1);
        QCOMPARE(failedSpy.count(), 0);
        QVERIFY(attemptedBackends.contains(
            markshot::recording::RecordingCaptureBackend::Polling));
    }

    /**
     * 验证采集流已经产生首帧后发生错误时不切换后端。
     * @return 无返回值。
     */
    void asynchronousFailureAfterFirstFrameIsReported()
    {
        QVector<markshot::recording::RecordingCaptureBackend> attemptedBackends;
        auto factory = [&attemptedBackends](markshot::recording::RecordingCaptureBackend backend,
                                            const markshot::recording::RecordingOptions &,
                                            QObject *parent)
            -> std::unique_ptr<markshot::recording::RecordingCaptureStream> {
            attemptedBackends.append(backend);
            if (backend == markshot::recording::RecordingCaptureBackend::PipeWire) {
                return std::make_unique<FakeCaptureStream>(
                    FakeStreamBehavior::ProduceFrameThenFail,
                    parent);
            }
            if (backend == markshot::recording::RecordingCaptureBackend::Polling) {
                return std::make_unique<FakeCaptureStream>(FakeStreamBehavior::ProduceFrame,
                                                           parent);
            }
            return nullptr;
        };

        markshot::recording::RecordingOptions options;
        options.captureBackend = markshot::recording::RecordingCaptureBackend::PipeWire;
        markshot::recording::RecordingFrameGrabber grabber(options, factory);
        QSignalSpy frameSpy(&grabber,
                            &markshot::recording::RecordingFrameGrabber::frameReady);
        QSignalSpy failedSpy(&grabber,
                             &markshot::recording::RecordingFrameGrabber::failed);

        grabber.start();

        QTRY_COMPARE_WITH_TIMEOUT(frameSpy.count(), 1, 250);
        QTRY_COMPARE_WITH_TIMEOUT(failedSpy.count(), 1, 250);
        QVERIFY(!attemptedBackends.contains(
            markshot::recording::RecordingCaptureBackend::Polling));
    }
};

QTEST_GUILESS_MAIN(RecordingFrameGrabberTest)

#include "recording_frame_grabber_test.moc"
