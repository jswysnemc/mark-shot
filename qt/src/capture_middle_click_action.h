#pragma once

#include "capture_click_action.h"

#include <QJsonObject>
#include <QString>

#include <optional>

namespace markshot {

/// @brief 返回选区中键单击动作的内置默认值。
/// @return CaptureClickAction::None，即保留中键原有的平移和全屏标注切换行为。
CaptureClickAction defaultCaptureMiddleClickAction();

/// @brief 将配置文本解析为选区中键单击动作。
/// @param value 配置中的字符串值；default、zoom、pan、none 等表示保留内置行为。
/// @return 解析成功时返回动作，否则返回空值。
std::optional<CaptureClickAction> captureMiddleClickActionFromText(QString value);

/// @brief 从应用配置根对象解析选区中键单击动作。
/// @param root 应用配置根对象。
/// @return 配置的中键单击动作，缺失或非法时返回默认值。
CaptureClickAction captureMiddleClickActionFromConfigRoot(const QJsonObject &root);

/// @brief 返回选区中键单击动作的规范配置名称。
/// @param action 中键单击动作。
/// @return 可写入配置文件的字符串名称，内置行为对应 "default"。
QString captureMiddleClickActionName(CaptureClickAction action);

}  // namespace markshot
