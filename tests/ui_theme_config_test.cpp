#include "ui/interface_theme_config.h"

#include <QtTest/QtTest>

/**
 * 界面主题配置测试。
 */
class UiThemeConfigTest : public QObject
{
    Q_OBJECT

private slots:
    /**
     * 验证常见配置名称可以解析为主题模式。
     * @return 无返回值。
     */
    void parsesThemeModeNames()
    {
        QCOMPARE(markshot::ui::uiThemeModeFromString(QStringLiteral("system")),
                 markshot::ui::UiThemeMode::System);
        QCOMPARE(markshot::ui::uiThemeModeFromString(QStringLiteral("dark")),
                 markshot::ui::UiThemeMode::Dark);
        QCOMPARE(markshot::ui::uiThemeModeFromString(QStringLiteral("light")),
                 markshot::ui::UiThemeMode::Light);
    }

    /**
     * 验证主题模式可以写回稳定配置名称。
     * @return 无返回值。
     */
    void writesStableConfigNames()
    {
        QCOMPARE(markshot::ui::uiThemeModeName(markshot::ui::UiThemeMode::System),
                 QStringLiteral("system"));
        QCOMPARE(markshot::ui::uiThemeModeName(markshot::ui::UiThemeMode::Dark),
                 QStringLiteral("dark"));
        QCOMPARE(markshot::ui::uiThemeModeName(markshot::ui::UiThemeMode::Light),
                 QStringLiteral("light"));
    }

    /**
     * 验证可以从应用配置根对象读取界面主题模式。
     * @return 无返回值。
     */
    void readsConfigRoot()
    {
        QJsonObject ui;
        ui.insert(QStringLiteral("theme"), QStringLiteral("light"));

        QJsonObject root;
        root.insert(QStringLiteral("ui"), ui);

        QCOMPARE(markshot::ui::uiThemeModeFromConfigRoot(root),
                 markshot::ui::UiThemeMode::Light);
        QCOMPARE(markshot::ui::uiThemeModeFromConfigRoot(QJsonObject()),
                 markshot::ui::UiThemeMode::System);
    }

    /**
     * 验证 system 模式会根据调色板明暗解析实际主题。
     * @return 无返回值。
     */
    void resolvesSystemThemeFromPalette()
    {
        QPalette darkPalette;
        darkPalette.setColor(QPalette::Window, QColor(15, 23, 42));
        darkPalette.setColor(QPalette::WindowText, QColor(241, 245, 249));
        QCOMPARE(markshot::ui::effectiveUiThemeMode(markshot::ui::UiThemeMode::System, darkPalette),
                 markshot::ui::UiThemeMode::Dark);

        QPalette lightPalette;
        lightPalette.setColor(QPalette::Window, QColor(248, 250, 252));
        lightPalette.setColor(QPalette::WindowText, QColor(15, 23, 42));
        QCOMPARE(markshot::ui::effectiveUiThemeMode(markshot::ui::UiThemeMode::System, lightPalette),
                 markshot::ui::UiThemeMode::Light);
    }
};

/**
 * 界面主题配置测试入口。
 * @param argc 参数数量。
 * @param argv 参数数组。
 * @return 进程退出码。
 */
QTEST_APPLESS_MAIN(UiThemeConfigTest)

#include "ui_theme_config_test.moc"
