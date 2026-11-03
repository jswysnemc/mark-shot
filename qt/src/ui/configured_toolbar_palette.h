#pragma once

#include "ui/toolbar_palette.h"

namespace markshot::ui {

/**
 * 【界面主题】【工具栏配色】按应用配置中的界面主题读取工具栏配色，供截图窗口之外的浮动面板使用，例如录制控制条与倒计时。
 * 每次调用都会重新读取配置，设置页切换主题后新建的面板立即生效。
 * @return 跟随系统时按当前系统明暗解析的工具栏配色。
 */
ToolbarPalette configuredToolbarPalette();

}  // namespace markshot::ui
