#pragma once

#include <QPoint>
#include <optional>

namespace markshot::wayland {

/**
 * 【Wayland】【指针查询】查询合成器提供的全局逻辑指针位置。
 * @return GNOME 扩展返回的位置；当前会话不支持或接口不可用时返回空值。
 */
std::optional<QPoint> compositorPointerPosition();

}  // namespace markshot::wayland
