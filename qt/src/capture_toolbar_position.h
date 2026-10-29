#pragma once

#include <QJsonObject>
#include <QString>

#include <optional>

namespace markshot {

// 选区编辑阶段两条工具栏的摆放方式。数值同时作为设置界面下拉项的稳定标识。
enum class CaptureToolbarPosition {
    FollowSelection,
    ScreenBottom,
    ScreenRight,
};

/// @brief 返回工具栏摆放方式的内置默认值。
/// @return 默认摆放方式，保持跟随选区避让的旧行为。
CaptureToolbarPosition defaultCaptureToolbarPosition();

/// @brief 将配置文本解析为工具栏摆放方式。
/// @param value 配置中的字符串值。
/// @return 解析成功时返回摆放方式，否则返回空值。
std::optional<CaptureToolbarPosition> captureToolbarPositionFromText(QString value);

/// @brief 从应用配置根对象解析工具栏摆放方式。
/// @param root 应用配置根对象。
/// @return 配置的摆放方式，缺失或非法时返回默认值。
CaptureToolbarPosition captureToolbarPositionFromConfigRoot(const QJsonObject &root);

/// @brief 返回工具栏摆放方式的规范配置名称。
/// @param position 摆放方式。
/// @return 可写入配置文件的字符串名称。
QString captureToolbarPositionName(CaptureToolbarPosition position);

}  // namespace markshot
