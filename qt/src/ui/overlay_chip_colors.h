#pragma once

#include "ui/toolbar_palette.h"

#include <QColor>

namespace markshot::ui {

/**
 * 【截图覆盖层】【提示标签】压在截图上的提示标签颜色，例如选区尺寸、放大镜取色值、缩放比例与录制状态。
 */
struct OverlayChipColors {
    // 标签底板与细边框，无边框时为透明色
    QColor fill;
    QColor border;
    // 主要文字与次要文字
    QColor text;
    QColor mutedText;
    // 强调文字，例如录制状态或快捷键说明
    QColor accentText;
};

/**
 * 按截图会话的工具栏配色选择提示标签颜色。暗色主题保留调用方原有的配色，亮色主题统一取工具栏的白色面板与石板灰文字。
 * @param palette 截图会话的工具栏配色。
 * @param dark 调用方原有的暗色配色。
 * @return 暗色主题返回 dark，亮色主题返回由工具栏配色生成的颜色。
 */
inline OverlayChipColors overlayChipColors(const ToolbarPalette &palette, const OverlayChipColors &dark)
{
    if (palette.dark) {
        return dark;
    }
    OverlayChipColors light;
    light.fill = palette.panelFill;
    // 1. 原本无边框的标签在亮色下补一条细边，避免白底压在浅色截图上失去边界
    light.border = palette.panelBorder;
    light.text = palette.strongText;
    light.mutedText = palette.mutedText;
    light.accentText = palette.pressedFill;
    return light;
}

}  // namespace markshot::ui
