#include "ui/theme.h"

#include <QApplication>
#include <QFontDatabase>
#include <QStringLiteral>
#include <QStringList>
#include <QToolTip>

namespace markshot::theme {
namespace {

/// @brief Finds the first available font family in the system from a list of candidates.
/// @param candidates List of font family names to check.
/// @param fallback The fallback font family name if no candidates are available.
/// @return The first available candidate font family, or the fallback.
QString firstAvailableFontFamily(const QStringList &candidates, const QString &fallback)
{
    const QStringList families = QFontDatabase::families();
    for (const QString &candidate : candidates) {
        if (families.contains(candidate, Qt::CaseInsensitive)) {
            return candidate;
        }
    }
    return fallback;
}

/// @brief Formats a font family name as a quoted CSS string, escaping special characters.
/// @param family The font family name to format.
/// @return The formatted CSS font family string.
QString quotedCssFontFamily(QString family)
{
    family = family.trimmed();
    if (family.isEmpty()) {
        return {};
    }
    family.replace(QLatin1Char('\\'), QStringLiteral("\\\\"));
    family.replace(QLatin1Char('\''), QStringLiteral("\\'"));
    return QStringLiteral("'%1'").arg(family);
}

/// @brief Creates a QFont object with specified properties.
/// @param family The font family name.
/// @param pointSize The size of the font in points (ignored if <= 0).
/// @param weight The font weight.
/// @return The constructed QFont object.
QFont makeFont(const QString &family, int pointSize, QFont::Weight weight)
{
    QFont font(family);
    if (pointSize > 0) {
        font.setPointSize(pointSize);
    }
    font.setWeight(weight);
    return font;
}

}  // namespace

/// @brief Returns the list of standard colors in the application palette.
/// @return A vector of QColor objects.
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

QString uiFontFamily()
{
#if defined(Q_OS_WIN)
    static const QString family = firstAvailableFontFamily({
        QStringLiteral("Microsoft YaHei UI"),
        QStringLiteral("Segoe UI Variable Text"),
        QStringLiteral("Segoe UI"),
        QStringLiteral("Microsoft YaHei"),
    }, QStringLiteral("Segoe UI"));
    return family;
#else
    return QStringLiteral("Sans Serif");
#endif
}

QString textFontFamily()
{
    return uiFontFamily();
}

QString monospaceFontFamily()
{
#if defined(Q_OS_WIN)
    static const QString family = firstAvailableFontFamily({
        QStringLiteral("Cascadia Mono"),
        QStringLiteral("Consolas"),
        QStringLiteral("JetBrains Mono"),
    }, QStringLiteral("Consolas"));
    return family;
#else
    static const QString family = firstAvailableFontFamily({
        QStringLiteral("JetBrains Mono"),
        QStringLiteral("DejaVu Sans Mono"),
        QStringLiteral("Noto Sans Mono"),
    }, QStringLiteral("monospace"));
    return family;
#endif
}

QString uiFontFamilyCss()
{
    return quotedCssFontFamily(uiFontFamily());
}

QString monospaceFontFamilyCss()
{
    const QString primary = quotedCssFontFamily(monospaceFontFamily());
    return primary.isEmpty()
        ? QStringLiteral("monospace")
        : QStringLiteral("%1, 'DejaVu Sans Mono', monospace").arg(primary);
}

QFont uiFont(int pointSize, QFont::Weight weight)
{
    return makeFont(uiFontFamily(), pointSize, weight);
}

QFont textFont(int pointSize, QFont::Weight weight, QString family)
{
    family = family.trimmed();
    return makeFont(family.isEmpty() ? textFontFamily() : family, pointSize, weight);
}

QFont monospaceFont(int pointSize, QFont::Weight weight)
{
    return makeFont(monospaceFontFamily(), pointSize, weight);
}

void synchronizeToolTipPalette(const QPalette &palette)
{
    QToolTip::setPalette(palette);

    const QColor background = palette.color(QPalette::Active, QPalette::ToolTipBase);
    const QColor foreground = palette.color(QPalette::Active, QPalette::ToolTipText);
    qApp->setStyleSheet(QStringLiteral(
        "QToolTip {"
        " color: %1;"
        " background-color: %2;"
        " border: 1px solid rgba(255, 255, 255, 42);"
        " border-radius: 6px;"
        " padding: 4px 6px;"
        "}")
        .arg(foreground.name(QColor::HexRgb), background.name(QColor::HexRgb)));
}

QString ocrPanelButtonStyleSheet()
{
    return QStringLiteral(
        "QPushButton#ocrPanelButton {"
        " color: #E5E7EB;"
        " text-align: center;"
        " background: transparent;"
        " border: 1px solid transparent;"
        " border-radius: 8px;"
        " padding: 8px 10px;"
        " min-height: 22px;"
        "}"
        "QPushButton#ocrPanelButton:hover {"
        " background: rgba(45, 212, 191, 28);"
        " border-color: rgba(45, 212, 191, 80);"
        "}");
}

QString ocrEditorStyleSheet()
{
    return QStringLiteral(
        "QTextEdit#ocrEditor {"
        " color: #F9FAFB;"
        " background: rgba(2, 6, 12, 200);"
        " border: 1px solid rgba(45, 212, 191, 120);"
        " border-radius: 8px;"
        " padding: 8px 10px;"
        " font-size: 13px;"
        " font-weight: 500;"
        " selection-background-color: #2DD4BF;"
        " selection-color: #042F2E;"
        "}"
        "QTextEdit#ocrEditor:focus {"
        " border: 1px solid rgba(94, 234, 212, 220);"
        "}"
        "QTextEdit#ocrEditor QAbstractScrollArea { background: transparent; }"
        "QScrollBar:vertical { background: transparent; width: 8px; margin: 0; }"
        "QScrollBar::handle:vertical {"
        " background: rgba(94, 234, 212, 90);"
        " border-radius: 4px;"
        " min-height: 24px;"
        "}"
        "QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical { height: 0; }");
}

QString ocrPinButtonStyleSheet()
{
    return QStringLiteral(
        "QPushButton#ocrPinButton {"
        " color: #9CA3AF;"
        " background: transparent;"
        " border: 1px solid transparent;"
        " border-radius: 6px;"
        " padding: 2px;"
        "}"
        "QPushButton#ocrPinButton:hover {"
        " background: rgba(45, 212, 191, 28);"
        " border-color: rgba(45, 212, 191, 80);"
        "}"
        "QPushButton#ocrPinButton:checked {"
        " color: #5EEAD4;"
        " background: rgba(45, 212, 191, 38);"
        " border-color: rgba(45, 212, 191, 110);"
        "}");
}

QString colorPaletteStyleSheet()
{
    return QStringLiteral(
        "QWidget#colorPalette { background: transparent; }"
        "QPushButton {"
        " border: 2px solid rgba(15, 17, 23, 220);"
        " border-radius: 12px;"
        " min-width: 24px;"
        " min-height: 24px;"
        " max-width: 24px;"
        " max-height: 24px;"
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
               " border: 2px solid rgba(255, 255, 255, 100);"
               " border-radius: 9px;"
               " min-width: 28px;"
               " min-height: 28px;"
               " max-width: 28px;"
               " max-height: 28px;"
               " padding: 0;"
               " font-size: 10px;"
               " font-weight: 600;"
               "}"
               "QPushButton:hover {"
               " border-color: #5EEAD4;"
               "}")
        .arg(fillColor.red())
        .arg(fillColor.green())
        .arg(fillColor.blue())
        .arg(fillColor.alpha())
        .arg(textColor);
}

QString menuStyleSheet()
{
    return QStringLiteral(
        "QMenu {"
        " background: #1E293B;"
        " border: 1px solid #334155;"
        " border-radius: 8px;"
        " padding: 4px;"
        "}"
        "QMenu::item {"
        " color: #F1F5F9;"
        " padding: 6px 24px 6px 16px;"
        " border-radius: 4px;"
        "}"
        "QMenu::item:disabled {"
        " color: #64748B;"
        "}"
        "QMenu::item:disabled:selected {"
        " background: transparent;"
        " color: #64748B;"
        "}"
        "QMenu::item:selected {"
        " background: #2DD4BF;"
        " color: #0F172A;"
        "}"
        "QMenu::separator {"
        " height: 1px;"
        " background: #334155;"
        " margin: 4px 8px;"
        "}"
    );
}

}  // namespace markshot::theme
