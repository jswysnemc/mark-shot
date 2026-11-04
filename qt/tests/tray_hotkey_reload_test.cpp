#include "settings/settings_saved_notifier.h"
#include "windows_tray_controller.h"

#include <QApplication>
#include <QFile>
#include <QGuiApplication>
#include <QProcess>
#include <QTemporaryDir>
#include <QtTest>

#include <memory>

using markshot::WindowsTrayController;

class TrayHotkeyReloadTest final : public QObject {
    Q_OBJECT

private:
    QTemporaryDir m_configDir;
    std::unique_ptr<WindowsTrayController> m_tray;
    int m_captureCount = 0;

    /**
     * 【托盘测试】【配置】写入只包含全局快捷键的应用配置。
     * @param enabled 是否启用全局快捷键。
     * @param capture 截图快捷键文本。
     * @return 写入成功返回 true。
     */
    bool writeHotkeyConfig(bool enabled, const QString &capture) const
    {
        QFile file(m_configDir.filePath(QStringLiteral("config.json")));
        if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
            return false;
        }
        // 1. 与设置页写入方式一致，同时写入 windows.hotkeys 与 globalHotkeys
        const QString block = QStringLiteral("{\"enabled\":%1,\"capture\":\"%2\"}")
                                  .arg(enabled ? QStringLiteral("true") : QStringLiteral("false"), capture);
        const QString json = QStringLiteral("{\"windows\":{\"hotkeys\":%1},\"globalHotkeys\":%1}").arg(block);
        return file.write(json.toUtf8()) > 0;
    }

    /**
     * 【托盘测试】【模拟按键】通过 XTest 向 X 服务器发送组合键，经过与真实键盘相同的被动抓键路径。
     * @param combo xdotool 组合键文本，例如 ctrl+alt+shift+F9。
     * @return xdotool 正常退出返回 true。
     */
    static bool pressKeys(const QString &combo)
    {
        QProcess process;
        process.start(qEnvironmentVariable("MARK_SHOT_TEST_XDOTOOL", QStringLiteral("xdotool")),
                      {QStringLiteral("key"), combo});
        return process.waitForFinished(5000) && process.exitStatus() == QProcess::NormalExit
            && process.exitCode() == 0;
    }

    /**
     * 【托盘测试】【未触发】按下组合键后等待一段时间，确认截图回调没有被调用。
     * @param combo xdotool 组合键文本。
     * @return 回调次数保持不变返回 true。
     */
    bool pressDoesNotCapture(const QString &combo)
    {
        const int before = m_captureCount;
        if (!pressKeys(combo)) {
            return false;
        }
        QTest::qWait(500);
        return m_captureCount == before;
    }

private slots:
    /** 【托盘测试】【隔离环境】使用临时配置目录并确认运行在 X11 平台，无参数和返回值。 */
    void initTestCase()
    {
        if (QGuiApplication::platformName() != QStringLiteral("xcb")) {
            QSKIP("requires the xcb platform inside Xvfb");
        }
        QVERIFY(m_configDir.isValid());
        qputenv("MARK_SHOT_CONFIG_DIR", m_configDir.path().toUtf8());
    }

    /** 【托盘测试】【启动注册】启动时注册配置中的截图快捷键，无参数和返回值。 */
    void registersConfiguredHotkeyAtStartup()
    {
        QVERIFY(writeHotkeyConfig(true, QStringLiteral("Ctrl+Alt+Shift+F9")));
        const WindowsTrayController::Config config = WindowsTrayController::readConfig();
        QCOMPARE(config.captureHotkey, QKeySequence(QStringLiteral("Ctrl+Alt+Shift+F9")));

        m_tray = std::make_unique<WindowsTrayController>(qobject_cast<QApplication *>(qApp), config);
        m_tray->setCaptureCallbacks([this] { ++m_captureCount; }, [] {});
        QVERIFY2(m_tray->start(), qPrintable(m_tray->errorString()));

        QVERIFY(pressKeys(QStringLiteral("ctrl+alt+shift+F9")));
        QTRY_COMPARE_WITH_TIMEOUT(m_captureCount, 1, 3000);
    }

    /** 【托盘测试】【保存生效】保存设置后旧组合失效、新组合立即生效，无需重启，无参数和返回值。 */
    void savedHotkeyTakesEffectWithoutRestart()
    {
        QVERIFY(m_tray);
        QVERIFY(writeHotkeyConfig(true, QStringLiteral("Ctrl+Alt+Shift+F10")));
        markshot::settings::SettingsSavedNotifier::instance().notifySaved();

        QVERIFY(pressDoesNotCapture(QStringLiteral("ctrl+alt+shift+F9")));
        const int before = m_captureCount;
        QVERIFY(pressKeys(QStringLiteral("ctrl+alt+shift+F10")));
        QTRY_COMPARE_WITH_TIMEOUT(m_captureCount, before + 1, 3000);
    }

    /** 【托盘测试】【关闭快捷键】保存时关闭全局快捷键会立即注销，重新开启后恢复，无参数和返回值。 */
    void disablingAndEnablingAppliesImmediately()
    {
        QVERIFY(m_tray);
        QVERIFY(writeHotkeyConfig(false, QStringLiteral("Ctrl+Alt+Shift+F10")));
        markshot::settings::SettingsSavedNotifier::instance().notifySaved();
        QVERIFY(pressDoesNotCapture(QStringLiteral("ctrl+alt+shift+F10")));

        QVERIFY(writeHotkeyConfig(true, QStringLiteral("Ctrl+Alt+Shift+F10")));
        markshot::settings::SettingsSavedNotifier::instance().notifySaved();
        const int before = m_captureCount;
        QVERIFY(pressKeys(QStringLiteral("ctrl+alt+shift+F10")));
        QTRY_COMPARE_WITH_TIMEOUT(m_captureCount, before + 1, 3000);
    }

    /** 【托盘测试】【释放】销毁托盘并释放抓键，无参数和返回值。 */
    void cleanupTestCase()
    {
        m_tray.reset();
    }
};

QTEST_MAIN(TrayHotkeyReloadTest)
#include "tray_hotkey_reload_test.moc"
