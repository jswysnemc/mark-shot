#include "settings/application_theme.h"

#include "app_config_store.h"
#include "settings/settings_design_tokens.h"
#include "ui/theme.h"

#include <QApplication>
#include <QJsonObject>

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
    applyApplicationTheme(ok ? markshot::ui::uiThemeModeFromConfigRoot(root) : markshot::ui::UiThemeMode::System);
}

}  // namespace markshot::settings
