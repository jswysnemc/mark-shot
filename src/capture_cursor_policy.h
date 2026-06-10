#pragma once

#include <QJsonObject>

namespace markshot {

/// @brief 返回捕获冻结图是否包含鼠标的内置默认值。
/// @return 默认不包含鼠标时返回 false。
bool defaultCaptureIncludeCursor();

/// @brief 从应用配置根对象解析捕获冻结图是否包含鼠标。
/// @param root 应用配置根对象。
/// @return 配置的鼠标包含策略，缺失或非法时返回默认值。
bool captureIncludeCursorFromConfigRoot(const QJsonObject &root);

/// @brief 从当前应用配置文件读取捕获冻结图是否包含鼠标。
/// @return 配置的鼠标包含策略。
bool configuredCaptureIncludeCursor();

}  // namespace markshot
