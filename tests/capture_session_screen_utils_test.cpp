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

    /**
     * 验证普通区域截图只在多屏全冻结模式下冻结全部显示器。
     * @return 无返回值。
     */
    void regionSelectionFreezesAllConfiguredScreens()
    {
        using markshot::CaptureFreezeScope;
        using markshot::capture_session::shouldFreezeAllScreens;

        QVERIFY(shouldFreezeAllScreens(false, false, CaptureFreezeScope::AllScreens, 2));
        QVERIFY(!shouldFreezeAllScreens(true, false, CaptureFreezeScope::AllScreens, 2));
        QVERIFY(!shouldFreezeAllScreens(false, true, CaptureFreezeScope::AllScreens, 2));
        QVERIFY(!shouldFreezeAllScreens(false, false, CaptureFreezeScope::CursorScreen, 2));
        QVERIFY(!shouldFreezeAllScreens(false, false, CaptureFreezeScope::AllScreens, 1));
    }
};

QTEST_APPLESS_MAIN(CaptureSessionScreenUtilsTest)

#include "capture_session_screen_utils_test.moc"
