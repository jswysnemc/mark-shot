#pragma once

#include "ui/toolbar_palette.h"

#include <QString>

namespace markshot::theme {

/// @brief 生成截图主工具栏、动作工具栏、属性面板与形状弹层共用的样式表（暗色配色、默认尺寸）
/// @return 按 objectName 作用域隔离的样式表
QString panelStyleSheet();

/// @brief 按配色与尺寸生成工具面板样式表
/// @param palette 工具栏配色令牌
/// @param buttonSize 工具栏按钮边长
/// @param fontSize 工具栏控件字体大小
/// @return 按 objectName 作用域隔离的样式表
QString panelStyleSheet(const markshot::ui::ToolbarPalette &palette, int buttonSize, int fontSize);

/// @brief 生成“打开方式”、扩展与字体下拉面板的样式表
/// @param palette 工具栏配色令牌，默认暗色
/// @return 下拉面板样式表
QString openWithPanelStyleSheet(const markshot::ui::ToolbarPalette &palette = {});

/// @brief 生成属性面板旁取色面板的外框样式表，内部控件由取色器自绘
/// @param palette 工具栏配色令牌
/// @return 取色面板外框样式表
QString propertyColorDialogPanelStyleSheet(const markshot::ui::ToolbarPalette &palette);

}  // namespace markshot::theme
