#include "shot_window_click_action.h"

#include "shot_window_module.h"

#include "debug_log.h"

using markshot::CaptureClickAction;
using markshot::shot::types::Mode;
using markshot::shot::types::StartupTool;

ShotWindowClickAction::ShotWindowClickAction(ShotWindow &window)
    : m_window(window)
{
}

bool ShotWindowClickAction::run(CaptureClickAction action)
{
    switch (action) {
    case CaptureClickAction::None:
        return false;
    case CaptureClickAction::Copy:
        m_window.copySelection();
        return true;
    case CaptureClickAction::Save:
        m_window.saveSelection();
        return true;
    case CaptureClickAction::SaveAs:
        m_window.saveSelectionAs();
        return true;
    case CaptureClickAction::Pin:
        m_window.pinSelection();
        return true;
    case CaptureClickAction::Cancel:
        m_window.commitTextEditor();
        emit m_window.sessionCancelRequested();
        m_window.close();
        return true;
    }

    return false;
}

bool ShotWindowClickAction::canRunMiddleClick(QPointF widgetPoint) const
{
    // 1. 保留内置行为时中键只负责平移，不登记单击
    if (m_window.m_middleClick.action() == CaptureClickAction::None) {
        return false;
    }

    // 2. 仅在框选完成后的编辑阶段生效，选区阶段中键仍切换全屏标注
    if (m_window.m_mode != Mode::Editing || m_window.m_startupTool != StartupTool::None) {
        return false;
    }

    // 3. 没有可导出的选区时没有动作可执行
    if (!m_window.hasUsableSelection()) {
        return false;
    }

    // 4. 落在工具栏等子控件上的中键不属于选区操作
    if (m_window.mouseOverUiWidget(widgetPoint)) {
        return false;
    }

    // 5. 单击点必须落在选区内部，选区外的中键仍只平移画布
    return m_window.normalizedSelection().contains(m_window.widgetToImage(widgetPoint));
}

void ShotWindowClickAction::beginMiddleClick(QPointF widgetPoint, Qt::MouseButtons buttons)
{
    // 1. 与其他按键组合的按下不是单独的中键单击
    if (buttons != Qt::MiddleButton || !canRunMiddleClick(widgetPoint)) {
        m_window.m_middleClick.cancel();
        return;
    }

    // 2. 使用平台拖动阈值区分单击和平移
    m_window.m_middleClick.press(widgetPoint, QApplication::startDragDistance());
}

bool ShotWindowClickAction::finishMiddleClick(QPointF widgetPoint)
{
    // 1. 移动超过拖动阈值或没有候选时，本次中键只是平移
    if (!m_window.m_middleClick.release(widgetPoint)) {
        return false;
    }

    // 2. 按住中键期间快捷键可能已改变阶段或选区，释放时重新确认
    if (!canRunMiddleClick(widgetPoint)) {
        return false;
    }

    const CaptureClickAction action = m_window.m_middleClick.action();
    markshot::debugLog("capture-session", "【截图】【中键单击】执行选区动作 %s",
                       qPrintable(markshot::captureMiddleClickActionName(action)));
    return run(action);
}
