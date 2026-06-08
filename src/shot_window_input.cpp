#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

void ShotWindow::mouseMoveEvent(QMouseEvent *event)
{
    if (m_imagePanning) {
        panImageTo(event->position());
        event->accept();
        return;
    }

    if (m_showWheelPreview && m_wheelPreviewTimer.isValid() && m_wheelPreviewTimer.elapsed() <= 900) {
        m_wheelPreviewPosition = event->position();
        update();
    } else if (m_showWheelPreview) {
        m_showWheelPreview = false;
        updateCursor();
        update();
    }

    const QPointF imagePoint = widgetToImage(event->position());
    if (m_mode == Mode::Selecting && m_startupTool != StartupTool::None) {
        if (m_frozenImageRect.contains(event->position()) || m_startupRulerDragging) {
            m_startupHoverImagePoint = clampImagePoint(imagePoint);
            m_startupHoverValid = true;
            if (m_startupTool == StartupTool::Ruler && m_startupRulerDragging) {
                m_startupRulerEnd = m_startupHoverImagePoint;
            }
        } else {
            m_startupHoverValid = false;
        }
        update();
        event->accept();
        return;
    }

    if (m_mode == Mode::Selecting && !m_dragging) {
        std::optional<QRect> best;
        qint64 bestArea = std::numeric_limits<qint64>::max();
        const QPoint imgPt = imagePoint.toPoint();
        for (const QRect &r : std::as_const(m_windowRects)) {
            if (r.contains(imgPt)) {
                qint64 area = static_cast<qint64>(r.width()) * r.height();
                if (area < bestArea) {
                    bestArea = area;
                    best = r;
                }
            }
        }
        if (best != m_hoveredWindowRect) {
            m_hoveredWindowRect = best;
            update();
        }
    }
    if (m_mode == Mode::Selecting && m_dragging) {
        m_selection = normalizedRect(m_selectionStart, imagePoint);
        revealSelectionInfo();
        update();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Select && m_dragging && m_annotationDrag != SelectionDrag::None) {
        updateAnnotationDrag(imagePoint, event->modifiers().testFlag(Qt::ControlModifier));
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Select && m_dragging && m_annotationSelectionBoxActive) {
        updateAnnotationSelectionBox(imagePoint);
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Select && !m_dragging) {
        if (selectedAnnotationIds().size() > 1) {
            m_annotationDrag = selectedAnnotationsDragAt(imagePoint);
            if (m_annotationDrag != SelectionDrag::None) {
                updateCursor();
                return;
            }
        } else if (m_selectedAnnotationId.has_value()) {
            m_annotationDrag = annotationDragAt(imagePoint, *m_selectedAnnotationId);
            if (m_annotationDrag != SelectionDrag::None) {
                updateCursor();
                return;
            }
        }
        m_annotationDrag = annotationAt(imagePoint).has_value() ? SelectionDrag::Move : SelectionDrag::None;
        updateCursor();
        return;
    }

    if (m_fullscreenAnnotation && m_toolbarDragging) {
        const QPoint delta = event->pos() - m_toolbarDragStart;
        QRect toolbarGeometry = m_toolbarBeforeDrag.translated(delta);
        if (m_toolbar) {
            m_toolbarUserPlaced = true;
            m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        }
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Move && !m_fullscreenAnnotation && !m_dragging) {
        const SelectionDrag hoverDrag = selectionDragAt(imagePoint);
        switch (hoverDrag) {
        case SelectionDrag::MagnifierSource:
        case SelectionDrag::MagnifierLens:
        case SelectionDrag::Rotate:
        case SelectionDrag::LineControl:
        case SelectionDrag::NumberTip:
        case SelectionDrag::NumberBubble:
            setCursor(Qt::SizeAllCursor);
            break;
        case SelectionDrag::Left:
        case SelectionDrag::Right:
            setCursor(Qt::SizeHorCursor);
            break;
        case SelectionDrag::Top:
        case SelectionDrag::Bottom:
            setCursor(Qt::SizeVerCursor);
            break;
        case SelectionDrag::TopLeft:
        case SelectionDrag::BottomRight:
            setCursor(Qt::SizeFDiagCursor);
            break;
        case SelectionDrag::TopRight:
        case SelectionDrag::BottomLeft:
            setCursor(Qt::SizeBDiagCursor);
            break;
        case SelectionDrag::Move:
            setCursor(Qt::SizeAllCursor);
            break;
        case SelectionDrag::None:
            setCursor(captureCrossCursor());
            break;
        }
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Move && !m_fullscreenAnnotation && m_dragging && m_selectionDrag != SelectionDrag::None) {
        const QPointF clamped = clampImagePoint(imagePoint);
        const QRectF start = m_selectionBeforeDrag;
        const qreal maxWidth = m_frozenFrame.width();
        const qreal maxHeight = m_frozenFrame.height();
        qreal left = start.left();
        qreal top = start.top();
        qreal right = start.right();
        qreal bottom = start.bottom();

        if (m_selectionDrag == SelectionDrag::Move) {
            const QPointF delta = clamped - m_dragStart;
            left = std::clamp(start.left() + delta.x(), 0.0, std::max<qreal>(0.0, maxWidth - start.width()));
            top = std::clamp(start.top() + delta.y(), 0.0, std::max<qreal>(0.0, maxHeight - start.height()));
            right = left + start.width();
            bottom = top + start.height();
        } else {
            if (m_selectionDrag == SelectionDrag::Left || m_selectionDrag == SelectionDrag::TopLeft
                || m_selectionDrag == SelectionDrag::BottomLeft) {
                left = std::clamp(clamped.x(), 0.0, right - kMinSelectionSize);
            }
            if (m_selectionDrag == SelectionDrag::Right || m_selectionDrag == SelectionDrag::TopRight
                || m_selectionDrag == SelectionDrag::BottomRight) {
                right = std::clamp(clamped.x(), left + kMinSelectionSize, maxWidth);
            }
            if (m_selectionDrag == SelectionDrag::Top || m_selectionDrag == SelectionDrag::TopLeft
                || m_selectionDrag == SelectionDrag::TopRight) {
                top = std::clamp(clamped.y(), 0.0, bottom - kMinSelectionSize);
            }
            if (m_selectionDrag == SelectionDrag::Bottom || m_selectionDrag == SelectionDrag::BottomLeft
                || m_selectionDrag == SelectionDrag::BottomRight) {
                bottom = std::clamp(clamped.y(), top + kMinSelectionSize, maxHeight);
            }
        }

        m_selection = QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
        revealSelectionInfo();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        updateTextEditorGeometry();
        update();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Laser && m_dragging && m_laserDraft.has_value()) {
        updateLaserStroke(imagePoint);
        return;
    }

    if (m_mode != Mode::Editing || !m_dragging || !m_draft.has_value()) {
        return;
    }

    const QPointF clamped = clampImagePoint(imagePoint);
    if (m_draft->tool == Tool::Pen
        || (m_draft->tool == Tool::Highlighter
            && m_draft->highlighterStyle == HighlighterStyle::Freehand)) {
        m_draft->points.append(clamped);
    } else if (m_draft->tool == Tool::Magnifier) {
        const qreal dragDistance = QLineF(m_dragStart, clamped).length();
        if (dragDistance < kMinMagnifierDragDistance) {
            m_draft->rect = QRectF(m_dragStart, m_dragStart);
            m_draft->points[1] = clamped;
            update();
            return;
        }

        const qreal frameDiameter = std::min<qreal>(m_frozenFrame.width(), m_frozenFrame.height());
        const qreal maxDiameter = std::max<qreal>(4.0,
                                                  std::min(kMaxMagnifierDiameter,
                                                           frameDiameter));
        const qreal minDiameter = std::min(kMinMagnifierDiameter, maxDiameter);
        const qreal diameter = std::clamp(dragDistance * kMagnifierDragScale,
                                          minDiameter,
                                          maxDiameter);
        const QRectF lensRect = magnifierCircleRect(clamped, diameter);
        m_draft->rect = lensRect;
        m_draft->points[1] = lensRect.center();
    } else if (m_draft->tool == Tool::Number) {
        if (m_draft->points.size() < 2) {
            m_draft->points.append(m_dragStart);
        } else {
            m_draft->points[1] = m_dragStart;
        }
        m_draft->points[0] = clamped;
        m_draft->rect = QRectF(m_dragStart, clamped).normalized();
    } else {
        if ((m_draft->tool == Tool::Rectangle || m_draft->tool == Tool::Ellipse || m_draft->tool == Tool::Magnifier)
            && event->modifiers().testFlag(Qt::ControlModifier)) {
            m_draft->rect = constrainedRect(m_dragStart, clamped);
        } else {
            m_draft->rect = normalizedRect(m_dragStart, clamped);
        }
        if (m_draft->points.size() >= 2) {
            m_draft->points[1] = (m_draft->tool == Tool::Rectangle || m_draft->tool == Tool::Ellipse || m_draft->tool == Tool::Magnifier)
                ? m_draft->rect.bottomRight()
                : clamped;
        }
    }
    update();
}

