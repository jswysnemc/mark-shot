#include "recording/recording_frame_rate_limiter.h"

#include <QtTest/QtTest>

class RecordingFrameRateLimiterTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证首帧总是写出。
     * @return 无返回值。
     */
    void firstFrameAlwaysPasses()
    {
        markshot::recording::RecordingFrameRateLimiter limiter(10);
        QVERIFY(limiter.shouldWrite(1234));
    }

    /**
     * 验证高于目标帧率的帧被丢弃。
     * @return 无返回值。
     */
    void dropsFramesFasterThanTarget()
    {
        markshot::recording::RecordingFrameRateLimiter limiter(10);
        QVERIFY(limiter.shouldWrite(0));
        // 目标间隔 100ms，容差后约 85ms
        QVERIFY(!limiter.shouldWrite(20));
        QVERIFY(!limiter.shouldWrite(60));
        QVERIFY(limiter.shouldWrite(100));
        QVERIFY(!limiter.shouldWrite(150));
        QVERIFY(limiter.shouldWrite(200));
    }

    /**
     * 验证 60fps 采集降到 12fps 目标后的写出数量接近预期。
     * @return 无返回值。
     */
    void keepsTargetRateUnderFastCapture()
    {
        markshot::recording::RecordingFrameRateLimiter limiter(12);
        int written = 0;
        // 模拟 60fps 采集持续 2 秒
        for (int i = 0; i < 120; ++i) {
            if (limiter.shouldWrite(i * 1000 / 60)) {
                ++written;
            }
        }
        QVERIFY(written >= 20);
        QVERIFY(written <= 30);
    }

    /**
     * 验证时间戳回退后重新计时。
     * @return 无返回值。
     */
    void restartsAfterTimestampRewind()
    {
        markshot::recording::RecordingFrameRateLimiter limiter(10);
        QVERIFY(limiter.shouldWrite(500));
        QVERIFY(limiter.shouldWrite(100));
        QVERIFY(!limiter.shouldWrite(120));
    }
};

QTEST_APPLESS_MAIN(RecordingFrameRateLimiterTest)

#include "recording_frame_rate_limiter_test.moc"
