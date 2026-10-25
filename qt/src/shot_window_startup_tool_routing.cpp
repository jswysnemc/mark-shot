#include "shot_window_module.h"

#include <QCursor>
#include <QGuiApplication>
#include <QScreen>

using namespace markshot::shot;

namespace {

/// 关闭时指针工具留在收到快捷键的覆盖层
inline constexpr bool FEATURE_STARTUP_TOOL_FOLLOWS_CURSOR_ENABLED = true;

}  // namespace

/** 【截图会话】【工具状态】读取当前启动工具，无参数，返回工具类型。 */
markshot::shot::types::StartupTool ShotWindow::startupTool() const
{
    return m_startupTool;
}

/** 【截图会话】【工具转移】仅允许未开始拖动的扫码、取色及测量模式跨屏，无参数，返回是否允许转移。 */
bool ShotWindow::startupToolCanFollowPointer() const
{
    return m_mode == Mode::Selecting && !m_dragging && !m_startupRulerDragging
        && (m_startupTool == StartupTool::CodeScanner || m_startupTool == StartupTool::ColorPicker
            || m_startupTool == StartupTool::Ruler);
}

/** 【截图会话】【屏幕状态】读取截图屏幕，无参数，返回指定屏幕或窗口当前屏幕。 */
QScreen *ShotWindow::captureScreen() const
{
    return m_captureScreen ? m_captureScreen.data() : screen();
}

/** 【截图会话】【工具转移】应用会话转移的 tool 工具，None 表示退出当前工具，无返回值。 */
void ShotWindow::applyRoutedStartupTool(markshot::shot::types::StartupTool tool)
{
    // 1. 【截图会话】【工具转移】会话转发期间不再根据光标二次发射，避免两个覆盖层来回切换
    m_applyingRoutedStartupTool = true;
    if (tool == markshot::shot::types::StartupTool::None) {
        if (m_startupTool != markshot::shot::types::StartupTool::None) {
            leaveStartupTool();
        }
    } else if (m_startupTool != tool) {
        setStartupTool(tool);
    }
    m_applyingRoutedStartupTool = false;
}

/** 【截图会话】【工具转移】把快捷键启用的指针工具转到实际指针屏幕，无参数和返回值。 */
void ShotWindow::routeStartupToolToCursorScreenIfEnabled()
{
    if (!FEATURE_STARTUP_TOOL_FOLLOWS_CURSOR_ENABLED || m_applyingRoutedStartupTool) {
        return;
    }
    if (!startupToolCanFollowPointer()) {
        return;
    }
    QScreen *cursorScreen = QGuiApplication::screenAt(QCursor::pos());
    QScreen *ownScreen = captureScreen();
    if (!cursorScreen || !ownScreen || cursorScreen == ownScreen) {
        return;
    }
    emit startupToolRouteRequested(this);
}

/** 【截图会话】【工具转移】空闲覆盖层收到 event 进入事件时申请现有指针工具，无返回值。 */
void ShotWindow::claimStartupToolFromPeersIfEnabled(QEvent *event)
{
    if (!FEATURE_STARTUP_TOOL_FOLLOWS_CURSOR_ENABLED || !event) {
        return;
    }
    if (event->type() != QEvent::Enter) {
        return;
    }
    if (m_mode != Mode::Selecting || m_dragging || m_startupTool != StartupTool::None) {
        return;
    }
    emit startupToolClaimRequested(this);
}
