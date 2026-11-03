#include "selection_aspect/selection_hud_colors.h"

#include "ui/theme.h"

namespace markshot::selection_aspect::hud_style {

HudColors hudColors(const ui::ToolbarPalette &palette)
{
    HudColors colors;
    if (palette.dark) {
        // 1. 暗色沿用原有 HUD 配色，避免既有界面出现像素差异
        colors.text = QColor(229, 231, 235);
        colors.muted = QColor(156, 163, 175);
        colors.hover = QColor(255, 255, 255, 26);
        colors.panelFill = QColor(8, 13, 19, 228);
        colors.panelBorder = QColor(255, 255, 255, 30);
        colors.separator = QColor(255, 255, 255, 36);
        colors.accent = markshot::theme::kAccent;
        colors.editorText = QColor(249, 250, 251);
        colors.editorFill = QColor(255, 255, 255, 20);
        colors.editorBorder = QColor(94, 234, 212, 170);
        colors.editorSelection = QColor(94, 234, 212, 110);
        return colors;
    }

    // 2. 亮色取工具栏的白色面板与深青色强调，压在截图上保持可读
    colors.text = palette.text;
    colors.muted = palette.mutedText;
    colors.hover = palette.controlHoverFill;
    colors.panelFill = palette.panelFill;
    colors.panelBorder = palette.panelBorder;
    colors.separator = palette.separator;
    colors.accent = palette.hoverFill;
    colors.editorText = palette.strongText;
    colors.editorFill = palette.controlFill;
    colors.editorBorder = palette.focusBorder;
    colors.editorSelection = palette.selectionFill;
    return colors;
}

QString hudEditorStyleSheet(const HudColors &colors)
{
    return QStringLiteral(
               "QLineEdit {"
               " color: %1;"
               " background: %2;"
               " border: 1px solid %3;"
               " border-radius: 5px;"
               " padding: 0px 2px;"
               " selection-background-color: %4;"
               "}")
        .arg(ui::cssColor(colors.editorText), ui::cssColor(colors.editorFill), ui::cssColor(colors.editorBorder),
             ui::cssColor(colors.editorSelection));
}

}  // namespace markshot::selection_aspect::hud_style
