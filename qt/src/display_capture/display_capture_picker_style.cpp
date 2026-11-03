#include "display_capture/display_capture_picker_style.h"

namespace markshot::display_capture {

QString displayCapturePickerStyleSheet(const ui::ToolbarPalette &palette)
{
    using ui::cssColor;
    // 1. 面板、卡片与悬停操作栏取工具栏面板色，卡片悬停使用强调色细边
    const QString panel = QStringLiteral(
                              "QWidget#displayCapturePicker {"
                              " background: %1;"
                              " border: 1px solid %2;"
                              " border-radius: 18px;"
                              "}"
                              "QFrame#displayCaptureCard {"
                              " background: %3;"
                              " border: 1px solid %4;"
                              " border-radius: 12px;"
                              "}"
                              "QFrame#displayCaptureCard:hover {"
                              " background: %5;"
                              " border-color: %6;"
                              "}"
                              "QFrame#displayCaptureActionBar {"
                              " background: %7;"
                              " border: 1px solid %8;"
                              " border-radius: 12px;"
                              "}")
                              .arg(cssColor(palette.panelFill), cssColor(palette.panelBorder),
                                   cssColor(palette.controlFill), cssColor(palette.controlBorder),
                                   cssColor(palette.controlHoverFill), cssColor(palette.focusBorder),
                                   cssColor(palette.popupFill), cssColor(palette.popupBorder));

    // 2. 操作按钮与标题文字使用工具栏前景色，悬停与工具栏按钮一致
    const QString controls = QStringLiteral(
                                 "QPushButton#displayCaptureActionButton {"
                                 " background: %1;"
                                 " border: 1px solid %2;"
                                 " border-radius: 8px;"
                                 " color: %3;"
                                 " font-weight: 800;"
                                 " padding: 4px 8px;"
                                 "}"
                                 "QPushButton#displayCaptureActionButton:hover {"
                                 " background: %4;"
                                 " border-color: %5;"
                                 " color: %6;"
                                 "}"
                                 "QLabel#displayCaptureTitle {"
                                 " color: %7;"
                                 " font-weight: 800;"
                                 "}"
                                 "QLabel#displayCaptureSubtitle {"
                                 " color: %8;"
                                 " font-weight: 600;"
                                 "}")
                                 .arg(cssColor(palette.controlFill), cssColor(palette.controlBorder),
                                      cssColor(palette.strongText), cssColor(palette.hoverFill),
                                      cssColor(palette.hoverBorder), cssColor(palette.hoverText),
                                      cssColor(palette.strongText), cssColor(palette.mutedText));
    return panel + controls;
}

}  // namespace markshot::display_capture
