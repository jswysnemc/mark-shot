#include "recording/backend/backend_recording_writer.h"
#include "recording/recording_frame_handoff.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QStandardPaths>
#include <QTemporaryDir>
#include <QtTest/QtTest>

#include <cmath>

using namespace markshot::recording;

class RecordingVideoTimelineTest final : public QObject {
    Q_OBJECT

private slots:
    /** 固定软件编码并检查媒体探测工具，无参数和返回值。 */
    void initTestCase()
    {
        qputenv("MARK_SHOT_RECORDING_SW_ENCODER", "1");
        if (QStandardPaths::findExecutable(QStringLiteral("ffprobe")).isEmpty()) {
            QSKIP("ffprobe is required to verify decoded video timestamps");
        }
    }

    /** 提供输入刷新率、输出帧率及容器组合，无参数和返回值。 */
    void normalizesInputRate_data()
    {
        QTest::addColumn<int>("outputFps");
        QTest::addColumn<QString>("extension");
        QTest::addColumn<QList<qint64>>("timestamps");
        QTest::addColumn<QList<qint64>>("expectedIndices");
        QTest::addColumn<bool>("coalesced");
        for (const auto &extension : {QStringLiteral("mp4"), QStringLiteral("mkv")}) {
            for (int output : {12, 30, 60}) {
                for (int input : {60, 144}) {
                    const QByteArray name = QStringLiteral("%1-%2-to-%3")
                        .arg(extension).arg(input).arg(output).toLatin1();
                    QList<qint64> timestamps;
                    QList<qint64> expected;
                    for (int index = 0; index < input * 2; ++index) {
                        timestamps.append(5000 + static_cast<qint64>(index) * 1000 / input);
                    }
                    for (int index = 0; index < output * 2; ++index) {
                        expected.append(index);
                    }
                    QTest::newRow(name.constData()) << output << extension << timestamps << expected << false;
                    if (input == 60 && output == 30) {
                        QTest::newRow(qPrintable(extension + QStringLiteral("-handoff-backlog")))
                            << output << extension << timestamps << expected << true;
                    }
                }
            }
            QList<qint64> heartbeatIndices;
            for (int index = 0; index <= 30; ++index) {
                heartbeatIndices.append(index);
            }
            QTest::newRow(qPrintable(extension + QStringLiteral("-heartbeat-stale")))
                << 30 << extension << QList<qint64>{0, 400, 433, 399, 1000} << heartbeatIndices << false;
            QList<qint64> gapIndices;
            for (int index = 0; index <= 300; ++index) {
                gapIndices.append(index);
            }
            gapIndices.append({600, 601});
            QTest::newRow(qPrintable(extension + QStringLiteral("-bounded-long-gap")))
                << 30 << extension << QList<qint64>{0, 20000, 20033} << gapIndices << false;
        }
    }

    /** 编码高频或断流样本，解码后验证帧数、PTS、DTS 与持续时间，无参数和返回值。 */
    void normalizesInputRate()
    {
        QFETCH(int, outputFps);
        QFETCH(QString, extension);
        QFETCH(QList<qint64>, timestamps);
        QFETCH(QList<qint64>, expectedIndices);
        QFETCH(bool, coalesced);
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        RecordingOptions options;
        options.mode = RecordingMode::Video;
        options.includeAudio = false;
        options.container = extension == QStringLiteral("mp4")
            ? RecordingContainerFormat::Mp4 : RecordingContainerFormat::Mkv;
        options.outputPath = directory.filePath(QStringLiteral("paced.") + extension);
        QString error;
        auto writer = createBackendRecordingWriter(options, &error);
        QVERIFY2(writer != nullptr, qPrintable(error));
        QVERIFY2(writer->start(QSize(32, 24), outputFps, &error), qPrintable(error));
        RecordingFrameHandoff handoff;
        bool writesOk = true;
        int delivered = 0;
        connect(&handoff, &RecordingFrameHandoff::frameReady, this,
                [&](const RecordingFrameSample &sample) {
                    ++delivered;
                    writesOk = writesOk && writer->writeFrame(sample, &error);
                });
        const auto publish = handoff.publisher();
        for (int index = 0; index < timestamps.size(); ++index) {
            RecordingFrameSample sample;
            sample.image = QImage(32, 24, QImage::Format_ARGB32);
            sample.image.fill(QColor(index % 256, 90, 120));
            sample.timestampMs = timestamps.at(index);
            sample.sequence = index + 1;
            if (coalesced) {
                publish(std::move(sample));
                // 1. 【录制】【交接时间轴】先交付首帧建立时间原点，再模拟主线程长时间不处理帧通知
                if (index == 0) {
                    QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
                }
            } else {
                QVERIFY2(writer->writeFrame(sample, &error), qPrintable(error));
            }
        }
        if (coalesced) {
            QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
            QCOMPARE(delivered, 2);
            QVERIFY2(writesOk, qPrintable(error));
        }
        QVERIFY2(writer->finish(&error), qPrintable(error));
        writer.reset();

        // 1. 【录制】【时间轴测试】ffprobe 解码所有视频帧，而非依赖容器声明的帧数
        QProcess probe;
        probe.start(QStringLiteral("ffprobe"), {
            QStringLiteral("-v"), QStringLiteral("error"),
            QStringLiteral("-select_streams"), QStringLiteral("v:0"),
            QStringLiteral("-show_frames"), QStringLiteral("-show_format"),
            QStringLiteral("-show_entries"),
            QStringLiteral("frame=best_effort_timestamp_time,pkt_dts_time:format=duration"),
            QStringLiteral("-of"), QStringLiteral("json"), options.outputPath});
        QVERIFY(probe.waitForFinished(10000));
        QCOMPARE(probe.exitStatus(), QProcess::NormalExit);
        QCOMPARE(probe.exitCode(), 0);
        const auto json = QJsonDocument::fromJson(probe.readAllStandardOutput()).object();
        const auto frames = json.value(QStringLiteral("frames")).toArray();
        QCOMPARE(frames.size(), expectedIndices.size());
        double previousDts = -1.0;
        for (int index = 0; index < frames.size(); ++index) {
            const double pts = frames.at(index).toObject()
                .value(QStringLiteral("best_effort_timestamp_time")).toString().toDouble();
            QVERIFY2(std::abs(pts - static_cast<double>(expectedIndices.at(index)) / outputFps) < 0.0011,
                     qPrintable(QStringLiteral("Frame %1 has unexpected PTS %2").arg(index).arg(pts)));
            bool validDts = false;
            const double dts = frames.at(index).toObject()
                .value(QStringLiteral("pkt_dts_time")).toString().toDouble(&validDts);
            QVERIFY(validDts);
            QVERIFY(dts > previousDts);
            previousDts = dts;
        }
        // 2. 【录制】【时间轴测试】只容忍毫秒级容器取整，末帧也必须有完整的播放时长
        const double duration = json.value(QStringLiteral("format")).toObject()
            .value(QStringLiteral("duration")).toString().toDouble();
        const double expectedDuration = static_cast<double>(expectedIndices.last() + 1) / outputFps;
        QVERIFY2(std::abs(duration - expectedDuration) <= 0.002,
                 qPrintable(QStringLiteral("Unexpected duration %1").arg(duration)));
    }
};

QTEST_GUILESS_MAIN(RecordingVideoTimelineTest)
#include "recording_video_timeline_test.moc"
