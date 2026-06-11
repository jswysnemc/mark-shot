#pragma once

#include <QPainterPath>
#include <QPointF>
#include <QRectF>
#include <QVector>

#include <optional>

namespace markshot::shot {

inline constexpr int kLineSkeletonFirstPointIndex = 2;

struct LineSkeletonInsertion {
    int pointIndex = -1;
    QPointF point;
    qreal distance = 0.0;
};

/**
 * 构造直线类工具的骨架路由点。
 * @param points 标注点列表，0 为起点，1 为终点，2 及以后为骨架点。
 * @return 按绘制顺序排列的点列表。
 */
QVector<QPointF> lineSkeletonRoutePoints(const QVector<QPointF> &points);

/**
 * 构造直线类工具的平滑绘制路径。
 * @param points 标注点列表，0 为起点，1 为终点，2 及以后为骨架点。
 * @return 可直接绘制的路径。
 */
QPainterPath lineSkeletonPath(const QVector<QPointF> &points);

/**
 * 采样直线类工具的中心线。
 * @param points 标注点列表，0 为起点，1 为终点，2 及以后为骨架点。
 * @param sampleCount 曲线采样数量。
 * @return 按路径方向排列的中心线点列表。
 */
QVector<QPointF> sampledLineSkeletonCenterLine(const QVector<QPointF> &points, int sampleCount = 64);

/**
 * 判断指定骨架点是否允许删除。
 * @param points 标注点列表，0 为起点，1 为终点，2 及以后为骨架点。
 * @param pointIndex 待删除的标注点索引。
 * @return 可以删除时返回 true，否则返回 false。
 */
bool canRemoveLineSkeletonPoint(const QVector<QPointF> &points, int pointIndex);

/**
 * 查找点击位置附近可插入骨架点的位置。
 * @param points 标注点列表，0 为起点，1 为终点，2 及以后为骨架点。
 * @param point 点击位置。
 * @param tolerance 命中容差。
 * @return 命中路径时返回插入信息，否则返回 std::nullopt。
 */
std::optional<LineSkeletonInsertion> lineSkeletonInsertionAt(const QVector<QPointF> &points,
                                                            QPointF point,
                                                            qreal tolerance);

/**
 * 判断点是否位于直线类工具路径附近。
 * @param points 标注点列表，0 为起点，1 为终点，2 及以后为骨架点。
 * @param point 待检测位置。
 * @param tolerance 命中容差。
 * @return 位于路径附近时返回 true，否则返回 false。
 */
bool lineSkeletonContainsPoint(const QVector<QPointF> &points, QPointF point, qreal tolerance);

} // namespace markshot::shot
