#pragma once

#include "window_detection.h"

#include <QPoint>
#include <optional>

namespace markshot::window_selection {

/**
 * 选择指定位置最靠前的窗口矩形。
 * @param windows 候选窗口及其可选层级信息。
 * @param point 需要命中的图像坐标。
 * @return 命中窗口的矩形；没有窗口覆盖该坐标时返回空值。
 */
std::optional<QRect> topmostWindowRectAt(const QVector<WindowInfo> &windows,
                                         const QPoint &point);

} // namespace markshot::window_selection
