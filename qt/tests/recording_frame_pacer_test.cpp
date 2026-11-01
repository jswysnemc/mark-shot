#include "recording/recording_frame_pacer.h"

#include <QtTest/QtTest>

using markshot::recording::RecordingFramePacer;

class RecordingFramePacerTest final : public QObject {
    Q_OBJECT

private slots:
    /** 覆盖毫秒截断与常用采集、输出频率，无参数和返回值。 */
    void keepsThirtyMinuteTimeline_data()
    {
        QTest::addColumn<int>("inputFps");
        QTest::addColumn<int>("outputFps");
        for (int output : {12, 30, 60}) {
            for (int input : {12, 30, 60, 144}) {
                const QByteArray name = QByteArray::number(input) + "-to-" + QByteArray::number(output);
                QTest::newRow(name.constData()) << input << output;
            }
        }
    }

    /** 用三十分钟虚拟时间验证降频、补帧及无累计漂移，无参数和返回值。 */
    void keepsThirtyMinuteTimeline()
    {
        QFETCH(int, inputFps);
        QFETCH(int, outputFps);
        RecordingFramePacer pacer(outputFps);
        qint64 written = 0;
        for (int index = 0; index <= inputFps * 1800; ++index) {
            const auto decision = pacer.plan(1234 + static_cast<qint64>(index) * 1000 / inputFps);
            if (decision.frameIndex < 0) {
                continue;
            }
            QCOMPARE(decision.frameIndex, written + decision.repeatCount);
            written += decision.repeatCount + 1;
        }
        QCOMPARE(written, static_cast<qint64>(outputFps) * 1800 + 1);
    }

    /** 同时间格、迟到帧及起点前时间戳不能推进编码时间，无参数和返回值。 */
    void dropsEarlyAndStaleSamples()
    {
        RecordingFramePacer pacer(30);
        QCOMPARE(pacer.plan(100).frameIndex, 0);
        QCOMPARE(pacer.plan(100).frameIndex, -1);
        QCOMPARE(pacer.plan(116).frameIndex, -1);
        QCOMPARE(pacer.plan(132).frameIndex, -1);
        QCOMPARE(pacer.plan(133).frameIndex, 1);
        QCOMPARE(pacer.plan(99).frameIndex, -1);
        QCOMPARE(pacer.plan(120).frameIndex, -1);
        QCOMPARE(pacer.plan(166).frameIndex, 2);
    }

    /** 静止心跳补齐时间，超十秒断流限制补帧量但不丢失真实序号，无参数和返回值。 */
    void preservesHeartbeatAndLongGap()
    {
        RecordingFramePacer pacer(30);
        QCOMPARE(pacer.plan(0).frameIndex, 0);
        const auto heartbeat = pacer.plan(400);
        QCOMPARE(heartbeat.frameIndex, 12);
        QCOMPARE(heartbeat.repeatCount, 11);
        const auto gap = pacer.plan(20400);
        QCOMPARE(gap.frameIndex, 612);
        QCOMPARE(gap.repeatCount, 300);
        const auto next = pacer.plan(20433);
        QCOMPARE(next.frameIndex, 613);
        QCOMPARE(next.repeatCount, 0);
    }

    /** 暂停后继续使用扣除暂停时间的时间轴，重启才重置起点，无参数和返回值。 */
    void resetsOnlyForNewSession()
    {
        RecordingFramePacer pacer(30);
        QCOMPARE(pacer.plan(0).frameIndex, 0);
        QCOMPARE(pacer.plan(33).frameIndex, 1);
        QCOMPARE(pacer.plan(66).frameIndex, 2);
        pacer.reset(12);
        QCOMPARE(pacer.plan(5000).frameIndex, 0);
        QCOMPARE(pacer.plan(5083).frameIndex, 1);
    }

    /** 等待期限使用固定时间格，背压或停顿后不追赶采集请求，无参数和返回值。 */
    void schedulesCaptureWithoutDriftOrBursts()
    {
        RecordingFramePacer pacer(30);
        QCOMPARE(pacer.delayBeforeNext(1234), 0);
        pacer.plan(1234);
        QCOMPARE(pacer.delayBeforeNext(1234), 33);
        QCOMPARE(pacer.delayBeforeNext(1266), 1);
        QCOMPARE(pacer.delayBeforeNext(1267), 0);
        pacer.plan(1267);
        QCOMPARE(pacer.delayBeforeNext(1267), 33);
        pacer.plan(1300);
        QCOMPARE(pacer.delayBeforeNext(1300), 34);
        QCOMPARE(pacer.delayBeforeNext(5000), 0);
        pacer.plan(5000);
        QVERIFY(pacer.delayBeforeNext(5000) > 0);
        QVERIFY(pacer.delayBeforeNext(5000) <= 34);
    }
};

QTEST_APPLESS_MAIN(RecordingFramePacerTest)
#include "recording_frame_pacer_test.moc"
