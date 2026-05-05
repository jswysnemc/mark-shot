#include "ui/theme.h"

#include <QStringLiteral>

namespace markshot::theme {

QVector<QColor> paletteColors()
{
    return {
        QColor(0xEF, 0x44, 0x44),  // red-500
        QColor(0xF9, 0x73, 0x16),  // orange-500
        QColor(0xEA, 0xB3, 0x08),  // yellow-500
        QColor(0x22, 0xC5, 0x5E),  // green-500
        QColor(0x06, 0xB6, 0xD4),  // cyan-500
        QColor(0x3B, 0x82, 0xF6),  // blue-500
        QColor(0x8B, 0x5C, 0xF6),  // violet-500
        QColor(0xEC, 0x48, 0x99),  // pink-500
        QColor(0xF8, 0xFA, 0xFC),  // slate-50
        QColor(0x0F, 0x17, 0x2A),  // slate-900
    };
}

QString panelStyleSheet()
{
    return QStringLiteral(
        "QWidget#shotToolbar, QWidget#actionToolbar,"
        "QWidget#annotationPropertyPanel, QWidget#propertyColorDialogPanel {"
        " background: rgba(15, 17, 23, 235);"
        " border: 1px solid rgba(255, 255, 255, 14);"
        " border-radius: 14px;"
        "}"
        "QPushButton {"
        " color: #E5E7EB;"
        " background: transparent;"
        " border: 1px solid transparent;"
        " border-radius: 9px;"
        " padding: 0;"
        " min-width: 36px;"
        " min-height: 36px;"
        " max-width: 36px;"
        " max-height: 36px;"
        "}"
        "QPushButton:hover {"
        " background: rgba(45, 212, 191, 28);"
        " border-color: rgba(45, 212, 191, 80);"
        "}"
        "QPushButton:pressed {"
        " background: rgba(45, 212, 191, 70);"
        " border-color: rgba(94, 234, 212, 160);"
        "}"
        "QPushButton[active=\"true\"] {"
        " color: #042F2E;"
        " background: #2DD4BF;"
        " border-color: #5EEAD4;"
        "}"
        "QPushButton[active=\"true\"]:hover {"
        " background: #5EEAD4;"
        " border-color: #99F6E4;"
        "}"
        "QPushButton[role=\"primary\"] {"
        " color: #1F1300;"
        " background: #F59E0B;"
        " border-color: #FBBF24;"
        "}"
        "QPushButton[role=\"primary\"]:hover {"
        " background: #FBBF24;"
        " border-color: #FCD34D;"
        "}"
        "QPushButton[role=\"danger\"]:hover {"
        " color: #FCA5A5;"
        " background: rgba(239, 68, 68, 70);"
        " border-color: rgba(252, 165, 165, 140);"
        "}"
        "QLabel {"
        " color: #E5E7EB;"
        " font-size: 12px;"
        " font-weight: 500;"
        " letter-spacing: 0.2px;"
        " padding: 0 2px;"
        "}"
        "QComboBox {"
        " color: #E5E7EB;"
        " background: rgba(255, 255, 255, 16);"
        " border: 1px solid rgba(255, 255, 255, 24);"
        " border-radius: 8px;"
        " padding: 4px 8px;"
        " min-height: 24px;"
        "}"
        "QComboBox:hover {"
        " border-color: rgba(45, 212, 191, 110);"
        "}"
        "QComboBox::drop-down {"
        " border: 0;"
        " width: 18px;"
        "}"
        "QComboBox QAbstractItemView {"
        " color: #E5E7EB;"
        " background: #111827;"
        " border: 1px solid rgba(255, 255, 255, 18);"
        " border-radius: 8px;"
        " selection-background-color: rgba(45, 212, 191, 70);"
        " selection-color: #E5E7EB;"
        " outline: 0;"
        " padding: 4px;"
        "}"
        "QSlider::groove:horizontal {"
        " height: 4px;"
        " background: rgba(255, 255, 255, 22);"
        " border-radius: 2px;"
        "}"
        "QSlider::sub-page:horizontal {"
        " height: 4px;"
        " background: rgba(45, 212, 191, 180);"
        " border-radius: 2px;"
        "}"
        "QSlider::handle:horizontal {"
        " width: 14px;"
        " height: 14px;"
        " margin: -5px 0;"
        " border-radius: 7px;"
        " background: #2DD4BF;"
        " border: 1px solid #99F6E4;"
        "}"
        "QSlider::handle:horizontal:hover {"
        " background: #5EEAD4;"
        "}");
}

QString openWithPanelStyleSheet()
{
    return QStringLiteral(
        "QWidget#openWithPanel, QWidget#propertyFontPanel {"
        " background: rgba(15, 17, 23, 240);"
        " border: 1px solid rgba(255, 255, 255, 14);"
        " border-radius: 14px;"
        "}"
        "QLabel {"
        " color: #9CA3AF;"
        " font-size: 11px;"
        " font-weight: 600;"
        " letter-spacing: 0.6px;"
        " text-transform: uppercase;"
        " padding: 0 4px 4px 4px;"
        "}"
        "QListWidget {"
        " color: #E5E7EB;"
        " background: transparent;"
        " border: 0;"
        " outline: 0;"
        "}"
        "QListWidget::item {"
        " background: transparent;"
        " border: 1px solid transparent;"
        " border-radius: 8px;"
        " padding: 8px 10px;"
        " margin: 2px 0;"
        "}"
        "QListWidget::item:hover {"
        " background: rgba(45, 212, 191, 28);"
        " border-color: rgba(45, 212, 191, 80);"
        "}"
        "QListWidget::item:selected {"
        " color: #042F2E;"
        " background: #2DD4BF;"
        " border-color: #5EEAD4;"
        "}"
        "QPushButton {"
        " color: #E5E7EB;"
        " text-align: left;"
        " background: transparent;"
        " border: 1px solid transparent;"
        " border-radius: 8px;"
        " padding: 8px 10px;"
        " min-height: 22px;"
        "}"
        "QPushButton:hover {"
        " background: rgba(45, 212, 191, 28);"
        " border-color: rgba(45, 212, 191, 80);"
        "}");
}

QString propertyColorDialogPanelStyleSheet()
{
    // The panel now hosts the lightweight ColorPicker, which paints all of
    // its own controls. We only need framing here.
    return QStringLiteral(
        "QWidget#propertyColorDialogPanel {"
        " background: rgba(15, 17, 23, 240);"
        " border: 1px solid rgba(255, 255, 255, 14);"
        " border-radius: 14px;"
        "}");
}

QString colorPaletteStyleSheet()
{
    return QStringLiteral(
        "QWidget#colorPalette { background: transparent; }"
        "QPushButton {"
        " border: 2px solid rgba(15, 17, 23, 220);"
        " border-radius: 14px;"
        " min-width: 28px;"
        " min-height: 28px;"
        " max-width: 28px;"
        " max-height: 28px;"
        "}"
        "QPushButton:hover {"
        " border: 2px solid #2DD4BF;"
        "}");
}

QString textEditorStyleSheet(const QColor &color, const QColor &backgroundColor, int pointSize)
{
    const QString foreground = QStringLiteral("rgba(%1, %2, %3, %4)")
                                   .arg(color.red())
                                   .arg(color.green())
                                   .arg(color.blue())
                                   .arg(color.alpha());
    const QString background = QStringLiteral("rgba(%1, %2, %3, %4)")
                                   .arg(backgroundColor.red())
                                   .arg(backgroundColor.green())
                                   .arg(backgroundColor.blue())
                                   .arg(backgroundColor.alpha());
    return QStringLiteral(
               "QTextEdit#textEditor {"
               " color: %1;"
               " background: %2;"
               " border: 1px dashed rgba(45, 212, 191, 95);"
               " border-radius: 4px;"
               " padding: 2px 4px;"
               " font-size: %3px;"
               " font-weight: 700;"
               " selection-background-color: #2DD4BF;"
               " selection-color: #042F2E;"
               "}"
               "QTextEdit#textEditor QAbstractScrollArea { background: transparent; }"
               "QTextEdit#textEditor QWidget { background: %2; }")
        .arg(foreground)
        .arg(background)
        .arg(pointSize);
}

QString propertyColorButtonStyleSheet(const QColor &fillColor)
{
    const int luma =
        (fillColor.red() * 299 + fillColor.green() * 587 + fillColor.blue() * 114) / 1000;
    const QString textColor = (luma > 150 && fillColor.alpha() > 120)
        ? QStringLiteral("#0B1120")
        : QStringLiteral("#F8FAFC");
    return QStringLiteral(
               "QPushButton {"
               " color: %5;"
               " background: rgba(%1, %2, %3, %4);"
               " border: 2px solid rgba(255, 255, 255, 64);"
               " border-radius: 9px;"
               " min-width: 36px;"
               " min-height: 36px;"
               " max-width: 36px;"
               " max-height: 36px;"
               " padding: 0;"
               " font-size: 11px;"
               " font-weight: 600;"
               "}"
               "QPushButton:hover {"
               " border-color: #2DD4BF;"
               "}")
        .arg(fillColor.red())
        .arg(fillColor.green())
        .arg(fillColor.blue())
        .arg(fillColor.alpha())
        .arg(textColor);
}

}  // namespace markshot::theme
