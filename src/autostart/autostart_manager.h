#pragma once

#include <QString>

namespace markshot::autostart {

/// @brief 判断当前平台是否支持写入开机启动项。
/// @return 支持系统自启动返回 true。
bool isSupported();

/// @brief 查询 Mark Shot 当前是否已注册开机启动。
/// @return 已注册开机启动返回 true。
bool isEnabled();

/// @brief 启用或禁用 Mark Shot 开机启动。
/// @param enabled 是否启用开机启动。
/// @param error 操作失败时输出错误信息。
/// @return 操作成功返回 true。
bool setEnabled(bool enabled, QString *error = nullptr);

/// @brief 生成开机启动时使用的命令行。
/// @return 当前可执行文件路径加托盘启动参数。
QString startupCommand();

}  // namespace markshot::autostart
