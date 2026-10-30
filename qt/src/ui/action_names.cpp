#include "ui/icons.h"

namespace markshot::ui {

/**
 * 【Qt界面】【动作名称】将动作标识转换为界面名称
 * @param action 工具栏动作
 * @return 未翻译的动作名称
 */
QString actionName(ShotWindow::Action action)
{
    switch (action) {
    case ShotWindow::Action::ToolMove:
        return QStringLiteral("Move");
    case ShotWindow::Action::ToolSelect:
        return QStringLiteral("Select");
    case ShotWindow::Action::ToolPen:
        return QStringLiteral("Pen");
    case ShotWindow::Action::ToolLine:
        return QStringLiteral("Line");
    case ShotWindow::Action::ToolHighlighter:
        return QStringLiteral("Highlighter");
    case ShotWindow::Action::ToolRectangle:
        return QStringLiteral("Rect");
    case ShotWindow::Action::ToolEllipse:
        return QStringLiteral("Ellipse");
    case ShotWindow::Action::ToolArrow:
        return QStringLiteral("Arrow");
    case ShotWindow::Action::ToolText:
        return QStringLiteral("Text");
    case ShotWindow::Action::ToolNumber:
        return QStringLiteral("Number");
    case ShotWindow::Action::ToolMosaic:
        return QStringLiteral("Mosaic");
    case ShotWindow::Action::ToolMagnifier:
        return QStringLiteral("Magnifier");
    case ShotWindow::Action::ToolLaser:
        return QStringLiteral("Laser");
    case ShotWindow::Action::ToolMarker:
        return QStringLiteral("Marker");
    case ShotWindow::Action::ToggleCaptureScope:
        return QStringLiteral("Scope");
    case ShotWindow::Action::ToggleToolbarLayout:
        return QStringLiteral("Layout");
    case ShotWindow::Action::Clear:
        return QStringLiteral("Clear");
    case ShotWindow::Action::Undo:
        return QStringLiteral("Undo");
    case ShotWindow::Action::Redo:
        return QStringLiteral("Redo");
    case ShotWindow::Action::OpenWith:
        return QStringLiteral("Open With");
    case ShotWindow::Action::Extensions:
        return QStringLiteral("Extensions");
    case ShotWindow::Action::ScrollCapture:
        return QStringLiteral("Scroll Capture");
    case ShotWindow::Action::Pin:
        return QStringLiteral("Pin");
    case ShotWindow::Action::OcrCopy:
        return QStringLiteral("OCR Copy");
    case ShotWindow::Action::Copy:
        return QStringLiteral("Copy");
    case ShotWindow::Action::Save:
        return QStringLiteral("Save As");
    case ShotWindow::Action::Upload:
        return QStringLiteral("Upload");
    case ShotWindow::Action::Settings:
        return QStringLiteral("Settings");
    case ShotWindow::Action::Cancel:
        return QStringLiteral("Cancel");
    case ShotWindow::Action::Translate:
        return QStringLiteral("Translate");
    case ShotWindow::Action::ToolRuler:
        return QStringLiteral("Ruler");
    }
    return {};
}

} // namespace markshot::ui
