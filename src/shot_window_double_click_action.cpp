#include "shot_window_module.h"

using namespace markshot::shot;

/**
 * 判断当前双击是否应执行配置的快捷动作。
 * @param imagePoint 双击点在冻结图像坐标系中的位置。
 * @return 满足触发条件返回 true，否则返回 false。
 */
bool ShotWindow::canRunDoubleClickAction(QPointF imagePoint) const
{
    // 1. 配置为不执行动作时直接放弃
    if (m_doubleClickAction == markshot::CaptureDoubleClickAction::None) {
        return false;
    }

    // 2. 仅在编辑阶段生效，选区尚未确定或启动工具占用时不接管双击
    if (m_mode != Mode::Editing || m_startupTool != StartupTool::None) {
        return false;
    }

    // 3. 没有可导出的选区时没有动作可执行
    if (!hasUsableSelection()) {
        return false;
    }

    // 4. 文字编辑框打开时双击属于文本操作，不触发手势
    if (m_textEditor && m_textEditor->isVisible()) {
        return false;
    }

    // 5. 双击点必须落在选区内部，避免选区外误触
    return normalizedSelection().contains(imagePoint);
}

/**
 * 丢弃双击第一击在当前工具下留下的草稿。
 * 双击的首次按下已经走过 mousePressEvent，若不回滚，紧随其后的
 * mouseReleaseEvent 会提交一个误画的小点。
 * @return 无返回值
 */
void ShotWindow::discardDraftForDoubleClick()
{
    if (m_tool == Tool::Laser) {
        m_laserDraft.reset();
    } else {
        m_draft.reset();
    }

    m_dragging = false;
    m_annotationDrag = SelectionDrag::None;
    m_annotationHistoryCaptured = false;
}

/**
 * 执行配置的选区双击动作。
 * 调用方需先用 canRunDoubleClickAction 确认触发条件。
 * @return 已执行动作返回 true，配置为无动作时返回 false。
 */
bool ShotWindow::runConfiguredDoubleClickAction()
{
    // 1. 先回滚第一击产生的草稿，保证导出的图像不含误画笔迹
    discardDraftForDoubleClick();
    update();

    // 2. 按配置分发到既有动作入口，各入口自行处理关闭窗口
    switch (m_doubleClickAction) {
    case markshot::CaptureDoubleClickAction::None:
        return false;
    case markshot::CaptureDoubleClickAction::Copy:
        copySelection();
        return true;
    case markshot::CaptureDoubleClickAction::Save:
        saveSelection();
        return true;
    case markshot::CaptureDoubleClickAction::SaveAs:
        saveSelectionAs();
        return true;
    case markshot::CaptureDoubleClickAction::Pin:
        pinSelection();
        return true;
    case markshot::CaptureDoubleClickAction::Cancel:
        commitTextEditor();
        emit sessionCancelRequested();
        close();
        return true;
    }

    return false;
}