void ShotWindow::mouseDoubleClickEvent(QMouseEvent *event)
{
    if (event->button() == Qt::RightButton && m_mode == Mode::Editing) {
        toggleColorPalette(event->pos());
        event->accept();
        return;
    }

    if (event->button() != Qt::LeftButton || m_mode != Mode::Editing) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const QPointF imagePoint = widgetToImage(event->position());
    if (!m_frozenImageRect.contains(event->position())) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const auto annotationId = annotationAt(imagePoint);
    if (!annotationId) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const Annotation *annotation = annotationById(*annotationId);
    if (!annotation || annotation->tool != Tool::Text) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const int targetId = *annotationId;

    // The single click that preceded this double-click already executed
    // mousePressEvent in the active tool's branch. Roll back its side
    // effects so the user does not see a stray duplicate annotation when
    // we transition into text editing.
    switch (m_tool) {
    case Tool::Number:
        m_draft.reset();
        break;
    case Tool::Pen:
    case Tool::Highlighter:
    case Tool::Line:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Arrow:
    case Tool::Mosaic:
    case Tool::Magnifier:
        // First press created an in-flight draft; discard it so the
        // upcoming mouseReleaseEvent (which still fires for the second
        // click of the double-click) does not commit a tiny stamp.
        m_draft.reset();
        break;
    case Tool::Laser:
        m_laserDraft.reset();
        break;
    case Tool::Text:
        // First press opened a fresh, empty text editor at the click point.
        // setTool(Select) below will call commitTextEditor() and tear it
        // down without producing an empty annotation.
        break;
    case Tool::Move:
    case Tool::Select:
        // No draft to discard.
        break;
    }

    m_dragging = false;
    m_annotationDrag = SelectionDrag::None;
    m_annotationHistoryCaptured = false;

    if (m_tool != Tool::Select) {
        setTool(Tool::Select);
    }
    setSelectedAnnotations({targetId});
    beginEditingSelectedTextAnnotation();
    update();
    event->accept();
}

