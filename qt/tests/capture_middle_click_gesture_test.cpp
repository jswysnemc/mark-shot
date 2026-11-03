#include "capture_middle_click_gesture.h"

#include <QtTest/QtTest>

namespace {

// 测试使用的拖动阈值，与常见平台默认值一致
constexpr int kThreshold = 10;
const QPointF kPress(100.0, 100.0);

}  // namespace

class CaptureMiddleClickGestureTest : public QObject {
    Q_OBJECT

private slots:
    /** 验证默认动作保留中键内置行为，可改为配置动作，无参数和返回值。 */
    void actionDefaultsToBuiltinBehaviour()
    {
        markshot::CaptureMiddleClickGesture gesture;
        QCOMPARE(gesture.action(), markshot::CaptureClickAction::None);
        gesture.setAction(markshot::CaptureClickAction::Pin);
        QCOMPARE(gesture.action(), markshot::CaptureClickAction::Pin);
    }

    /** 验证原地或阈值内抖动的按下释放构成单击，无参数和返回值。 */
    void releaseWithinThresholdIsClick()
    {
        markshot::CaptureMiddleClickGesture gesture;
        gesture.press(kPress, kThreshold);
        QVERIFY(gesture.pending());
        QVERIFY(gesture.track(kPress + QPointF(4, 3)));
        QVERIFY(gesture.release(kPress + QPointF(-5, 4)));
        QVERIFY(!gesture.pending());
    }

    /** 验证超过阈值后转为拖动，指针移回原处也不再算作单击，无参数和返回值。 */
    void dragBeyondThresholdIsNotClick()
    {
        markshot::CaptureMiddleClickGesture gesture;
        gesture.press(kPress, kThreshold);
        QVERIFY(!gesture.track(kPress + QPointF(6, 4)));
        QVERIFY(!gesture.pending());
        QVERIFY(!gesture.track(kPress));
        QVERIFY(!gesture.release(kPress));
    }

    /** 验证没有移动事件时释放位置同样按阈值判定，无参数和返回值。 */
    void releaseFarAwayWithoutMoveIsNotClick()
    {
        markshot::CaptureMiddleClickGesture gesture;
        gesture.press(kPress, kThreshold);
        QVERIFY(!gesture.release(kPress + QPointF(0, kThreshold)));
    }

    /** 验证取消或未按下时释放不构成单击，下一次按下重新登记，无参数和返回值。 */
    void cancelAndIdleReleaseAreNotClicks()
    {
        markshot::CaptureMiddleClickGesture gesture;
        QVERIFY(!gesture.release(kPress));
        gesture.press(kPress, kThreshold);
        gesture.cancel();
        QVERIFY(!gesture.pending());
        QVERIFY(!gesture.release(kPress));
        gesture.press(kPress, kThreshold);
        QVERIFY(gesture.release(kPress));
    }

    /** 验证阈值为 0 时原地单击仍然有效，无参数和返回值。 */
    void zeroThresholdStillAllowsStationaryClick()
    {
        markshot::CaptureMiddleClickGesture gesture;
        gesture.press(kPress, 0);
        QVERIFY(gesture.release(kPress));
        gesture.press(kPress, 0);
        QVERIFY(!gesture.release(kPress + QPointF(1, 0)));
    }
};

QTEST_MAIN(CaptureMiddleClickGestureTest)
#include "capture_middle_click_gesture_test.moc"
