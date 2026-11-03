#include "ui/configured_toolbar_palette.h"

#include "app_config_store.h"
#include "ui/interface_theme_config.h"

namespace markshot::ui {

ToolbarPalette configuredToolbarPalette()
{
    // 1. 配置缺失或读取失败时按跟随系统处理
    bool ok = false;
    const QJsonObject root = markshot::readAppConfigRoot(&ok);
    const UiThemeMode mode = ok ? uiThemeModeFromConfigRoot(root) : UiThemeMode::System;
    return toolbarPalette(effectiveUiThemeMode(mode));
}

}  // namespace markshot::ui
