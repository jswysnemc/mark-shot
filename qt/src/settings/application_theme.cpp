#include "settings/application_theme.h"

#include "app_config_store.h"
#include "settings/settings_design_tokens.h"
#include "ui/theme.h"

#include <QApplication>
#include <QGuiApplication>
#include <QJsonObject>
#include <QStyleHints>

namespace markshot::settings {

QPalette applyApplicationTheme(markshot::ui::UiThemeMode mode)
{
    // 1. 设置页与其他普通窗口共用同一套调色板，工具提示随调色板同步
    const markshot::ui::UiThemeMode effectiveMode = markshot::ui::effectiveUiThemeMode(mode);
    const QPalette palette = tokens::settingsPalette(effectiveMode);
    qApp->setPalette(palette);
    markshot::theme::synchronizeToolTipPalette(palette);
    return palette;
}

void applyConfiguredApplicationTheme()
{
    // 1. 配置缺失或读取失败时按跟随系统处理
    bool ok = false;
    const QJsonObject root = markshot::readAppConfigRoot(&ok);
    markshot::ui::UiThemeMode mode = ok ? markshot::ui::uiThemeModeFromConfigRoot(root) : markshot::ui::UiThemeMode::System;
    // 2. 跟随系统且系统未报告明暗方案时，按平台主题已加载的调色板判断，避免样式默认调色板误判为亮色
    if (mode == markshot::ui::UiThemeMode::System) {
        bool schemeKnown = false;
#if QT_VERSION >= QT_VERSION_CHECK(6, 5, 0)
        const QStyleHints *hints = QGuiApplication::styleHints();
        schemeKnown = hints && hints->colorScheme() != Qt::ColorScheme::Unknown;
#endif
        if (!schemeKnown) {
            mode = markshot::ui::effectiveUiThemeMode(mode, qApp->palette());
        }
    }
    applyApplicationTheme(mode);
}

}  // namespace markshot::settings
