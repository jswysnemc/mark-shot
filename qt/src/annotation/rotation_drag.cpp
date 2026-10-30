#include "annotation/rotation_drag.h"

#include <QLineF>
#include <QtMath>

#include <cmath>

namespace markshot::annotation {

namespace {

// 指针离轴心不足该距离时方向不可靠，图像像素
constexpr qreal kMinimumRotationRadius = 0.1;
// Shift 吸附的角度间隔
constexpr qreal kRotationSnapStep = 45.0;

}  // namespace

std::optional<qreal> rotationDragDegrees(QPointF center, QPointF start, QPointF current)
{
    const QPointF startVector = start - center;
    const QPointF currentVector = current - center;
    if (QLineF(QPointF(0, 0), startVector).length() <= kMinimumRotationRadius
        || QLineF(QPointF(0, 0), currentVector).length() <= kMinimumRotationRadius) {
        return std::nullopt;
    }
    const qreal startAngle = std::atan2(startVector.y(), startVector.x());
    const qreal currentAngle = std::atan2(currentVector.y(), currentVector.x());
    return (currentAngle - startAngle) * 180.0 / M_PI;
}

qreal snappedRotationDegrees(qreal degrees)
{
    // 1. 先取最近的 45 度倍数，再折回 -180 到 180，结果仍是 45 度倍数
    return std::remainder(std::round(degrees / kRotationSnapStep) * kRotationSnapStep, 360.0);
}

}  // namespace markshot::annotation
