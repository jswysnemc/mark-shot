#pragma once

#include <QString>

#include <optional>

namespace markshot {

// 选区快捷手势（双击、中键单击）共用的动作集合。数值同时作为设置界面下拉项的稳定标识。
// None 表示手势不执行配置动作，保留该手势的内置行为。
enum class CaptureClickAction {
    None,
    Copy,
    Save,
    SaveAs,
    Pin,
    Cancel,
};

/// @brief 将配置文本解析为选区快捷手势动作。
/// @param value 配置中的字符串值，忽略大小写和分隔符。
/// @return 解析成功时返回动作，否则返回空值。
std::optional<CaptureClickAction> captureClickActionFromText(QString value);

/// @brief 返回选区快捷手势动作的规范配置名称。
/// @param action 手势动作。
/// @return 可写入配置文件的字符串名称，None 对应 "none"。
QString captureClickActionName(CaptureClickAction action);

}  // namespace markshot
