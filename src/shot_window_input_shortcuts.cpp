#include "shot_window_module.h"

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

    const qreal steps = annotationWidthWheelSteps(event);
    if (qFuzzyIsNull(steps) || m_mode != Mode::Editing) {
        QWidget::wheelEvent(event);
        return;
    }

    const QVector<int> selectedIds = selectedAnnotationIds();
    const bool editingAnnotationWidth = m_tool == Tool::Select && !selectedIds.isEmpty();

    if (!editingAnnotationWidth && wheelZoomsImage()) {
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

    const Annotation *firstSelectedAnnotation = editingAnnotationWidth ? annotationById(selectedIds.first()) : nullptr;
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

    m_showWheelPreview = true;
    m_wheelPreviewPosition = event->position();
    m_wheelPreviewTimer.restart();
    updateCursor();
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
