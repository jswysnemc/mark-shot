#include "shot_window_module.h"

#include "selection_adjustment.h"
#include "selection_cursor_nudge.h"

using namespace markshot::shot;

bool ShotWindow::canAdjustSelection() const
{
    return m_mode == Mode::Editing && m_tool == Tool::Move
        && !m_fullscreenAnnotation && !m_toolbarDragging && !m_imagePanning
        && hasUsableSelection();
}

void ShotWindow::refreshAdjustedSelection()
{
    revealSelectionInfo();
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
    updateTextEditorGeometry();
    updateSelectionSizePanelGeometry();
    update();
}

void ShotWindow::updateSelectionDrag(QPointF imagePoint)
{
    m_startupHoverImagePoint = clampImagePoint(imagePoint);
    m_startupHoverValid = true;
    m_selection = aspectLockedSelection(
        m_selectionBeforeDrag,
        m_selectionDrag,
        adjustedSelectionRect(m_selectionBeforeDrag, m_selectionDrag,
                              m_dragStart, m_startupHoverImagePoint, m_frozenFrame.size()));
    refreshAdjustedSelection();
}

bool ShotWindow::handleSelectionAdjustmentKey(QKeyEvent *event)
{
    if (!canAdjustSelection() || !isSelectionCursorNudgeKey(event->key())
        || event->modifiers().testFlag(Qt::ControlModifier)
        || event->modifiers().testFlag(Qt::AltModifier)
        || event->modifiers().testFlag(Qt::MetaModifier)) {
        return false;
    }

    // 1. 【截图】【选区微调】沿用当前拖动目标，未拖动时按悬停边框确定目标
    const bool dragging = m_dragging && m_selectionDrag != SelectionDrag::None;
    QPointF pointer = m_startupHoverValid
        ? m_startupHoverImagePoint
        : clampImagePoint(selectingPointerImagePoint(mapFromGlobal(QCursor::pos())));
    SelectionDrag handle = dragging ? m_selectionDrag : selectionDragAt(pointer);
    if (handle == SelectionDrag::None) {
        handle = SelectionDrag::Move;
    }
    const QRectF before = normalizedSelection();
    pointer = selectionAdjustmentPoint(before, handle, pointer);

    // 2. 【截图】【选区微调】以图像像素为单位调整，共用鼠标拖动的边界规则
    const QPoint delta = selectionCursorNudgeDelta(event->key(),
                                                   event->modifiers().testFlag(Qt::ShiftModifier));
    QPointF target = pointer + delta;
    m_selection = aspectLockedSelection(
        before, handle, adjustedSelectionRect(before, handle, pointer, target, m_frozenFrame.size()));
    if (handle == SelectionDrag::Move) {
        target = pointer + m_selection.topLeft() - before.topLeft();
    } else {
        target = selectionAdjustmentPoint(m_selection, handle, target);
    }

    // 3. 【截图】【选区微调】无法移动系统指针时继续使用可见的软件指针
    m_startupHoverImagePoint = clampImagePoint(target);
    m_startupHoverValid = true;
    m_selectionKeyboardAdjusting = !dragging;
    if (dragging) {
        // 4. 【截图】【选区微调】以调整后的状态继续拖动，避免边界外偏移阻塞反向微调
        m_selectionBeforeDrag = m_selection;
        m_dragStart = m_startupHoverImagePoint;
    }
    detachSelectionPointerIfWarpFailed(imageToWidget(m_startupHoverImagePoint).toPoint());
    refreshAdjustedSelection();
    event->accept();
    return true;
}

/// @brief 开始创建新的截图选区
/// @param imagePoint 起点的图像坐标
/// @return 无返回值
void ShotWindow::beginSelection(QPointF imagePoint)
{
    // 1. 【截图】【选区创建】初始化选区并清理旧交互状态
    m_selectionKeyboardAdjusting = false;
    m_dragging = true;
    m_fullscreenAnnotation = false;
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_actionToolbarUserPlaced = false;
    m_selectionDrag = SelectionDrag::None;
    m_controls.m_selectionBeforeFullscreenAnnotation.reset();
    m_selectionStart = imagePoint;
    m_selection = QRectF(imagePoint, imagePoint);
    // 2. 【截图】【选区创建】隐藏旧选区相关的编辑控件
    if (m_textEditor) {
        m_textEditor->hide();
        m_textEditor->clear();
        updateLayerShellForIme();
    }
    if (m_controls.m_openWithPanel) {
        m_controls.m_openWithPanel->hide();
    }
    if (m_controls.m_extensionPanel) {
        m_controls.m_extensionPanel->hide();
    }
    if (m_controls.m_annotationPropertyPanel) {
        m_controls.m_annotationPropertyPanel->hide();
    }
    if (m_controls.m_propertyColorDialogPanel) {
        m_controls.m_propertyColorDialogPanel->hide();
    }
    if (m_controls.m_propertyFontPanel) {
        m_controls.m_propertyFontPanel->hide();
    }
    if (m_selectionSizePanel) {
        m_selectionSizePanel->hide();
    }
    setFullscreenActionButtonsVisible(false);
    // 3. 【截图】【选区创建】清理标注和撤销记录
    m_annotations.clear();
    m_annotationHistory.clear();
    m_draft.reset();
    m_laserStrokes.clear();
    m_laserDraft.reset();
    setSelectedAnnotations({});
    m_nextNumber = 1;
    m_nextAnnotationId = 1;
    revealSelectionInfo();
    update();
}

