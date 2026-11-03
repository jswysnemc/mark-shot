#include "windows_tray_controller.h"

#include "debug_log.h"
#include "windows_tray_hotkey_diff.h"

#include <utility>

namespace markshot {

void WindowsTrayController::reloadHotkeys()
{
    // 1. 重新读取配置，快捷键未变化时保持现有注册，避免 portal 重复弹出授权
    Config next = readConfig();
    if (!trayHotkeysDiffer(m_config, next)) {
        return;
    }
    markshot::debugLog("tray",
                       "【托盘】【全局快捷键】settings saved, re-registering hotkeys enabled=%d",
                       next.hotkeysEnabled ? 1 : 0);

    // 2. 先注销旧组合，再按新配置注册；关闭全局快捷键时只注销
    unregisterHotkeys();
    m_errorString.clear();
    m_config = std::move(next);
    if (m_config.hotkeysEnabled) {
        registerHotkeys();
    }
}

}  // namespace markshot
