#pragma once

#include "ui/toolbar_palette.h"

#include <QColor>

namespace markshot::selection_aspect::hud_style {

/**
 * 【选区尺寸】【配色】尺寸 HUD、比例弹层与行内输入框共用的颜色。
 */
struct HudColors {
    // 主要文字、单位与分隔符、悬停底色
    QColor text;
    QColor muted;
    QColor hover;
    // 面板底板、细边框与竖向分隔线
    QColor panelFill;
    QColor panelBorder;
    QColor separator;
    // 锁定比例、当前比例与选中标记使用的强调色
    QColor accent;
    // 行内输入框的文字、底色、描边与选中底色
    QColor editorText;
    QColor editorFill;
    QColor editorBorder;
    QColor editorSelection;
};

/**
 * 按截图会话的工具栏配色生成 HUD 颜色；暗色保持原有 HUD 配色，亮色沿用工具栏的白色面板。
 * @param palette 截图会话的工具栏配色，默认值为暗色。
 * @return HUD 颜色。
 */
HudColors hudColors(const ui::ToolbarPalette &palette = {});

/**
 * 生成行内输入框的样式表。
 * @param colors HUD 颜色。
 * @return QLineEdit 样式表文本。
 */
QString hudEditorStyleSheet(const HudColors &colors);

}  // namespace markshot::selection_aspect::hud_style
