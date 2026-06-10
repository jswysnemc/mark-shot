#pragma once

#include <QJsonObject>

namespace markshot {

/// @brief 返回 KDE KWin ScreenShot2 捕获开关的内置默认值。
/// @return 默认启用 KWin ScreenShot2 捕获时返回 true。
bool defaultKdeKWinScreenshotEnabled();

/// @brief 从应用配置根对象解析 KDE KWin ScreenShot2 捕获开关。
/// @param root 应用配置根对象。
/// @return 配置的 KWin ScreenShot2 捕获开关，缺失或非法时返回默认值。
bool kdeKWinScreenshotEnabledFromConfigRoot(const QJsonObject &root);

/// @brief 从当前应用配置文件读取 KDE KWin ScreenShot2 捕获开关。
/// @return 配置的 KWin ScreenShot2 捕获开关。
bool configuredKdeKWinScreenshotEnabled();

}  // namespace markshot
