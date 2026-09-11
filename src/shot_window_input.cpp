#include "shot_window_module.h"

#include "selection_cursor_nudge.h"
#include "selection_loupe.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

/// @brief 处理选区、标注及图像的指针移动
/// @param event 鼠标移动事件
/// @return 无返回值
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

    if (updateStartupShortcutHintAnchor(event->position())) {
        update();
    }

    // 录制中提示面板的停止按钮是手绘控件，悬停反馈在此处理；悬停期间
    // 不再进行窗口预选高亮，避免面板下方的窗口边框闪烁。
    if (updateActiveRecordingStopHover(event->position())) {
        if (m_hoveredWindowRect.has_value()) {
            m_hoveredWindowRect.reset();
            update();
        }
        event->accept();
        return;
    }

    const QPointF imagePoint = (m_mode == Mode::Selecting || canAdjustSelection())
        ? selectingPointerImagePoint(event->position())
        : widgetToImage(event->position());
    // 1. 【截图】【选区调整】同步编辑阶段的放大镜位置，保留方向键使用的软件指针
    if (canAdjustSelection()) {
        if (m_selectionKeyboardAdjusting
            && hardwarePointerMoved(imageToWidget(m_startupHoverImagePoint).toPoint(),
                                    event->position().toPoint())) {
            m_selectionKeyboardAdjusting = false;
            update();
        }
        m_startupHoverImagePoint = clampImagePoint(imagePoint);
        m_startupHoverValid = true;
    }

    const bool startupPointerTool = m_startupTool == StartupTool::ColorPicker
        || m_startupTool == StartupTool::Ruler;
    if (m_mode == Mode::Selecting && startupPointerTool) {
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

    auto selectionLoupeDirty = [this](QPointF hoverPoint) -> QRegion {
        if (!m_selectionLoupeEnabled || m_frozenFrame.isNull()) {
            return {};
        }
        const QPointF widgetPoint = imageToWidget(hoverPoint);
        const SelectionLoupeLayout layout =
            selectionLoupeLayout(widgetPoint,
                                 size(),
                                 m_startupColorLoupeSize,
                                 hoverPoint,
                                 m_frozenFrame.size());
        QRegion dirty(selectionLoupeDirtyRect(layout));
        if (m_selectionPointerDetached) {
            dirty |= QRegion(selectionPointerDirtyRect(widgetPoint));
        }
        return dirty;
    };

    if (m_mode == Mode::Selecting) {
        QRegion dirty;
        if (m_startupHoverValid) {
            dirty |= selectionLoupeDirty(m_startupHoverImagePoint);
        }
        const bool followHardware = !m_selectionPointerDetached
            || hardwarePointerMoved(m_selectionPointerHardwareAnchor,
                                    event->position().toPoint());
        if (followHardware && (m_frozenImageRect.contains(event->position()) || m_dragging)) {
            m_startupHoverImagePoint = clampImagePoint(imagePoint);
            m_startupHoverValid = true;
            dirty |= selectionLoupeDirty(m_startupHoverImagePoint);
        } else if (followHardware) {
            m_startupHoverValid = false;
        } else if (m_startupHoverValid) {
            dirty |= selectionLoupeDirty(m_startupHoverImagePoint);
        }

        if (!m_dragging) {
            const QPoint imgPt = imagePoint.toPoint();
            const std::optional<QRect> bestRect =
                markshot::window_selection::topmostWindowRectAt(m_windowInfos, imgPt);
            if (bestRect != m_hoveredWindowRect) {
                if (m_hoveredWindowRect.has_value()) {
                    dirty |= QRegion(imageRectToWidget(QRectF(*m_hoveredWindowRect)).toAlignedRect());
                }
                m_hoveredWindowRect = bestRect;
                if (m_hoveredWindowRect.has_value()) {
                    dirty |= QRegion(imageRectToWidget(QRectF(*m_hoveredWindowRect)).toAlignedRect());
                }
            }
            if (!dirty.isEmpty()) {
                update(dirty);
            }
        } else {
            const QRectF previousSelection = normalizedSelection();
            const bool previousSelectionUsable = previousSelection.width() >= kMinSelectionSize
                && previousSelection.height() >= kMinSelectionSize;
            m_selection = normalizedRect(m_selectionStart, imagePoint);
            revealSelectionInfo();
            scheduleInitialSelectionRepaint(
                initialSelectionDirtyRegion(previousSelection, previousSelectionUsable) | dirty);
            return;
        }
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

    if (m_mode == Mode::Editing && m_tool == Tool::Move && !m_fullscreenAnnotation && !m_dragging) {
        if (m_selectionPointerDetached) {
            setCursor(Qt::BlankCursor);
            return;
        }
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
            setCursor(Qt::ArrowCursor);
            break;
        }
        return;
    }

    if (canAdjustSelection() && m_dragging && m_selectionDrag != SelectionDrag::None) {
        updateSelectionDrag(imagePoint);
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
        if (m_draft->magnifierShape == MagnifierShape::Rectangle) {
            const QRectF lensRect = clampedMagnifierRect(
                QRectF(clamped.x() - diameter / 2.0, clamped.y() - diameter / 2.0, diameter, diameter));
            m_draft->rect = lensRect;
            m_draft->points[1] = lensRect.center();
        } else {
            const QRectF lensRect = magnifierCircleRect(clamped, diameter);
            m_draft->rect = lensRect;
            m_draft->points[1] = lensRect.center();
        }
    } else if (m_draft->tool == Tool::Number) {
        if (m_draft->points.size() < 2) {
            m_draft->points.append(m_dragStart);
        } else {
            m_draft->points[1] = m_dragStart;
        }
        m_draft->points[0] = clamped;
        m_draft->rect = QRectF(m_dragStart, clamped).normalized();
    } else {
        const bool straightLineDraft = m_draft->tool == Tool::Line
            || m_draft->tool == Tool::Arrow
            || (m_draft->tool == Tool::Highlighter
                && m_draft->highlighterStyle == HighlighterStyle::StraightLine);
        const bool constrainLine = straightLineDraft && event->modifiers().testFlag(Qt::ShiftModifier);
        const QPointF lineEnd = constrainLine
            ? clampImagePoint(markshot::shot::constrainedLineEnd(m_dragStart, clamped))
            : clamped;
        if ((m_draft->tool == Tool::Rectangle || m_draft->tool == Tool::Ellipse || m_draft->tool == Tool::Marker || m_draft->tool == Tool::Magnifier)
            && event->modifiers().testFlag(Qt::ControlModifier)) {
            m_draft->rect = constrainedRect(m_dragStart, clamped);
        } else {
            m_draft->rect = normalizedRect(m_dragStart, lineEnd);
        }
        if (m_draft->points.size() >= 2) {
            m_draft->points[1] = (m_draft->tool == Tool::Rectangle || m_draft->tool == Tool::Ellipse || m_draft->tool == Tool::Marker || m_draft->tool == Tool::Magnifier)
                ? m_draft->rect.bottomRight()
                : lineEnd;
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

    if (m_tool == Tool::Select) {
        const QVector<int> selectedIds = selectedAnnotationIds();
        std::optional<int> targetId = selectedIds.size() == 1
            ? std::optional<int>(selectedIds.first())
            : annotationAt(imagePoint);
        if (targetId.has_value() && !selectedIds.contains(*targetId)) {
            setSelectedAnnotations({*targetId});
        }
        if (targetId.has_value() && insertLineSkeletonPointAt(*targetId, imagePoint)) {
            m_dragging = false;
            m_annotationDrag = SelectionDrag::None;
            m_annotationHistoryCaptured = false;
            event->accept();
            return;
        }
    }

    const auto annotationId = annotationAt(imagePoint);
    if (!annotationId) {
        // 选区内空白处双击执行用户配置的快捷动作（issue #85）
        if (canRunDoubleClickAction(imagePoint) && runConfiguredDoubleClickAction()) {
            event->accept();
            return;
        }
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
    case Tool::Marker:
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

/// @brief 结束选区和标注拖动并提交当前操作
/// @param event 鼠标释放事件
/// @return 无返回值
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
    if (m_mode == Mode::Selecting) {
        flushInitialSelectionRepaint();
    }
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
        const QPointF releasePos = imageToWidget(selectingPointerImagePoint(event->position()));
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
            recordSelectionHistory();
            if (m_startupTool == StartupTool::CodeScanner) {
                scanCodeSelection();
                return;
            }
            if (handleStartupRecordingSelection()) {
                return;
            }
            m_mode = Mode::Editing;
            attachSelectionPointer();
            m_fullscreenAnnotation = false;
            m_toolbarUserPlaced = false;
            m_actionToolbarUserPlaced = false;
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
        recordSelectionHistory();
        if (m_startupTool == StartupTool::CodeScanner) {
            scanCodeSelection();
            return;
        }
        if (handleStartupRecordingSelection()) {
            return;
        }
        m_mode = Mode::Editing;
        attachSelectionPointer();
        m_fullscreenAnnotation = false;
        m_toolbarUserPlaced = false;
        m_actionToolbarUserPlaced = false;
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
        m_selectionKeyboardAdjusting = false;
        attachSelectionPointer();
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
        case Tool::Marker:
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

/// @brief 切换当前标注工具并清理临时交互状态
/// @param tool 新的标注工具
/// @return 无返回值
void ShotWindow::setTool(Tool tool)
{
    m_selectionKeyboardAdjusting = false;
    attachSelectionPointer();
    clearWheelPreview();
    commitAnnotationWidthWheelHistory();
    commitTextEditor();
    m_selectionDrag = SelectionDrag::None;
    m_annotationDrag = SelectionDrag::None;
    m_lineSkeletonDragPointIndex = -1;
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
