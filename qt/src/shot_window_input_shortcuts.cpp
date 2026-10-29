#include "shot_window_module.h"

#include "selection_cursor_nudge.h"
#include "selection_loupe.h"
#include "capture_history/history_window.h"

#include <algorithm>
#include <optional>

using namespace markshot::shot;

namespace {

/// @brief 计算标注粗细调整使用的滚轮步进
/// @param event 滚轮事件
/// @return 标准滚轮每一格返回 1.0,触控板按像素增量换算为近似步进
qreal annotationWidthWheelSteps(const QWheelEvent *event)
{
    const int angleY = event->angleDelta().y();
    if (angleY != 0) {
        return static_cast<qreal>(angleY) / 120.0;
    }

    const int pixelY = event->pixelDelta().y();
    if (pixelY != 0) {
        return static_cast<qreal>(pixelY) / 80.0;
    }

    return 0.0;
}

/// @brief 判断当前工具或选中标注的滚轮粗细调整粒度
/// @param tool 要调整的工具类型
/// @return 每一格滚轮对应的粗细变化值
qreal annotationWidthWheelStepSize(ShotWindow::Tool tool)
{
    switch (tool) {
    case ShotWindow::Tool::Mosaic:
    case ShotWindow::Tool::Number:
        return 2.0;
    case ShotWindow::Tool::Text:
        return 1.5;
    case ShotWindow::Tool::Move:
    case ShotWindow::Tool::Select:
    case ShotWindow::Tool::Pen:
    case ShotWindow::Tool::Line:
    case ShotWindow::Tool::Highlighter:
    case ShotWindow::Tool::Rectangle:
    case ShotWindow::Tool::Ellipse:
    case ShotWindow::Tool::Marker:
    case ShotWindow::Tool::Arrow:
    case ShotWindow::Tool::Magnifier:
    case ShotWindow::Tool::Laser:
        return 1.0;
    }

    return 1.0;
}

}  // namespace

/**
 * 处理滚轮输入。
 * @param event 滚轮事件。
 * @return 无返回值。
 */
