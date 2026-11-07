/** 【主题对照】【原版令牌】执行原版配色函数，返回明暗主题完整颜色；不改写参考项目 */
#include "ui/toolbar_palette.h"
#include "ui/context_menu_style.h"
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <iostream>
/** 【主题对照】【通道读取】把 color 转换为 RGBA 数组并返回，不参与颜色推算 */
static QJsonArray rgba(QColor color) { return {color.red(),color.green(),color.blue(),color.alpha()}; }
/** 【主题对照】【实际执行】调用原版明暗函数并输出 JSON；返回零表示成功 */
int main() {
    QJsonObject result;
    for (bool dark : {false, true}) {
        auto toolbar = markshot::ui::toolbarPalette(dark ? markshot::ui::UiThemeMode::Dark : markshot::ui::UiThemeMode::Light);
        auto menu = markshot::ui::contextMenuStyle(dark);
        QJsonObject toolbarColors, menuColors;
        toolbarColors["panel_fill"] = rgba(toolbar.panelFill);
        toolbarColors["panel_border"] = rgba(toolbar.panelBorder);
        toolbarColors["popup_fill"] = rgba(toolbar.popupFill);
        toolbarColors["popup_border"] = rgba(toolbar.popupBorder);
        toolbarColors["separator"] = rgba(toolbar.separator);
        toolbarColors["grip_dot"] = rgba(toolbar.gripDot);
        toolbarColors["grip_dot_hover"] = rgba(toolbar.gripDotHover);
        toolbarColors["grip_dot_disabled"] = rgba(toolbar.gripDotDisabled);
        toolbarColors["grip_hover_fill"] = rgba(toolbar.gripHoverFill);
        toolbarColors["text"] = rgba(toolbar.text);
        toolbarColors["strong_text"] = rgba(toolbar.strongText);
        toolbarColors["muted_text"] = rgba(toolbar.mutedText);
        toolbarColors["primary_text"] = rgba(toolbar.primaryText);
        toolbarColors["hover_fill"] = rgba(toolbar.hoverFill);
        toolbarColors["hover_border"] = rgba(toolbar.hoverBorder);
        toolbarColors["hover_text"] = rgba(toolbar.hoverText);
        toolbarColors["hover_icon"] = rgba(toolbar.hoverIcon);
        toolbarColors["pressed_fill"] = rgba(toolbar.pressedFill);
        toolbarColors["pressed_border"] = rgba(toolbar.pressedBorder);
        toolbarColors["active_fill"] = rgba(toolbar.activeFill);
        toolbarColors["active_border"] = rgba(toolbar.activeBorder);
        toolbarColors["primary_top"] = rgba(toolbar.primaryTop);
        toolbarColors["primary_bottom"] = rgba(toolbar.primaryBottom);
        toolbarColors["primary_border"] = rgba(toolbar.primaryBorder);
        toolbarColors["primary_hover_top"] = rgba(toolbar.primaryHoverTop);
        toolbarColors["primary_hover_bottom"] = rgba(toolbar.primaryHoverBottom);
        toolbarColors["primary_hover_border"] = rgba(toolbar.primaryHoverBorder);
        toolbarColors["accent_fill"] = rgba(toolbar.accentFill);
        toolbarColors["accent_border"] = rgba(toolbar.accentBorder);
        toolbarColors["accent_hover_fill"] = rgba(toolbar.accentHoverFill);
        toolbarColors["accent_hover_border"] = rgba(toolbar.accentHoverBorder);
        toolbarColors["danger_hover_text"] = rgba(toolbar.dangerHoverText);
        toolbarColors["danger_hover_fill"] = rgba(toolbar.dangerHoverFill);
        toolbarColors["danger_hover_border"] = rgba(toolbar.dangerHoverBorder);
        toolbarColors["control_fill"] = rgba(toolbar.controlFill);
        toolbarColors["control_hover_fill"] = rgba(toolbar.controlHoverFill);
        toolbarColors["control_border"] = rgba(toolbar.controlBorder);
        toolbarColors["focus_border"] = rgba(toolbar.focusBorder);
        toolbarColors["list_fill"] = rgba(toolbar.listFill);
        toolbarColors["list_border"] = rgba(toolbar.listBorder);
        toolbarColors["selection_fill"] = rgba(toolbar.selectionFill);
        toolbarColors["soft_hover_fill"] = rgba(toolbar.softHoverFill);
        toolbarColors["soft_hover_border"] = rgba(toolbar.softHoverBorder);
        toolbarColors["slider_groove"] = rgba(toolbar.sliderGroove);
        toolbarColors["slider_start"] = rgba(toolbar.sliderStart);
        toolbarColors["slider_end"] = rgba(toolbar.sliderEnd);
        toolbarColors["slider_handle"] = rgba(toolbar.sliderHandle);
        toolbarColors["slider_handle_border"] = rgba(toolbar.sliderHandleBorder);
        toolbarColors["slider_handle_hover_border"] = rgba(toolbar.sliderHandleHoverBorder);
        toolbarColors["tile_fill"] = rgba(toolbar.tileFill);
        toolbarColors["tile_border"] = rgba(toolbar.tileBorder);
        toolbarColors["tile_hover_fill"] = rgba(toolbar.tileHoverFill);
        toolbarColors["tile_hover_border"] = rgba(toolbar.tileHoverBorder);
        toolbarColors["tile_checked_top"] = rgba(toolbar.tileCheckedTop);
        toolbarColors["tile_checked_bottom"] = rgba(toolbar.tileCheckedBottom);
        toolbarColors["tile_checked_border"] = rgba(toolbar.tileCheckedBorder);
        toolbarColors["tile_checked_text"] = rgba(toolbar.tileCheckedText);
        menuColors["background"] = rgba(menu.background);
        menuColors["border"] = rgba(menu.border);
        menuColors["text"] = rgba(menu.text);
        menuColors["secondary_text"] = rgba(menu.secondaryText);
        menuColors["disabled_text"] = rgba(menu.disabledText);
        menuColors["hover_background"] = rgba(menu.hoverBackground);
        menuColors["hover_text"] = rgba(menu.hoverText);
        menuColors["danger_text"] = rgba(menu.dangerText);
        menuColors["danger_hover_background"] = rgba(menu.dangerHoverBackground);
        menuColors["danger_hover_text"] = rgba(menu.dangerHoverText);
        menuColors["separator"] = rgba(menu.separator);
        menuColors["accent"] = rgba(menu.accent);
        menuColors["checked_background"] = rgba(menu.checkedBackground);
        result[dark ? "dark" : "light"] = QJsonObject{{"toolbar", toolbarColors},{"menu", menuColors}};
    }
    std::cout << QJsonDocument(result).toJson().toStdString();
}
