#include "shot_window_module.h"

#include "debug_log.h"

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

/**
 * 【截图会话】【工具转移】判断当前启动工具能否跟随指针跨屏。
 * 扫码、取色在未拖动时可转移；尺子只在按下快捷键的瞬间按指针屏幕转移，随后进入整屏标注；
 * GIF 和视频录制仅在已确认配置、等待选区时可转移。
 * @return 允许转移时返回 true，无参数。
 */
bool ShotWindow::startupToolCanFollowPointer() const
{
    if (m_mode != Mode::Selecting || m_dragging) {
        return false;
    }
    if (recordingModeForStartupTool(m_startupTool).has_value()) {
        return m_pendingRecordingOptions.has_value();
    }
    return m_startupTool == StartupTool::CodeScanner || m_startupTool == StartupTool::ColorPicker
        || m_startupTool == StartupTool::Ruler;
}

/**
 * 【截图会话】【工具转移】把当前启动工具转给同一会话中的另一个覆盖层。
 * 录制工具必须连同已确认的录制配置一起转移，否则目标屏幕完成选区时没有配置可用。
 * @param target 接收工具的覆盖层。
 * @return 已完成转移返回 true，目标无效或当前工具不可转移时返回 false。
 */
bool ShotWindow::transferStartupToolTo(ShotWindow *target)
{
    if (!target || target == this || !startupToolCanFollowPointer()) {
        return false;
    }
    // 1. 【截图会话】【工具转移】先取出工具和录制配置，leaveStartupTool 会清空待处理配置
    const StartupTool tool = m_startupTool;
    std::optional<markshot::recording::RecordingOptions> recordingOptions;
    if (recordingModeForStartupTool(tool).has_value()) {
        recordingOptions = m_pendingRecordingOptions;
    }
    QScreen *sourceScreen = captureScreen();
    QScreen *targetScreen = target->captureScreen();
    markshot::debugLog("capture-session",
                       "【截图会话】【工具转移】tool=%d source=%s target=%s recording=%d",
                       static_cast<int>(tool),
                       sourceScreen ? sourceScreen->name().toUtf8().constData() : "(none)",
                       targetScreen ? targetScreen->name().toUtf8().constData() : "(none)",
                       recordingOptions.has_value() ? 1 : 0);
    // 2. 【截图会话】【工具转移】原覆盖层退出工具，避免两块屏幕同时处于同一工具
    applyRoutedStartupTool(StartupTool::None);
    // 3. 【截图会话】【工具转移】录制工具在目标覆盖层重新进入区域选区，其他工具直接切换
    if (recordingOptions.has_value()) {
        target->m_applyingRoutedStartupTool = true;
        target->beginRegionRecordingSelection(std::move(*recordingOptions));
        target->m_applyingRoutedStartupTool = false;
    } else {
        target->applyRoutedStartupTool(tool);
    }
    return true;
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