void ShotWindow::wheelEvent(QWheelEvent *event)
{
    // 1. 【截图】【区域识别】选区阶段用滚轮在控件、面板、窗口之间切换
    if (m_mode == Mode::Selecting
        && !m_dragging
        && m_startupTool != StartupTool::Ruler
        && m_startupTool != StartupTool::ColorPicker
        && m_regionDetector
        && !event->modifiers().testFlag(Qt::ShiftModifier)) {
        const int delta = event->angleDelta().y() != 0 ? event->angleDelta().y() : event->pixelDelta().y();
        if (delta != 0) {
            const QPoint imagePoint = selectingPointerImagePoint(event->position()).toPoint();
            if (cycleHoveredRegion(imagePoint, delta > 0 ? 1 : -1)) {
                if (m_frozenImageRect.contains(event->position()) || m_startupHoverValid) {
                    m_startupHoverImagePoint = clampImagePoint(widgetToImage(event->position()));
                    m_startupHoverValid = true;
                }
                event->accept();
                return;
            }
        }
    }

    if ((m_mode == Mode::Selecting || (canAdjustSelection() && m_dragging))
        && m_startupTool != StartupTool::Ruler
        && (m_selectionLoupeEnabled || m_startupTool == StartupTool::ColorPicker)) {
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

    // 1. 【标注】【滚轮输入】正在绘制或拖动时保持当前操作，避免预览覆盖抓取光标
    if (m_dragging || m_imagePanning || m_toolbarDragging || m_startupRulerDragging) {
        event->accept();
        return;
    }

    const qreal steps = annotationWidthWheelSteps(event);
    if (qFuzzyIsNull(steps) || m_mode != Mode::Editing) {
        QWidget::wheelEvent(event);
        return;
    }

    // 1. 【标注】【滚轮选中】粗细滚轮生效前，先选中当前工具刚画完的图形
    selectLatestDrawnAnnotationForWheel();
    const QVector<int> selectedIds = selectedAnnotationIds();
    const bool editingAnnotationWidth = m_tool == Tool::Select && !selectedIds.isEmpty();

    if (!editingAnnotationWidth && wheelZoomsImage()) {
        const qreal factor = imageNavigationWheelFactor(event);
        if (qFuzzyCompare(factor, 1.0)) {
            QWidget::wheelEvent(event);
            return;
        }
        zoomImageAt(factor, event->position());
        startWheelPreview(event->position(), WheelPreview::ImageZoom);
        event->accept();
        return;
    }

    const Annotation *firstSelectedAnnotation = nullptr;
    if (m_tool == Tool::Select && !selectedIds.isEmpty()) {
        firstSelectedAnnotation = annotationById(selectedIds.first());
    }
    const Tool widthTool = firstSelectedAnnotation ? firstSelectedAnnotation->tool : m_tool;
    const int wheelContext = static_cast<int>(widthTool) * 4096
        + (editingAnnotationWidth && firstSelectedAnnotation ? firstSelectedAnnotation->id : 0);
    if (m_annotationWidthWheelContext != wheelContext) {
        m_annotationWidthWheelContext = wheelContext;
        m_annotationWidthWheelRemainder = 0.0;
    }

    const qreal currentWidth = firstSelectedAnnotation ? firstSelectedAnnotation->width : currentToolWidth();
    const qreal rawDelta = steps * annotationWidthWheelStepSize(widthTool) + m_annotationWidthWheelRemainder;
    const int delta = rawDelta > 0.0 ? static_cast<int>(std::floor(rawDelta)) : static_cast<int>(std::ceil(rawDelta));
    m_annotationWidthWheelRemainder = rawDelta - delta;
    if (delta == 0) {
        event->accept();
        return;
    }

    const HistorySnapshot historyBeforeChange = editingAnnotationWidth
        ? currentHistorySnapshot()
        : HistorySnapshot{};
    const bool changed = setSelectedAnnotationWidth(qRound(currentWidth) + delta, false);
    if (!changed) {
        event->accept();
        return;
    }
    if (editingAnnotationWidth) {
        queueAnnotationWidthWheelHistory(wheelContext, historyBeforeChange);
    }

    startWheelPreview(event->position(), WheelPreview::ToolSize);
    event->accept();
}

/**
 * 处理键盘输入。
 * @param event 键盘事件。
 * @return 无返回值。
 */
void ShotWindow::keyPressEvent(QKeyEvent *event)
{
    clearWheelPreview();

    if (handleSelectionAdjustmentKey(event)) {
        return;
    }

    // 选区历史浏览（issue #79）：逗号回到上一次截图的选区，句号前进；恢复后可继续切换
    if (isSelectionHistoryKey(event)) {
        applySelectionHistoryStep(event->key() == Qt::Key_Comma ? 1 : -1);
        event->accept();
        return;
    }

    // 【截图】【选区尺寸】Ctrl+R 直接编辑左上角 HUD 的宽度，不再弹出独立面板
    if (event->key() == Qt::Key_R && event->modifiers() == Qt::ControlModifier && !m_dragging
        && m_mode == Mode::Editing && !m_fullscreenAnnotation) {
        focusSelectionSizeInput();
        event->accept();
        return;
    }

    if (event->key() == Qt::Key_H && event->modifiers() == Qt::ControlModifier && !m_dragging) {
        markshot::history::showHistoryWindow();
        close();
        event->accept();
        return;
    }

    if (m_mode == Mode::Selecting && activeRecordingAvailable() && event->key() == Qt::Key_S) {
        stopActiveRecordingFromOverlay();
        event->accept();
        return;
    }

    if (m_mode == Mode::Selecting
        && displayCapturePickerVisible()
        && eventMatchesShortcut(event, Action::Cancel)) {
        hideDisplayCapturePicker();
        event->accept();
        return;
    }

    if (m_mode == Mode::Selecting
        && m_startupTool != StartupTool::None
        && eventMatchesShortcut(event, Action::Cancel)) {
        leaveStartupTool();
        event->accept();
        return;
    }

    if (m_mode == Mode::Selecting) {
        if (event->isAutoRepeat()
            && (eventMatchesDisplayCaptureShortcut(event)
                || eventMatchesStartupShortcut(event, StartupTool::ColorPicker)
                || eventMatchesStartupShortcut(event, StartupTool::Ruler)
                || eventMatchesStartupShortcut(event, StartupTool::CodeScanner)
                || eventMatchesStartupShortcut(event, StartupTool::GifRecorder)
                || eventMatchesStartupShortcut(event, StartupTool::VideoRecorder))) {
            event->accept();
            return;
        }
        if (activeRecordingAvailable()
            && (eventMatchesStartupShortcut(event, StartupTool::GifRecorder)
                || eventMatchesStartupShortcut(event, StartupTool::VideoRecorder))) {
            event->accept();
            return;
        }
        if (eventMatchesDisplayCaptureShortcut(event)) {
            toggleDisplayCapturePicker();
            event->accept();
            return;
        }
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
        if (eventMatchesStartupShortcut(event, StartupTool::CodeScanner)) {
            setStartupTool(StartupTool::CodeScanner);
            event->accept();
            return;
        }
        if (eventMatchesStartupShortcut(event, StartupTool::GifRecorder)) {
            beginStartupRecording(markshot::recording::RecordingMode::Gif);
            event->accept();
            return;
        }
        if (eventMatchesStartupShortcut(event, StartupTool::VideoRecorder)) {
            beginStartupRecording(markshot::recording::RecordingMode::Video);
            event->accept();
            return;
        }
        if (m_selectionLoupeEnabled
            && m_startupTool != StartupTool::Ruler
            && isSelectionCursorNudgeKey(event->key())
            && !m_frozenFrame.isNull()) {
            if (!m_startupHoverValid) {
                m_startupHoverImagePoint = widgetToImage(mapFromGlobal(QCursor::pos()));
                m_startupHoverValid = true;
            }

            const QPointF previousWidgetPoint = imageToWidget(m_startupHoverImagePoint);
            QRegion dirty = QRegion(selectionLoupeDirtyRect(
                selectionLoupeLayout(previousWidgetPoint,
                                     size(),
                                     m_startupColorLoupeSize,
                                     m_startupHoverImagePoint,
                                     m_frozenFrame.size())));
            if (m_selectionPointerDetached) {
                dirty |= QRegion(selectionPointerDirtyRect(previousWidgetPoint));
            }
            const QPoint delta =
                selectionCursorNudgeDelta(event->key(), event->modifiers().testFlag(Qt::ShiftModifier));
            const QRectF bounds(0.0,
                                0.0,
                                std::max(0, m_frozenFrame.width() - 1),
                                std::max(0, m_frozenFrame.height() - 1));
            m_startupHoverImagePoint = nudgeSelectionCursor(m_startupHoverImagePoint, delta, bounds);
            const QPoint widgetPoint = imageToWidget(m_startupHoverImagePoint).toPoint();
            detachSelectionPointerIfWarpFailed(widgetPoint);
            if (m_selectionPointerDetached) {
                dirty |= QRegion(selectionPointerDirtyRect(QPointF(widgetPoint)));
            }
            dirty |= QRegion(selectionLoupeDirtyRect(
                selectionLoupeLayout(imageToWidget(m_startupHoverImagePoint),
                                     size(),
                                     m_startupColorLoupeSize,
                                     m_startupHoverImagePoint,
                                     m_frozenFrame.size())));

            if (m_dragging) {
                const QRectF previousSelection = normalizedSelection();
                const bool previousSelectionUsable = previousSelection.width() >= kMinSelectionSize
                    && previousSelection.height() >= kMinSelectionSize;
                m_selection = creationSelectionRect(m_selectionStart, m_startupHoverImagePoint, false);
                dirty |= dismissRecognizedSelectionOnManualDragIfEnabled();
                revealSelectionInfo();
                scheduleInitialSelectionRepaint(
                    initialSelectionDirtyRegion(previousSelection, previousSelectionUsable) | dirty);
            } else {
                dirty |= refreshHoveredRegion(m_startupHoverImagePoint.toPoint());
                update(dirty);
            }
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

namespace {

// 滚轮改粗细时先选中刚画完的图形
constexpr bool kSelectLatestDrawnOnWidthWheel = true;

}

/// @brief 滚轮开始调整粗细时，选中当前工具刚提交的图形。
/// @return 已选中刚绘制的图形时返回 true。
bool ShotWindow::selectLatestDrawnAnnotationForWheel()
{
    if (!kSelectLatestDrawnOnWidthWheel) {
        return false;
    }
    // 1. 【标注】【滚轮选中】已有选中项、正在绘制或文字编辑中时不抢当前操作
    if (m_mode != Mode::Editing || m_dragging || m_annotations.isEmpty() || !selectedAnnotationIds().isEmpty()) {
        return false;
    }
    if (m_textEditor && m_textEditor->isVisible()) {
        return false;
    }
    const Annotation &latest = m_annotations.last();
    if (latest.id == 0 || latest.tool != m_tool) {
        return false;
    }
    switch (latest.tool) {
    case Tool::Line:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Marker:
    case Tool::Arrow:
    case Tool::Number:
    case Tool::Magnifier:
        break;
    case Tool::Move:
    case Tool::Select:
    case Tool::Pen:
    case Tool::Highlighter:
    case Tool::Text:
    case Tool::Mosaic:
    case Tool::Laser:
        return false;
    }
    // 2. 【标注】【滚轮选中】切到选择工具后再写入选中，避免切换工具时清掉选中
    const int id = latest.id;
    setTool(Tool::Select);
    setSelectedAnnotations({id});
    updateAnnotationPropertyPanel();
    return selectedAnnotationIds().contains(id);
}
