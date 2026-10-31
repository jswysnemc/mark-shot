#pragma once

#include <QColor>
#include <QFont>
#include <QPalette>
#include <QString>
#include <QVector>

// Design tokens and stylesheet generators for the mark-shot overlay UI.
// Overlay panels use a dark glass aesthetic with a teal accent. Toolbar and
// property panel styles follow the interface theme and live in
// ui/toolbar_panel_style.h.
namespace markshot::theme {

// Default annotation stroke color (warm red used on first launch).
inline const QColor kDefaultAnnotationColor{255, 77, 77};

// Primary teal accent applied to active states, sliders, and the text editor.
inline const QColor kAccent{94, 234, 212};

// Quick palette shown in the radial color picker and used as the default
// preset list for annotations.
QVector<QColor> paletteColors();

// Platform-aware font choices for widgets, painted labels, and annotations.
QString uiFontFamily();
QString textFontFamily();
QString monospaceFontFamily();
QString uiFontFamilyCss();
QString monospaceFontFamilyCss();
QFont uiFont(int pointSize = -1, QFont::Weight weight = QFont::Normal);
QFont textFont(int pointSize = -1, QFont::Weight weight = QFont::Normal, QString family = {});
QFont monospaceFont(int pointSize = -1, QFont::Weight weight = QFont::Normal);

// Keep Qt's process-wide tooltip palette and stylesheet aligned with the
// application palette. This avoids platform styles rendering a dark tooltip
// frame with an unreadable dark text color.
void synchronizeToolTipPalette(const QPalette &palette);

// Stylesheet overrides for OCR result panel action buttons.
QString ocrPanelButtonStyleSheet();

// Stylesheet for the OCR result editor. Makes the recognized-text area the
// visual focus of the panel with a teal border and high-contrast selection.
QString ocrEditorStyleSheet();

// Stylesheet for the always-on-top toggle button in the OCR result title bar.
// The glyph alone communicates state, so the rule keeps the button frameless.
QString ocrPinButtonStyleSheet();

// Stylesheet for the radial color palette popup.
QString colorPaletteStyleSheet();

// Stylesheet for the inline text annotation editor. The foreground,
// background, and point size depend on the active annotation.
QString textEditorStyleSheet(const QColor &color, const QColor &backgroundColor, int pointSize);

// Stylesheet for the small color preview button inside the property panel.
// The button is filled with the annotation color, so the rule has to be
// regenerated whenever the selection changes.
QString propertyColorButtonStyleSheet(const QColor &fillColor);

QString menuStyleSheet();

}  // namespace markshot::theme
