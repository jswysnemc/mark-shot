#pragma once

#include <QMargins>
#include <QPoint>
#include <QRect>
#include <QSize>
#include <QVector>

namespace markshot::shot {

struct PinnedLayerShellPlacement {
    QRect logicalGeometry;
    QRect visibleGeometry;
    QPoint contentOffset;
    QMargins margins;
    QSize desiredSize;
};

/// @brief 计算贴图窗口在 layer-shell 屏幕坐标系中的边距。
/// @param geometry 贴图窗口全局逻辑几何。
/// @param screenGeometry 目标屏幕全局逻辑几何。
/// @return 可包含负数的 layer-shell 边距。
QMargins pinnedLayerShellMargins(QRect geometry, QRect screenGeometry);

/// @brief 选择与贴图窗口最匹配的屏幕索引。
/// @param geometry 贴图窗口全局逻辑几何。
/// @param screenGeometries 候选屏幕全局逻辑几何列表。
/// @return 匹配屏幕的索引,没有候选屏幕时返回 -1。
int bestPinnedLayerShellScreenIndex(QRect geometry, const QVector<QRect> &screenGeometries);

/// @brief 将贴图逻辑几何转换为 layer-shell 可见 surface 几何。
/// @param geometry 贴图窗口全局逻辑几何。
/// @param screenGeometry 目标屏幕全局逻辑几何。
/// @param minimumVisibleSize 至少保留在屏幕内的可见尺寸。
/// @return layer-shell 放置结果。
PinnedLayerShellPlacement pinnedLayerShellPlacement(QRect geometry,
                                                    QRect screenGeometry,
                                                    QSize minimumVisibleSize);

}  // namespace markshot::shot
