#include "pinned_window/pinned_layer_shell_geometry.h"

#include <QtGlobal>

#include <algorithm>
#include <limits>

namespace {

/// @brief 计算矩形交集面积。
/// @param rect 矩形。
/// @return 面积,无效矩形返回 0。
qint64 rectArea(const QRect &rect)
{
    if (!rect.isValid() || rect.isEmpty()) {
        return 0;
    }
    return static_cast<qint64>(rect.width()) * static_cast<qint64>(rect.height());
}

/// @brief 计算两个点的距离平方。
/// @param a 第一个点。
/// @param b 第二个点。
/// @return 距离平方。
qint64 distanceSquared(const QPoint &a, const QPoint &b)
{
    const qint64 dx = static_cast<qint64>(a.x()) - static_cast<qint64>(b.x());
    const qint64 dy = static_cast<qint64>(a.y()) - static_cast<qint64>(b.y());
    return dx * dx + dy * dy;
}

}  // namespace

namespace markshot::shot {

QMargins pinnedLayerShellMargins(QRect geometry, QRect screenGeometry)
{
    // 1. 保留负坐标,允许贴图嵌入屏幕左边缘和上边缘
    return QMargins(geometry.left() - screenGeometry.left(),
                    geometry.top() - screenGeometry.top(),
                    0,
                    0);
}

int bestPinnedLayerShellScreenIndex(QRect geometry, const QVector<QRect> &screenGeometries)
{
    if (screenGeometries.isEmpty()) {
        return -1;
    }

    int bestIndex = -1;
    qint64 bestIntersectionArea = -1;
    qint64 bestDistance = std::numeric_limits<qint64>::max();
    const QPoint geometryCenter = geometry.center();

    // 1. 优先选择与贴图可见部分相交面积最大的屏幕
    for (int i = 0; i < screenGeometries.size(); ++i) {
        const QRect screenGeometry = screenGeometries.at(i);
        if (!screenGeometry.isValid() || screenGeometry.isEmpty()) {
            continue;
        }

        const qint64 intersectionArea = rectArea(geometry.intersected(screenGeometry));
        const qint64 distance = distanceSquared(geometryCenter, screenGeometry.center());
        if (intersectionArea > bestIntersectionArea
            || (intersectionArea == bestIntersectionArea && distance < bestDistance)) {
            bestIndex = i;
            bestIntersectionArea = intersectionArea;
            bestDistance = distance;
        }
    }

    return bestIndex;
}

PinnedLayerShellPlacement pinnedLayerShellPlacement(QRect geometry,
                                                    QRect screenGeometry,
                                                    QSize minimumVisibleSize)
{
    PinnedLayerShellPlacement placement;
    if (!geometry.isValid() || geometry.isEmpty() || !screenGeometry.isValid() || screenGeometry.isEmpty()) {
        return placement;
    }

    // 1. 约束逻辑几何,保证贴图不会被完整拖出屏幕
    const int minVisibleWidth = std::clamp(minimumVisibleSize.width(), 1, geometry.width());
    const int minVisibleHeight = std::clamp(minimumVisibleSize.height(), 1, geometry.height());
    const int minLeft = screenGeometry.left() - geometry.width() + minVisibleWidth;
    const int maxLeft = screenGeometry.right() - minVisibleWidth + 1;
    const int minTop = screenGeometry.top() - geometry.height() + minVisibleHeight;
    const int maxTop = screenGeometry.bottom() - minVisibleHeight + 1;
    geometry.moveTopLeft(QPoint(std::clamp(geometry.left(), minLeft, maxLeft),
                                std::clamp(geometry.top(), minTop, maxTop)));

    // 2. layer-shell surface 只占用屏幕内可见区域,避免 compositor 缩放整张图片
    const QRect visibleGeometry = geometry.intersected(screenGeometry);
    placement.logicalGeometry = geometry;
    placement.visibleGeometry = visibleGeometry;
    placement.contentOffset = geometry.topLeft() - visibleGeometry.topLeft();
    placement.margins = pinnedLayerShellMargins(visibleGeometry, screenGeometry);
    placement.desiredSize = visibleGeometry.size();
    return placement;
}

}  // namespace markshot::shot