/// @brief 保存最近使用的屏幕选区几何
/// @return 无返回值
void ShotWindow::recordSelectionHistory()
{
    // 本地图像标注模式的选区是文件内坐标，与屏幕选区历史不同域，不记录。
    if (m_imageNavigationEnabled) {
        return;
    }
    const QRect globalRect = selectionGlobalRect();
    if (globalRect.isEmpty()) {
        return;
    }
    markshot::rememberSelection(globalRect);
    // 使会话内缓存失效，下次浏览时能看到刚写入的记录
    m_selectionHistoryLoaded = false;
    m_selectionHistoryIndex = -1;
}

/// @brief 确认当前选区并进入编辑状态，显示标注与动作工具栏
/// @param widgetPoint 用于刷新界面光标的窗口坐标
/// @return 无返回值
void ShotWindow::enterSelectionEditing(QPointF widgetPoint)
{
    m_mode = Mode::Editing;
    attachSelectionPointer();
    m_fullscreenAnnotation = false;
    m_toolbarUserPlaced = false;
    m_actionToolbarUserPlaced = false;
    setTool(defaultEditingTool());
    setFullscreenActionButtonsVisible(false);
    m_controls.m_toolbar->show();
    m_controls.m_actionToolbar->show();
    emit selectionActivated(this);
    revealSelectionInfo();
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    scheduleUiCursorRefresh(widgetPoint);
    update();
}

/// @brief 判断按键是否用于浏览选区历史
/// @param event 键盘事件
/// @return 逗号或句号且当前允许替换选区时返回 true
bool ShotWindow::isSelectionHistoryKey(const QKeyEvent *event) const
{
    if ((event->key() != Qt::Key_Comma && event->key() != Qt::Key_Period) || event->isAutoRepeat()
        || m_dragging || m_imageNavigationEnabled) {
        return false;
    }
    // 1. 【截图】【选区历史】选择阶段沿用原有条件，启动工具（取色、录制等）占用时不处理
    if (m_mode == Mode::Selecting) {
        return m_startupTool == StartupTool::None;
    }
    // 2. 【截图】【选区历史】编辑阶段只在尚未标注时切换，避免丢弃已画内容
    return m_mode == Mode::Editing && !m_fullscreenAnnotation && m_annotations.isEmpty()
        && !m_draft.has_value() && !(m_textEditor && m_textEditor->isVisible())
        && (event->modifiers() & ~Qt::KeypadModifier) == Qt::NoModifier;
}

/// @brief 沿历史恢复之前使用的屏幕选区
/// @param step 正值读取更早的记录，负值读取更新的记录
/// @return 无返回值
void ShotWindow::applySelectionHistoryStep(int step)
{
    if (!m_selectionHistoryLoaded) {
        m_selectionHistory = markshot::readSelectionHistory();
        m_selectionHistoryLoaded = true;
        m_selectionHistoryIndex = -1;
    }
    if (m_selectionHistory.isEmpty()) {
        showToast(MS_TR("No selection history"), 1400);
        return;
    }

    // step=+1 回到更早的记录，step=-1 前进到更新的记录。历史存全局逻辑
    // 坐标，需换算回当前帧的图像坐标；不落在当前帧内的记录跳过继续找。
    int index = m_selectionHistoryIndex;
    while (true) {
        index += step;
        if (index < 0 || index >= m_selectionHistory.size()) {
            return;
        }
        const QRect imageRect = markshot::capture::imageRectFromGeometry(
            m_selectionHistory.at(index), m_sourceGeometry, m_frozenFrame.size());
        if (imageRect.width() < kMinSelectionSize || imageRect.height() < kMinSelectionSize) {
            continue;
        }
        m_selectionHistoryIndex = index;
        m_selection = QRectF(imageRect);
        m_dragging = false;
        m_selectionDrag = SelectionDrag::None;
        m_hoveredWindowRect.reset();
        // 1. 【截图】【选区历史】恢复后与手动框选一样进入编辑，点击不会再被当作重新框选或窗口识别
        if (m_mode == Mode::Selecting) {
            enterSelectionEditing(mapFromGlobal(QCursor::pos()));
        } else {
            refreshAdjustedSelection();
        }
        return;
    }
}
