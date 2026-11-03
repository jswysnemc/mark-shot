#include "windows_tray_hotkey_diff.h"

namespace markshot {

bool trayHotkeysDiffer(const WindowsTrayController::Config &current, const WindowsTrayController::Config &next)
{
    // 1. 自动启动只在下次启动时生效，不参与比较
    return current.hotkeysEnabled != next.hotkeysEnabled
        || current.captureHotkey != next.captureHotkey
        || current.fullscreenHotkey != next.fullscreenHotkey
        || current.stopRecordingHotkey != next.stopRecordingHotkey
        || current.pauseRecordingHotkey != next.pauseRecordingHotkey;
}

}  // namespace markshot
