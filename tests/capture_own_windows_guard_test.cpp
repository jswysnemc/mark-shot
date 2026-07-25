#include "capture_own_windows_guard.h"

#include <QApplication>
#include <QWidget>
#include <QtTest/QtTest>

class CaptureOwnWindowsGuardTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证守卫隐藏可见顶层窗口并在销毁时恢复。
     * @return 无返回值。
     */
    void hidesAndRestoresVisibleTopLevelWindow()
    {
        QWidget window;
        window.setWindowTitle(QStringLiteral("capture-guard-test"));
        window.show();
        QApplication::processEvents();
        QVERIFY(window.isVisible());

        {
            markshot::CaptureOwnWindowsGuard guard;
            QVERIFY(!window.isVisible());
        }

        QVERIFY(window.isVisible());
        window.close();
    }

    /**
     * 验证关闭策略时不改变顶层窗口可见性。
     * @return 无返回值。
     */
    void disabledGuardLeavesWindowVisible()
    {
        QWidget window;
        window.show();
        QApplication::processEvents();
        QVERIFY(window.isVisible());

        markshot::CaptureOwnWindowsGuard guard(false);
        QVERIFY(window.isVisible());
        window.close();
    }
};

QTEST_MAIN(CaptureOwnWindowsGuardTest)

#include "capture_own_windows_guard_test.moc"
