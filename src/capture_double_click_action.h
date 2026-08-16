#pragma once

#include <QJsonObject>
#include <QString>

#include <optional>

namespace markshot {

// 选区内双击触发的动作。数值同时作为设置界面下拉项的稳定标识。
enum class CaptureDoubleClickAction {
    None,
    Copy,
    Save,
    SaveAs,
    Pin,
    Cancel,
};

/// @brief 返回选区双击动作的内置默认值。
/// @return 默认双击动作。
CaptureDoubleClickAction defaultCaptureDoubleClickAction();

/// @brief 将配置文本解析为选区双击动作。
/// @param value 配置中的字符串值。
/// @return 解析成功时返回双击动作，否则返回空值。
std::optional<CaptureDoubleClickAction> captureDoubleClickActionFromText(QString value);

/// @brief 从应用配置根对象解析选区双击动作。
/// @param root 应用配置根对象。
/// @return 配置的双击动作，缺失或非法时返回默认值。
CaptureDoubleClickAction captureDoubleClickActionFromConfigRoot(const QJsonObject &root);

/// @brief 从当前应用配置文件读取选区双击动作。
/// @return 配置的双击动作。
CaptureDoubleClickAction configuredCaptureDoubleClickAction();

/// @brief 返回选区双击动作的规范配置名称。
/// @param action 双击动作。
/// @return 可写入配置文件的字符串名称。
QString captureDoubleClickActionName(CaptureDoubleClickAction action);

}  // namespace markshot
