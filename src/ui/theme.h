#pragma once

#include <QColor>
#include <QString>
#include <QVector>

// Design tokens and stylesheet generators for the mark-shot overlay UI.
// All panels share a dark glass aesthetic with a teal accent. Tokens are
// kept here so palette adjustments do not require trawling through panel
// construction code.
namespace markshot::theme {

// Default annotation stroke color (warm red used on first launch).
inline const QColor kDefaultAnnotationColor{255, 77, 77};

// Primary teal accent applied to active states, sliders, and the text editor.
inline const QColor kAccent{94, 234, 212};

// Quick palette shown in the radial color picker and used as the default
// preset list for annotations.
QVector<QColor> paletteColors();

// Stylesheet for the main toolbar, action toolbar, and annotation property
// panel. They share an object-name-scoped rule set so the same string can be
// applied to all three widgets.
QString panelStyleSheet();

// Stylesheet for the "Open With" panel that lists desktop applications.
QString openWithPanelStyleSheet();

// Stylesheet for the inline color dialog panel docked next to the property
// panel.
QString propertyColorDialogPanelStyleSheet();

// Stylesheet for the radial color palette popup.
QString colorPaletteStyleSheet();

// Stylesheet for the inline text annotation editor. The foreground,
// background, and point size depend on the active annotation.
QString textEditorStyleSheet(const QColor &color, const QColor &backgroundColor, int pointSize);

// Stylesheet for the small color preview button inside the property panel.
// The button is filled with the annotation color, so the rule has to be
// regenerated whenever the selection changes.
QString propertyColorButtonStyleSheet(const QColor &fillColor);

}  // namespace markshot::theme
