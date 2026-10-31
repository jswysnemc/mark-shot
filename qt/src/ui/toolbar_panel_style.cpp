#include "ui/toolbar_panel_style.h"

#include <algorithm>

namespace markshot::theme {
namespace {

using markshot::ui::cssColor;
using markshot::ui::ToolbarPalette;

/// @brief 生成面板外框与形状弹层网格按钮规则；需排在通用按钮规则之前
/// @param palette 工具栏配色令牌
/// @param buttonSize 按钮边长
/// @return 样式表片段
QString frameRules(const ToolbarPalette &palette, int buttonSize)
{
    return QStringLiteral(
        "QWidget#shotToolbar, QWidget#actionToolbar,"
        "QWidget#annotationPropertyPanel, QWidget#propertyColorDialogPanel,"
        "QWidget#shapeMarkerPopup {"
        " background: %1;"
        " border: 1px solid %2;"
        " border-radius: 11px;"
        "}"
        "QWidget#shapeMarkerPopup QPushButton {"
        " color: %3;"
        " background: %4;"
        " border: 1px solid %5;"
        " border-radius: 8px;"
        " padding: 0;"
        " min-width: %6px;"
        " min-height: %6px;"
        " max-width: %6px;"
        " max-height: %6px;"
        "}"
        "QWidget#shapeMarkerPopup QPushButton:hover {"
        " background: %7;"
        " border-color: %8;"
        "}"
        "QWidget#shapeMarkerPopup QPushButton:checked,"
        "QWidget#shapeMarkerPopup QPushButton[role=\"primary\"] {"
        " color: %9;"
        " background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 %10, stop:1 %11);"
        " border-color: %12;"
        "}")
        .arg(cssColor(palette.panelFill), cssColor(palette.panelBorder), cssColor(palette.text),
             cssColor(palette.tileFill), cssColor(palette.tileBorder), QString::number(buttonSize),
             cssColor(palette.tileHoverFill), cssColor(palette.tileHoverBorder), cssColor(palette.tileCheckedText),
             cssColor(palette.tileCheckedTop), cssColor(palette.tileCheckedBottom),
             cssColor(palette.tileCheckedBorder));
}

/// @brief 生成工具按钮常态、悬停、按下、选中与各角色规则
/// @param palette 工具栏配色令牌
/// @param buttonSize 按钮边长
/// @param fontSize 字体大小
/// @return 样式表片段
QString buttonRules(const ToolbarPalette &palette, int buttonSize, int fontSize)
{
    return QStringLiteral(
        "QPushButton {"
        " color: %1;"
        " background: transparent;"
        " border: 1px solid transparent;"
        " border-radius: 7px;"
        " padding: 0;"
        " font-size: %2px;"
        " min-width: %3px;"
        " min-height: %3px;"
        " max-width: %3px;"
        " max-height: %3px;"
        "}"
        "QPushButton:hover {"
        " color: %4;"
        " background: %5;"
        " border-color: %6;"
        "}"
        "QPushButton:pressed {"
        " color: %4;"
        " background: %7;"
        " border-color: %8;"
        "}"
        "QPushButton[active=\"true\"] {"
        " color: %1;"
        " background: %9;"
        " border-color: %10;"
        "}"
        "QPushButton[active=\"true\"]:hover {"
        " color: %4;"
        " background: %5;"
        " border-color: %6;"
        "}"
        "QPushButton[role=\"primary\"] {"
        " color: %11;"
        " background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 %12, stop:1 %13);"
        " border: 1px solid %14;"
        "}"
        "QPushButton[role=\"primary\"]:hover {"
        " background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 %15, stop:1 %16);"
        " border-color: %17;"
        "}"
        "QPushButton[role=\"accent\"] {"
        " background: %18;"
        " border: 1px solid %19;"
        "}"
        "QPushButton[role=\"accent\"]:hover {"
        " background: %20;"
        " border-color: %21;"
        "}"
        "QPushButton[role=\"danger\"]:hover {"
        " color: %22;"
        " background: %23;"
        " border-color: %24;"
        "}")
        .arg(cssColor(palette.text), QString::number(fontSize), QString::number(buttonSize),
             cssColor(palette.hoverText), cssColor(palette.hoverFill), cssColor(palette.hoverBorder),
             cssColor(palette.pressedFill), cssColor(palette.pressedBorder), cssColor(palette.activeFill),
             cssColor(palette.activeBorder), cssColor(palette.primaryText), cssColor(palette.primaryTop),
             cssColor(palette.primaryBottom), cssColor(palette.primaryBorder), cssColor(palette.primaryHoverTop),
             cssColor(palette.primaryHoverBottom), cssColor(palette.primaryHoverBorder),
             cssColor(palette.accentFill), cssColor(palette.accentBorder), cssColor(palette.accentHoverFill),
             cssColor(palette.accentHoverBorder), cssColor(palette.dangerHoverText),
             cssColor(palette.dangerHoverFill), cssColor(palette.dangerHoverBorder));
}

/// @brief 生成属性面板标签、标题胶囊、图形标签与数值标签规则
/// @param palette 工具栏配色令牌
/// @param buttonSize 按钮边长
/// @param fontSize 字体大小
/// @return 样式表片段
QString labelRules(const ToolbarPalette &palette, int buttonSize, int fontSize)
{
    const int valueFontSize = std::max(8, fontSize - 1);
    const int controlHeight = std::max(28, buttonSize - 2);
    const int glyphSize = std::max(18, std::min(buttonSize - 8, fontSize + 8));
    return QStringLiteral(
        "QLabel {"
        " color: %1;"
        " font-size: %2px;"
        " font-weight: 500;"
        " letter-spacing: 0.2px;"
        " padding: 0 1px;"
        "}"
        "QLabel#propertyTitle {"
        " color: %3;"
        " background: %4;"
        " border: 1px solid %5;"
        " border-radius: 7px;"
        " padding: 0 8px;"
        " min-height: %6px;"
        " font-weight: 700;"
        "}"
        "QLabel#propertyGlyph {"
        " padding: 0;"
        " min-width: %7px;"
        " max-width: %7px;"
        " min-height: %6px;"
        " max-height: %6px;"
        "}"
        "QLabel#propertyValue {"
        " color: %3;"
        " font-size: %8px;"
        " font-weight: 700;"
        " letter-spacing: 0;"
        " padding: 0;"
        " min-height: %6px;"
        "}")
        .arg(cssColor(palette.text), QString::number(fontSize), cssColor(palette.strongText),
             cssColor(palette.controlFill), cssColor(palette.controlBorder), QString::number(controlHeight),
             QString::number(glyphSize), QString::number(valueFontSize));
}

/// @brief 生成下拉框与滑块规则
/// @param palette 工具栏配色令牌
/// @param fontSize 字体大小
/// @return 样式表片段
QString inputRules(const ToolbarPalette &palette, int fontSize)
{
    const int comboMinHeight = std::max(20, fontSize + 9);
    return QStringLiteral(
        "QComboBox {"
        " color: %1;"
        " background: %2;"
        " border: 1px solid %3;"
        " border-radius: 7px;"
        " padding: 3px 6px;"
        " font-size: %4px;"
        " min-height: %5px;"
        "}"
        "QComboBox:hover {"
        " border-color: %6;"
        "}"
        "QComboBox::drop-down {"
        " border: 0;"
        " width: 18px;"
        "}"
        "QComboBox QAbstractItemView {"
        " color: %1;"
        " background: %7;"
        " border: 1px solid %8;"
        " border-radius: 8px;"
        " selection-background-color: %9;"
        " selection-color: %1;"
        " outline: 0;"
        " padding: 4px;"
        "}"
        "QSlider::groove:horizontal {"
        " height: 4px;"
        " background: %10;"
        " border-radius: 2px;"
        "}"
        "QSlider::sub-page:horizontal {"
        " height: 4px;"
        " background: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 %11, stop:1 %12);"
        " border-radius: 2px;"
        "}"
        "QSlider::handle:horizontal {"
        " width: 12px;"
        " height: 12px;"
        " margin: -4px 0;"
        " border-radius: 6px;"
        " background: %13;"
        " border: 1px solid %14;"
        "}"
        "QSlider::handle:horizontal:hover {"
        " border-color: %15;"
        " background: %13;"
        "}")
        .arg(cssColor(palette.text), cssColor(palette.controlFill), cssColor(palette.controlBorder),
             QString::number(fontSize), QString::number(comboMinHeight), cssColor(palette.focusBorder),
             cssColor(palette.listFill), cssColor(palette.listBorder), cssColor(palette.selectionFill),
             cssColor(palette.sliderGroove), cssColor(palette.sliderStart), cssColor(palette.sliderEnd),
             cssColor(palette.sliderHandle), cssColor(palette.sliderHandleBorder),
             cssColor(palette.sliderHandleHoverBorder));
}

}  // namespace

QString panelStyleSheet()
{
    return panelStyleSheet(markshot::ui::ToolbarPalette{}, 30, 11);
}

QString panelStyleSheet(const ToolbarPalette &palette, int buttonSize, int fontSize)
{
    // 1. 限制尺寸范围，避免异常配置撑破面板
    buttonSize = std::clamp(buttonSize, 24, 64);
    fontSize = std::clamp(fontSize, 8, 24);
    // 2. 外框与弹层规则在前，通用按钮规则在后，保持原有覆盖顺序
    return frameRules(palette, buttonSize) + buttonRules(palette, buttonSize, fontSize)
        + labelRules(palette, buttonSize, fontSize) + inputRules(palette, fontSize);
}

QString openWithPanelStyleSheet(const ToolbarPalette &palette)
{
    return QStringLiteral(
        "QWidget#openWithPanel, QWidget#extensionPanel, QWidget#propertyFontPanel {"
        " background: %1;"
        " border: 1px solid %2;"
        " border-radius: 11px;"
        "}"
        "QLabel {"
        " color: %3;"
        " font-size: 10px;"
        " font-weight: 600;"
        " letter-spacing: 0.6px;"
        " padding: 0 3px 3px 3px;"
        "}"
        "QLineEdit {"
        " color: %4;"
        " background: %5;"
        " border: 1px solid %6;"
        " border-radius: 7px;"
        " padding: 4px 8px;"
        " min-height: 18px;"
        " selection-background-color: %7;"
        " selection-color: %8;"
        "}"
        "QLineEdit:focus { border-color: %7; }"
        "QListWidget {"
        " color: %4;"
        " background: transparent;"
        " border: 0;"
        " outline: 0;"
        "}"
        "QListWidget::item {"
        " background: transparent;"
        " border: 1px solid transparent;"
        " border-radius: 7px;"
        " padding: 5px 8px;"
        " margin: 1px 0;"
        "}"
        "QListWidget::item:hover {"
        " background: %9;"
        " border-color: %10;"
        "}"
        "QListWidget::item:selected {"
        " color: %8;"
        " background: %7;"
        " border-color: %11;"
        "}"
        "QPushButton {"
        " color: %4;"
        " text-align: left;"
        " background: transparent;"
        " border: 1px solid transparent;"
        " border-radius: 7px;"
        " padding: 5px 8px;"
        " min-height: 18px;"
        "}"
        "QPushButton:hover {"
        " background: %9;"
        " border-color: %10;"
        "}")
        .arg(cssColor(palette.popupFill), cssColor(palette.popupBorder), cssColor(palette.mutedText),
             cssColor(palette.text), cssColor(palette.controlFill), cssColor(palette.controlBorder),
             cssColor(palette.hoverFill), cssColor(palette.hoverText), cssColor(palette.softHoverFill),
             cssColor(palette.softHoverBorder), cssColor(palette.hoverBorder));
}

QString propertyColorDialogPanelStyleSheet(const ToolbarPalette &palette)
{
    // 面板内的轻量取色器自绘全部控件，这里只负责外框
    return QStringLiteral(
        "QWidget#propertyColorDialogPanel {"
        " background: %1;"
        " border: 1px solid %2;"
        " border-radius: 11px;"
        "}")
        .arg(cssColor(palette.popupFill), cssColor(palette.popupBorder));
}

}  // namespace markshot::theme
