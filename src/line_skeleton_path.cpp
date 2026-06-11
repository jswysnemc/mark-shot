#include "line_skeleton_path.h"

#include <QLineF>

#include <algorithm>
#include <cmath>
#include <limits>

namespace markshot::shot {
namespace {

constexpr qreal kMinimumLineSkeletonLength = 0.001;
constexpr qreal kDistinctLineSkeletonPointDistance = 0.01;

/**
 * 计算二维向量长度。
 * @param vector 待计算的二维向量。
 * @return 向量长度。
 */
qreal lineSkeletonVectorLength(QPointF vector)
{
    return QLineF(QPointF(0.0, 0.0), vector).length();
}

/**
 * 计算两点中点。
 * @param first 第一个点。
 * @param second 第二个点。
 * @return 两点中点。
 */
QPointF lineSkeletonMidpoint(QPointF first, QPointF second)
{
    return (first + second) / 2.0;
}

/**
 * 向列表追加非重复点。
 * @param points 待写入的点列表。
 * @param point 候选点。
 * @return 无返回值。
 */
void appendDistinctLineSkeletonPoint(QVector<QPointF> *points, QPointF point)
{
    if (!points || (!points->isEmpty()
                    && lineSkeletonVectorLength(points->last() - point) <= kDistinctLineSkeletonPointDistance)) {
        return;
    }
    points->append(point);
}

/**
 * 计算点在线段上的投影点。
 * @param point 待投影的点。
 * @param start 线段起点。
 * @param end 线段终点。
 * @return 投影到线段范围内的点。
 */
QPointF projectedPointOnSegment(QPointF point, QPointF start, QPointF end)
{
    const QPointF segment = end - start;
    const qreal segmentLengthSquared = QPointF::dotProduct(segment, segment);
    if (segmentLengthSquared <= kMinimumLineSkeletonLength) {
        return start;
    }

    const qreal ratio = std::clamp(QPointF::dotProduct(point - start, segment) / segmentLengthSquared, 0.0, 1.0);
    return start + segment * ratio;
}

/**
 * 计算点到线段的距离。
 * @param point 待检测的点。
 * @param start 线段起点。
 * @param end 线段终点。
 * @param projected 输出投影点。
 * @return 点到线段的最短距离。
 */
qreal distanceToSegment(QPointF point, QPointF start, QPointF end, QPointF *projected)
{
    const QPointF projection = projectedPointOnSegment(point, start, end);
    if (projected) {
        *projected = projection;
    }
    return lineSkeletonVectorLength(point - projection);
}

/**
 * 将采样线段索引映射为标注点插入索引。
 * @param points 原始标注点列表。
 * @param routePoints 按绘制顺序排列的点列表。
 * @param sampleSegmentIndex 采样线段索引。
 * @param sampleSegmentCount 采样线段总数。
 * @return 标注点插入索引。
 */
int insertionIndexFromSampleSegment(const QVector<QPointF> &points,
                                    const QVector<QPointF> &routePoints,
                                    int sampleSegmentIndex,
                                    int sampleSegmentCount)
{
    if (routePoints.size() < 2 || sampleSegmentCount <= 0) {
        return points.size();
    }

    const qreal ratio = static_cast<qreal>(sampleSegmentIndex) / static_cast<qreal>(sampleSegmentCount);
    const int routeSegmentCount = routePoints.size() - 1;
    const int routeSegmentIndex = std::clamp(static_cast<int>(std::floor(ratio * routeSegmentCount)),
                                             0,
                                             routeSegmentCount - 1);
    const int nextRouteIndex = routeSegmentIndex + 1;
    if (nextRouteIndex >= routePoints.size() - 1) {
        return points.size();
    }
    return nextRouteIndex + 1;
}

} // namespace

QVector<QPointF> lineSkeletonRoutePoints(const QVector<QPointF> &points)
{
    QVector<QPointF> routePoints;
    if (points.isEmpty()) {
        return routePoints;
    }
    if (points.size() == 1) {
        routePoints.append(points.first());
        return routePoints;
    }

    routePoints.reserve(points.size());
    routePoints.append(points.first());
    for (int i = kLineSkeletonFirstPointIndex; i < points.size(); ++i) {
        routePoints.append(points.at(i));
    }
    routePoints.append(points.at(1));
    return routePoints;
}

QPainterPath lineSkeletonPath(const QVector<QPointF> &points)
{
    const QVector<QPointF> routePoints = lineSkeletonRoutePoints(points);
    QPainterPath path;
    if (routePoints.isEmpty()) {
        return path;
    }

    path.moveTo(routePoints.first());
    if (routePoints.size() == 1) {
        return path;
    }
    if (routePoints.size() == 2) {
        path.lineTo(routePoints.last());
        return path;
    }
    if (routePoints.size() == 3) {
        path.quadTo(routePoints.at(1), routePoints.last());
        return path;
    }

    // 1. 多骨架点使用连续二次曲线，使每个骨架点都能影响局部弧度
    path.lineTo(lineSkeletonMidpoint(routePoints.at(0), routePoints.at(1)));
    for (int i = 1; i < routePoints.size() - 1; ++i) {
        path.quadTo(routePoints.at(i), lineSkeletonMidpoint(routePoints.at(i), routePoints.at(i + 1)));
    }
    // 2. 末端保持精确落在标注终点，避免箭头尖端偏移
    path.lineTo(routePoints.last());
    return path;
}

QVector<QPointF> sampledLineSkeletonCenterLine(const QVector<QPointF> &points, int sampleCount)
{
    const QVector<QPointF> routePoints = lineSkeletonRoutePoints(points);
    QVector<QPointF> samples;
    if (routePoints.size() < 2) {
        return routePoints;
    }
    if (routePoints.size() == 2) {
        return routePoints;
    }

    const QPainterPath path = lineSkeletonPath(points);
    const qreal pathLength = path.length();
    if (pathLength <= kMinimumLineSkeletonLength) {
        return routePoints;
    }

    const int boundedSampleCount = std::max(2, sampleCount);
    samples.reserve(boundedSampleCount + 1);
    for (int i = 0; i <= boundedSampleCount; ++i) {
        const qreal distance = pathLength * static_cast<qreal>(i) / static_cast<qreal>(boundedSampleCount);
        appendDistinctLineSkeletonPoint(&samples, path.pointAtPercent(path.percentAtLength(distance)));
    }
    return samples;
}

bool canRemoveLineSkeletonPoint(const QVector<QPointF> &points, int pointIndex)
{
    return pointIndex >= kLineSkeletonFirstPointIndex && pointIndex < points.size();
}

std::optional<LineSkeletonInsertion> lineSkeletonInsertionAt(const QVector<QPointF> &points,
                                                            QPointF point,
                                                            qreal tolerance)
{
    const QVector<QPointF> routePoints = lineSkeletonRoutePoints(points);
    if (routePoints.size() < 2 || tolerance < 0.0) {
        return std::nullopt;
    }

    const QVector<QPointF> samples = sampledLineSkeletonCenterLine(points);
    if (samples.size() < 2) {
        return std::nullopt;
    }

    qreal bestDistance = std::numeric_limits<qreal>::max();
    QPointF bestPoint;
    int bestSegmentIndex = -1;
    for (int i = 1; i < samples.size(); ++i) {
        QPointF projected;
        const qreal distance = distanceToSegment(point, samples.at(i - 1), samples.at(i), &projected);
        if (distance < bestDistance) {
            bestDistance = distance;
            bestPoint = projected;
            bestSegmentIndex = i - 1;
        }
    }

    if (bestSegmentIndex < 0 || bestDistance > tolerance) {
        return std::nullopt;
    }

    return LineSkeletonInsertion{
        insertionIndexFromSampleSegment(points, routePoints, bestSegmentIndex, samples.size() - 1),
        bestPoint,
        bestDistance,
    };
}

bool lineSkeletonContainsPoint(const QVector<QPointF> &points, QPointF point, qreal tolerance)
{
    return lineSkeletonInsertionAt(points, point, tolerance).has_value();
}

} // namespace markshot::shot
