#include "recording/recording_frame_heartbeat.h"

#include <QtTest/QtTest>

#include <QSignalSpy>

class RecordingFrameHeartbeatTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证画面静止超过阈值后请求补帧，且时间戳按墙钟推进。
     * @return 无返回值。
     */
    void requestsRepeatFrameWhenIdle()
    {
        markshot::recording::RecordingFrameHeartbeat heartbeat;
        QSignalSpy spy(&heartbeat, &markshot::recording::RecordingFrameHeartbeat::repeatFrameNeeded);

        heartbeat.start(30);
        heartbeat.noteFrameWritten(1000);
        QVERIFY(spy.wait(1500));

        const qint64 timestamp = spy.takeFirst().at(0).toLongLong();
        // 时间戳应当在上一帧之后，并与实际静止时长相当
        QVERIFY(timestamp > 1000);
        QVERIFY(timestamp < 1000 + 1500);
    }

    /**
     * 验证收到新帧后不再立即补帧。
     * @return 无返回值。
     */
    void staysQuietWhileFramesArrive()
    {
        markshot::recording::RecordingFrameHeartbeat heartbeat;
        QSignalSpy spy(&heartbeat, &markshot::recording::RecordingFrameHeartbeat::repeatFrameNeeded);

        heartbeat.start(60);
        // 持续以高于阈值的频率喂帧
        for (int i = 0; i < 8; ++i) {
            heartbeat.noteFrameWritten(i * 30);
            QTest::qWait(30);
        }
        QCOMPARE(spy.count(), 0);
    }

    /**
     * 验证暂停期间不请求补帧。
     * @return 无返回值。
     */
    void staysQuietWhilePaused()
    {
        markshot::recording::RecordingFrameHeartbeat heartbeat;
        QSignalSpy spy(&heartbeat, &markshot::recording::RecordingFrameHeartbeat::repeatFrameNeeded);

        heartbeat.start(30);
        heartbeat.noteFrameWritten(0);
        heartbeat.setPaused(true);
        QTest::qWait(700);
        QCOMPARE(spy.count(), 0);

        // 恢复后重新计时，随后应当继续补帧
        heartbeat.setPaused(false);
        QVERIFY(spy.wait(1500));
    }

    /**
     * 验证停止后不再请求补帧。
     * @return 无返回值。
     */
    void staysQuietAfterStop()
    {
        markshot::recording::RecordingFrameHeartbeat heartbeat;
        QSignalSpy spy(&heartbeat, &markshot::recording::RecordingFrameHeartbeat::repeatFrameNeeded);

        heartbeat.start(30);
        heartbeat.noteFrameWritten(0);
        heartbeat.stop();
        QTest::qWait(700);
        QCOMPARE(spy.count(), 0);
    }
};

QTEST_MAIN(RecordingFrameHeartbeatTest)

#include "recording_frame_heartbeat_test.moc"
