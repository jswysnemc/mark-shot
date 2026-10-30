#pragma once

#include <QPointF>

#include <optional>

namespace markshot::annotation {

/**
 * 计算拖动旋转把手时指针绕轴心转过的角度，与 QPainter::rotate 同向，屏幕上顺时针为正。
 * @param center 旋转轴心，图像像素坐标。
 * @param start 按下旋转把手时的指针位置，图像像素坐标。
 * @param current 当前指针位置，图像像素坐标。
 * @return 转过的角度；起点或当前点离轴心太近、方向不可靠时为空。
 */
std::optional<qreal> rotationDragDegrees(QPointF center, QPointF start, QPointF current);

/**
 * 把旋转角度吸附到最近的 45 度倍数，按住 Shift 拖动旋转把手时使用。
 * @param degrees 旋转角度。
 * @return 吸附后的角度，介于 -180 和 180 之间。
 */
qreal snappedRotationDegrees(qreal degrees);

}  // namespace markshot::annotation
