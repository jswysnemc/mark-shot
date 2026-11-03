#pragma once

#include "ui/interface_theme_config.h"

#include <QPalette>

namespace markshot::settings {

/**
 * 【界面主题】【应用调色板】按界面主题设置全局调色板与工具提示样式，使消息框、文件对话框和工具提示跟随配置主题。
 * @param mode 配置中的界面主题模式，跟随系统时按当前系统明暗解析。
 * @return 已应用的全局调色板。
 */
QPalette applyApplicationTheme(markshot::ui::UiThemeMode mode);

/**
 * 【界面主题】【启动应用】读取应用配置中的界面主题并应用到全局调色板，供程序启动时调用。
 * @return 无返回值。
 */
void applyConfiguredApplicationTheme();

}  // namespace markshot::settings
