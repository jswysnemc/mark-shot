#pragma once

#include "windows_tray_controller.h"

namespace markshot {

/**
 * 【托盘】【全局快捷键】判断两份托盘配置中的全局快捷键是否需要重新注册。
 * @param current 正在生效的托盘配置。
 * @param next 设置保存后重新读取的托盘配置。
 * @return 启用状态或任一快捷键组合不同返回 true，仅自动启动等其他字段变化返回 false。
 */
bool trayHotkeysDiffer(const WindowsTrayController::Config &current, const WindowsTrayController::Config &next);

}  // namespace markshot
