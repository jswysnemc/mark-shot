#pragma once

#include <QMimeData>
#include <memory>

namespace markshot::clipboard {

/// @brief 读取剪贴板快照，Wayland 无焦点且不持有剪贴板时绕过旧缓存，改用有限制的 wl-paste
/// @return 独立持有的图片或文字内容，读取失败时为空内容
std::unique_ptr<QMimeData> readContent();

}