void ShotWindow::mouseReleaseEvent(QMouseEvent *event)
{
    if (m_mode == Mode::Selecting
        && m_startupTool == StartupTool::Ruler
        && event->button() == Qt::LeftButton
        && m_startupRulerDragging) {
        m_startupRulerDragging = false;
        if (m_frozenImageRect.contains(event->position())) {
            m_startupRulerEnd = clampImagePoint(widgetToImage(event->position()));
            m_startupHoverImagePoint = m_startupRulerEnd;
            m_startupHoverValid = true;
        }
        update();
        event->accept();
        return;
    }

    if ((event->button() == Qt::LeftButton || event->button() == Qt::MiddleButton) && m_imagePanning) {
        m_imagePanning = false;
        updateCursor();
        event->accept();
        return;
    }

    if (event->button() != Qt::LeftButton || !m_dragging) {
        return;
    }

    m_dragging = false;
    if (m_toolbarDragging) {
        m_toolbarDragging = false;
        updateCursor();
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    if (m_tool == Tool::Select && m_annotationDrag != SelectionDrag::None) {
        m_annotationDrag = SelectionDrag::None;
        m_annotationHistoryCaptured = false;
        updateAnnotationPropertyPanel();
        updateCursor();
        update();
        return;
    }

    if (m_tool == Tool::Select && m_annotationSelectionBoxActive) {
        if (m_imageNavigationEnabled && m_imageSelected) {
            const QRectF box = m_annotationSelectionBox.normalized();
            if (box.width() < kMinSelectionSize || box.height() < kMinSelectionSize) {
                m_annotationSelectionBoxActive = false;
                m_annotationSelectionBox = {};
                updateAnnotationPropertyPanel();
                updateCursor();
                update();
                return;
            }
            m_imageSelected = false;
        }
        commitAnnotationSelectionBox();
        updateCursor();
        update();
        return;
    }

    if (m_mode == Mode::Selecting) {
        const QPointF releasePos = event->position();
        const qreal clickDistance = QLineF(m_selectionClickStart, releasePos).length();
        if (clickDistance < 5.0 && m_hoveredWindowRect.has_value()) {
            m_selection = QRectF(*m_hoveredWindowRect);
            m_hoveredWindowRect.reset();
            m_dragging = false;
            if (!hasUsableSelection()) {
                m_selection = {};
                update();
                return;
            }
            m_mode = Mode::Editing;
            m_fullscreenAnnotation = false;
            m_toolbarUserPlaced = false;
            setTool(defaultEditingTool());
            setFullscreenActionButtonsVisible(false);
            m_toolbar->show();
            m_actionToolbar->show();
            emit selectionActivated(this);
            revealSelectionInfo();
            updateToolbarGeometry();
            updateActionToolbarGeometry();
            update();
            return;
        }
        m_hoveredWindowRect.reset();
        m_selection = normalizedSelection();
        if (!hasUsableSelection()) {
            m_selection = {};
            update();
            return;
        }
        m_mode = Mode::Editing;
        m_fullscreenAnnotation = false;
        m_toolbarUserPlaced = false;
        setTool(defaultEditingTool());
        setFullscreenActionButtonsVisible(false);
        m_toolbar->show();
        m_actionToolbar->show();
        emit selectionActivated(this);
        revealSelectionInfo();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        update();
        return;
    }

    if (m_tool == Tool::Move && !m_fullscreenAnnotation && m_selectionDrag != SelectionDrag::None) {
        m_selection = normalizedSelection();
        m_selectionDrag = SelectionDrag::None;
        revealSelectionInfo();
        updateCursor();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        update();
        return;
    }

    if (m_tool == Tool::Laser && m_laserDraft.has_value()) {
        commitLaserStroke();
        updateCursor();
        update();
        return;
    }

    commitDraft();
}

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

void ShotWindow::beginSelection(QPointF imagePoint)
{
    m_dragging = true;
    m_fullscreenAnnotation = false;
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_selectionDrag = SelectionDrag::None;
    m_selectionBeforeFullscreenAnnotation.reset();
    m_selectionStart = imagePoint;
    m_selection = QRectF(imagePoint, imagePoint);
    if (m_textEditor) {
        m_textEditor->hide();
        m_textEditor->clear();
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    setFullscreenActionButtonsVisible(false);
    m_annotations.clear();
    m_undoStack.clear();
    m_redoStack.clear();
    m_draft.reset();
    m_laserStrokes.clear();
    m_laserDraft.reset();
    setSelectedAnnotations({});
    m_nextNumber = 1;
    m_nextAnnotationId = 1;
    revealSelectionInfo();
    update();
}

void ShotWindow::commitDraft()
{
    if (!m_draft.has_value()) {
        return;
    }

    const bool highlighterLineDraft = m_draft->tool == Tool::Highlighter
        && m_draft->highlighterStyle == HighlighterStyle::StraightLine;
    const bool highlighterFreehandDraft = m_draft->tool == Tool::Highlighter
        && m_draft->highlighterStyle == HighlighterStyle::Freehand;

    if ((m_draft->tool == Tool::Pen || highlighterFreehandDraft) && m_draft->points.size() < 2) {
        m_draft.reset();
        update();
        return;
    }

    if ((m_draft->tool == Tool::Line || m_draft->tool == Tool::Arrow || highlighterLineDraft)
        && m_draft->points.size() >= 2
        && QLineF(m_draft->points.first(), m_draft->points.last()).length() < 2.0) {
        m_draft.reset();
        update();
        return;
    }

    if (m_draft->tool != Tool::Pen && !highlighterFreehandDraft && !highlighterLineDraft && m_draft->tool != Tool::Line
        && m_draft->tool != Tool::Arrow && m_draft->tool != Tool::Text && m_draft->tool != Tool::Number
        && (m_draft->rect.width() < 2.0 || m_draft->rect.height() < 2.0)) {
        m_draft.reset();
        update();
        return;
    }

    pushHistorySnapshot();
    if (m_draft->id == 0) {
        m_draft->id = m_nextAnnotationId++;
    }
    const int committedId = m_draft->id;
    const Tool committedTool = m_draft->tool;
    if (m_draft->tool == Tool::Number) {
        if (m_draft->number <= 0) {
            m_draft->number = m_nextNumber;
        }
        m_nextNumber = std::max(m_nextNumber, m_draft->number + 1);
    }
    m_annotations.append(*m_draft);
    m_draft.reset();
    if (m_autoSelectAfterDrawByTool.at(static_cast<int>(committedTool))) {
        switch (committedTool) {
        case Tool::Line:
        case Tool::Rectangle:
        case Tool::Ellipse:
        case Tool::Arrow:
        case Tool::Number:
        case Tool::Magnifier:
            setTool(Tool::Select);
            setSelectedAnnotations({committedId});
            updateAnnotationPropertyPanel();
            break;
        case Tool::Move:
        case Tool::Select:
        case Tool::Pen:
        case Tool::Highlighter:
        case Tool::Text:
        case Tool::Mosaic:
        case Tool::Laser:
            break;
        }
    }
    update();
}

void ShotWindow::setTool(Tool tool)
{
    clearWheelPreview();
    commitTextEditor();
    m_selectionDrag = SelectionDrag::None;
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    if (tool != Tool::Laser) {
        m_laserDraft.reset();
    }
    m_tool = tool;
    if (m_tool != Tool::Select) {
        setSelectedAnnotations({});
        m_imageSelected = false;
        m_imagePanning = false;
    }
    updateAnnotationPropertyPanel();
    updateCursor();
    updateToolbarState();
    update();
}
