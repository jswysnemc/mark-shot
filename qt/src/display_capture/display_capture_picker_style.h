#pragma once

#include "ui/toolbar_palette.h"

#include <QString>

namespace markshot::display_capture {

/**
 * 【显示器截取】【主题】生成显示器选择器、卡片、悬停操作栏与按钮的样式表。
 * @param palette 截图会话的工具栏配色，面板与操作栏跟随同一明暗主题。
 * @return 选择器样式表文本。
 */
QString displayCapturePickerStyleSheet(const ui::ToolbarPalette &palette);

}  // namespace markshot::display_capture
