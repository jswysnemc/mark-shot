#pragma once

#include "ui/toolbar_palette.h"

#include <QJsonObject>

namespace markshot {

struct ToolbarAppearanceConfig {
    int toolbarIconSize = 22;
    int actionToolbarIconSize = 20;
    int fontSize = 11;
    int toolbarButtonSize = 30;
    int actionToolbarButtonSize = 28;
    /// @brief 工具栏配色，截图窗口创建时按界面主题设置解析，默认暗色
    markshot::ui::ToolbarPalette palette;
};

/// @brief 获取截图工具栏外观默认配置。
/// @return 工具栏外观默认配置。
ToolbarAppearanceConfig defaultToolbarAppearanceConfig();

/// @brief 从应用配置根对象解析截图工具栏外观配置。
/// @param root 应用配置根对象。
/// @return 解析后的工具栏外观配置。
ToolbarAppearanceConfig toolbarAppearanceFromConfigRoot(const QJsonObject &root);

}  // namespace markshot
