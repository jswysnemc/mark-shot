#include "capture_session_screen_utils.h"

#include <QtTest/QtTest>

class CaptureSessionScreenUtilsTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证 Wayland 多屏始终使用逐屏捕获，避免混合缩放帧共享。
     * @return 无返回值。
     */
    void waylandMultiScreenUsesIndividualFrames()
    {
        QVERIFY(markshot::capture_session::shouldCaptureScreensIndividually(true, 2));
        QVERIFY(markshot::capture_session::shouldCaptureScreensIndividually(true, 3));
    }

    /**
     * 验证单屏和非 Wayland 场景不启用逐屏捕获。
     * @return 无返回值。
     */
    void otherLayoutsKeepSingleFramePath()
    {
        QVERIFY(!markshot::capture_session::shouldCaptureScreensIndividually(true, 1));
        QVERIFY(!markshot::capture_session::shouldCaptureScreensIndividually(false, 2));
    }
};

QTEST_APPLESS_MAIN(CaptureSessionScreenUtilsTest)

#include "capture_session_screen_utils_test.moc"
