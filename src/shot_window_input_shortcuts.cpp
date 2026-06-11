#include "shot_window_module.h"

using namespace markshot::shot;

/**
 * 处理滚轮输入。
 * @param event 滚轮事件。
 * @return 无返回值。
 */
void ShotWindow::wheelEvent(QWheelEvent *event)
{
    if (m_mode == Mode::Selecting && m_startupTool == StartupTool::ColorPicker) {
        const int delta = event->angleDelta().y() != 0 ? event->angleDelta().y() : event->pixelDelta().y();
        if (delta == 0) {
            QWidget::wheelEvent(event);
            return;
        }

        const qreal factor = std::pow(1.12, static_cast<qreal>(delta) / 120.0);
        m_startupColorLoupeSize = std::clamp(m_startupColorLoupeSize * factor,
                                             kMinStartupColorLoupeSize,
                                             kMaxStartupColorLoupeSize);
        if (m_frozenImageRect.contains(event->position())) {
            m_startupHoverImagePoint = clampImagePoint(widgetToImage(event->position()));
            m_startupHoverValid = true;
        }
        event->accept();
        update();
        return;
    }

    const int steps = event->angleDelta().y() / 120;
    if (steps == 0 || m_mode != Mode::Editing) {
        QWidget::wheelEvent(event);
        return;
    }

    if (m_tool == Tool::Select && !selectedAnnotationIds().isEmpty()) {
        pushHistorySnapshot();
        for (int id : selectedAnnotationIds()) {
            if (Annotation *annotation = annotationById(id)) {
                if (annotation->tool == Tool::Mosaic) {
                    annotation->width = std::clamp(annotation->width + steps * 2.0, kMinMosaicBlockSize, kMaxMosaicBlockSize);
                } else if (annotation->tool == Tool::Number) {
                    annotation->width = std::clamp(annotation->width + steps * 2.0, kMinNumberWidth, kMaxNumberWidth);
                } else if (annotation->tool == Tool::Text) {
                    const qreal oldWidth = annotation->width;
                    annotation->width = std::clamp(annotation->width + steps * 1.5, 1.0, 1000.0);
                    const qreal factor = ((19.0 + annotation->width) / (19.0 + oldWidth)) * 1.05;
                    annotation->rect.setWidth(annotation->rect.width() * factor);
                    annotation->rect = textContentRect(*annotation, false);
                    if (!annotation->points.isEmpty()) {
                        annotation->points[0] = annotation->rect.topLeft();
                    }
                } else {
                    annotation->width = std::clamp(annotation->width + steps * 1.0, kMinStrokeWidth, kMaxStrokeWidth);
                }
            }
        }
        updateColorPalettePreview();
        updateAnnotationPropertyPanel();
        event->accept();
        update();
        return;
    }

    if (wheelZoomsImage()) {
        const qreal factor = imageNavigationWheelFactor(event);
        if (qFuzzyCompare(factor, 1.0)) {
            QWidget::wheelEvent(event);
            return;
        }
        zoomImageAt(factor, event->position());
        m_showWheelPreview = true;
        m_wheelPreviewPosition = event->position();
        m_wheelPreviewTimer.restart();
        updateCursor();
        event->accept();
        update();
        return;
    }

    if (m_tool == Tool::Mosaic) {
        m_mosaicBlockSize = std::clamp(m_mosaicBlockSize + steps * 2.0, kMinMosaicBlockSize, kMaxMosaicBlockSize);
    } else if (m_tool == Tool::Number) {
        m_numberWidth = std::clamp(m_numberWidth + steps * 2.0, kMinNumberWidth, kMaxNumberWidth);
    } else if (m_tool == Tool::Laser) {
        m_laserWidth = std::clamp(m_laserWidth + steps * 2.0, kMinLaserWidth, kMaxLaserWidth);
    } else if (m_tool == Tool::Pen || m_tool == Tool::Highlighter) {
        m_penWidth = std::clamp(m_penWidth + steps * 1.0, kMinStrokeWidth, kMaxStrokeWidth);
    } else if (m_tool == Tool::Text) {
        m_shapeWidth = std::clamp(m_shapeWidth + steps * 1.5, 1.0, 1000.0);
    } else {
        m_shapeWidth = std::clamp(m_shapeWidth + steps * 1.0, kMinStrokeWidth, kMaxStrokeWidth);
    }

    if (m_draft.has_value()) {
        m_draft->width = currentToolWidth();
    }
    m_showWheelPreview = true;
    m_wheelPreviewPosition = event->position();
    m_wheelPreviewTimer.restart();
    updateCursor();
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    event->accept();
    update();
}

/**
 * 处理键盘输入。
 * @param event 键盘事件。
 * @return 无返回值。
 */
void ShotWindow::keyPressEvent(QKeyEvent *event)
{
    clearWheelPreview();

    if (m_mode == Mode::Selecting
        && m_startupTool != StartupTool::None
        && eventMatchesShortcut(event, Action::Cancel)) {
        leaveStartupTool();
        event->accept();
        return;
    }

    if (m_mode == Mode::Selecting) {
        if (eventMatchesStartupShortcut(event, StartupTool::ColorPicker)) {
            setStartupTool(StartupTool::ColorPicker);
            event->accept();
            return;
        }
        if (eventMatchesStartupShortcut(event, StartupTool::Ruler)) {
            setStartupTool(StartupTool::Ruler);
            event->accept();
            return;
        }
    }

    if (imageNavigationAvailable() && event->key() == Qt::Key_Control && !event->isAutoRepeat()) {
        if (m_ctrlTapTimer.isValid() && m_ctrlTapTimer.elapsed() <= kCtrlDoubleTapMs) {
            resetImageZoom();
            m_ctrlTapTimer.invalidate();
        } else {
            m_ctrlTapTimer.restart();
        }
        event->accept();
        return;
    }

    if (handleConfiguredActionShortcut(event)) {
        event->accept();
        return;
    }

    if ((event->key() == Qt::Key_Backspace || event->key() == Qt::Key_Delete)
        && m_mode == Mode::Editing
        && m_tool == Tool::Select
        && !selectedAnnotationIds().isEmpty()) {
        commitTextEditor();
        if (removeSelectedLineSkeletonPoint()) {
            return;
        }
        deleteSelectedAnnotation();
        return;
    }

    switch (event->key()) {
    case Qt::Key_Return:
    case Qt::Key_Enter:
        saveSelection();
        break;
    default:
        if (!handleConfiguredToolShortcut(event)) {
            QWidget::keyPressEvent(event);
        } else {
            event->accept();
        }
        break;
    }
}
