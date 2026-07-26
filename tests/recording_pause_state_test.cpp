#include "recording/recording_pause_state.h"

#include <QtTest/QtTest>

class RecordingPauseStateTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证首帧确立时间轴起点，后续帧按相对时间输出。
     * @return 无返回值。
     */
    void timelineStartsAtFirstFrame()
    {
        markshot::recording::RecordingPauseState state;
        QCOMPARE(state.timelineFor(5000), 0);
        QCOMPARE(state.timelineFor(5100), 100);
        QCOMPARE(state.timelineFor(6000), 1000);
    }

    /**
     * 验证暂停时长从输出时间轴中扣除。
     * @return 无返回值。
     */
    void pausedDurationIsRemovedFromTimeline()
    {
        markshot::recording::RecordingPauseState state;
        QCOMPARE(state.timelineFor(1000), 0);
        QCOMPARE(state.timelineFor(1200), 200);

        QVERIFY(state.pause());
        QVERIFY(state.isPaused());
        QVERIFY(!state.pause());
        QTest::qWait(120);
        QVERIFY(state.resume());
        QVERIFY(!state.isPaused());
        QVERIFY(!state.resume());

        const qint64 pausedMs = state.pausedTotalMs();
        QVERIFY(pausedMs >= 100);

        // 暂停期间采集时间戳继续前进，输出时间轴应扣除这段时长
        const qint64 timeline = state.timelineFor(1200 + pausedMs + 300);
        QCOMPARE(timeline, 500);
    }

    /**
     * 验证重置后时间轴与暂停累计都归零。
     * @return 无返回值。
     */
    void resetClearsTimeline()
    {
        markshot::recording::RecordingPauseState state;
        state.timelineFor(2000);
        QVERIFY(state.pause());
        state.reset();

        QVERIFY(!state.isPaused());
        QCOMPARE(state.pausedTotalMs(), 0);
        QCOMPARE(state.timelineFor(9000), 0);
    }

    /**
     * 验证暂停进行中的累计时长包含当前这段。
     * @return 无返回值。
     */
    void pausedTotalIncludesOngoingPause()
    {
        markshot::recording::RecordingPauseState state;
        state.timelineFor(0);
        QVERIFY(state.pause());
        QTest::qWait(80);
        QVERIFY(state.pausedTotalMs() >= 60);
    }
};

QTEST_MAIN(RecordingPauseStateTest)

#include "recording_pause_state_test.moc"
