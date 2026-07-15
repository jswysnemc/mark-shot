#pragma once

#include <QJsonObject>

namespace markshot {

/// @brief 返回截屏时是否隐藏 mark-shot 自身窗口的内置默认值。
/// @return 默认隐藏时返回 true。
bool defaultHideOwnWindowsDuringCapture();

/// @brief 从应用配置根对象解析截屏时是否隐藏 mark-shot 自身窗口。
/// @param root 应用配置根对象。
/// @return 配置的策略，缺失或非法时返回默认值。
bool hideOwnWindowsDuringCaptureFromConfigRoot(const QJsonObject &root);

/// @brief 从当前应用配置文件读取截屏时是否隐藏 mark-shot 自身窗口。
/// @return 配置的策略。
bool configuredHideOwnWindowsDuringCapture();

/// @brief 判断 KWin ScreenShot2 是否支持当前自身窗口策略。
/// @param hideOwnWindows 是否隐藏 mark-shot 自身窗口。
/// @return KWin ScreenShot2 可以正确执行该策略时返回 true。
bool kwinScreenShotSupportsOwnWindowPolicy(bool hideOwnWindows);

}  // namespace markshot
