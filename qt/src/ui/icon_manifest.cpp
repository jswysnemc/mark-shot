#include "ui/icon_manifest.h"

namespace markshot::ui {

QString toolIconName(ShotWindow::Action action)
{
    switch (action) {
    case ShotWindow::Action::ToolMove:
        return QStringLiteral("tool-move");
    case ShotWindow::Action::ToolSelect:
        return QStringLiteral("tool-select");
    case ShotWindow::Action::ToolPen:
        return QStringLiteral("tool-pen");
    case ShotWindow::Action::ToolLine:
        return QStringLiteral("tool-line");
    case ShotWindow::Action::ToolHighlighter:
        return QStringLiteral("tool-highlighter");
    case ShotWindow::Action::ToolRectangle:
        return QStringLiteral("tool-rectangle");
    case ShotWindow::Action::ToolEllipse:
        return QStringLiteral("tool-ellipse");
    case ShotWindow::Action::ToolArrow:
        return QStringLiteral("tool-arrow");
    case ShotWindow::Action::ToolText:
        return QStringLiteral("tool-text");
    case ShotWindow::Action::ToolNumber:
        return QStringLiteral("tool-number");
    case ShotWindow::Action::ToolMosaic:
        return QStringLiteral("tool-mosaic");
    case ShotWindow::Action::ToolMagnifier:
        return QStringLiteral("tool-magnifier");
    case ShotWindow::Action::ToolLaser:
        return QStringLiteral("tool-laser");
    case ShotWindow::Action::ToolMarker:
        return QStringLiteral("tool-marker");
    case ShotWindow::Action::ToggleCaptureScope:
        return QStringLiteral("capture-scope");
    case ShotWindow::Action::ToggleToolbarLayout:
        return QStringLiteral("toolbar-layout");
    case ShotWindow::Action::Clear:
        return QStringLiteral("clear");
    case ShotWindow::Action::Undo:
        return QStringLiteral("undo");
    case ShotWindow::Action::Redo:
        return QStringLiteral("redo");
    case ShotWindow::Action::OpenWith:
        return QStringLiteral("open-with");
    case ShotWindow::Action::Extensions:
        return QStringLiteral("extensions");
    case ShotWindow::Action::Pin:
        return QStringLiteral("pin");
    case ShotWindow::Action::ScrollCapture:
        return QStringLiteral("scroll-capture");
    case ShotWindow::Action::OcrCopy:
        return QStringLiteral("ocr");
    case ShotWindow::Action::Copy:
        return QStringLiteral("copy");
    case ShotWindow::Action::Save:
        return QStringLiteral("save");
    case ShotWindow::Action::Upload:
        return QStringLiteral("upload");
    case ShotWindow::Action::Settings:
        return QStringLiteral("settings");
    case ShotWindow::Action::Cancel:
        return QStringLiteral("cancel");
    case ShotWindow::Action::Translate:
        return QStringLiteral("translate");
    case ShotWindow::Action::ToolRuler:
        return QStringLiteral("tool-ruler");
    }
    return QStringLiteral("tool-select");
}

QString propertyIconName(PropertyIcon icon)
{
    switch (icon) {
    case PropertyIcon::StrokeWidth:
        return QStringLiteral("stroke-width");
    case PropertyIcon::Opacity:
        return QStringLiteral("opacity");
    case PropertyIcon::Color:
        return QStringLiteral("color");
    case PropertyIcon::TextBackground:
        return QStringLiteral("text-background");
    case PropertyIcon::CornerRadius:
        return QStringLiteral("corner-radius");
    case PropertyIcon::MagnifierScale:
        return QStringLiteral("magnifier-scale");
    case PropertyIcon::MagnifierShape:
        return QStringLiteral("magnifier-shape");
    case PropertyIcon::Font:
        return QStringLiteral("font");
    case PropertyIcon::EditText:
        return QStringLiteral("edit-text");
    case PropertyIcon::ResetNumber:
        return QStringLiteral("reset-number");
    }
    return QStringLiteral("stroke-width");
}

}  // namespace markshot::ui
