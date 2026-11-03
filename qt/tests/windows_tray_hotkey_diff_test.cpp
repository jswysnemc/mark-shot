#include "windows_tray_hotkey_diff.h"

#include <QtTest>

using markshot::WindowsTrayController;
using markshot::trayHotkeysDiffer;

class WindowsTrayHotkeyDiffTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【托盘测试】【快捷键比较】相同配置不触发重新注册，无参数和返回值。 */
    void identicalConfigsDoNotDiffer()
    {
        WindowsTrayController::Config config;
        config.fullscreenHotkey = QKeySequence(QStringLiteral("Ctrl+Alt+F"));
        QVERIFY(!trayHotkeysDiffer(config, config));
    }

    /** 【托盘测试】【快捷键比较】任一快捷键组合变化都需要重新注册，无参数和返回值。 */
    void changedSequenceDiffers()
    {
        const WindowsTrayController::Config current;
        WindowsTrayController::Config next = current;
        next.captureHotkey = QKeySequence(QStringLiteral("Ctrl+Shift+A"));
        QVERIFY(trayHotkeysDiffer(current, next));

        next = current;
        next.fullscreenHotkey = QKeySequence(QStringLiteral("Ctrl+Alt+F"));
        QVERIFY(trayHotkeysDiffer(current, next));

        next = current;
        next.stopRecordingHotkey = QKeySequence(QStringLiteral("Ctrl+Alt+X"));
        QVERIFY(trayHotkeysDiffer(current, next));

        next = current;
        next.pauseRecordingHotkey = QKeySequence(QStringLiteral("Ctrl+Alt+P"));
        QVERIFY(trayHotkeysDiffer(current, next));
    }

    /** 【托盘测试】【快捷键比较】开关全局快捷键需要注销或注册，无参数和返回值。 */
    void toggledEnabledDiffers()
    {
        const WindowsTrayController::Config current;
        WindowsTrayController::Config next = current;
        next.hotkeysEnabled = !current.hotkeysEnabled;
        QVERIFY(trayHotkeysDiffer(current, next));
    }

    /** 【托盘测试】【快捷键比较】仅自动启动变化时保持现有注册，无参数和返回值。 */
    void autoStartAloneDoesNotDiffer()
    {
        const WindowsTrayController::Config current;
        WindowsTrayController::Config next = current;
        next.autoStart = !current.autoStart;
        QVERIFY(!trayHotkeysDiffer(current, next));
    }
};

QTEST_APPLESS_MAIN(WindowsTrayHotkeyDiffTest)
#include "windows_tray_hotkey_diff_test.moc"
