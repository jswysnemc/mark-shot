#include "shot_window_module.h"

using namespace markshot::shot;

namespace {

constexpr qreal kMinimumVectorLength = 0.001;
constexpr qreal kDistinctPointDistance = 0.01;

struct FletchedArrowMetrics {
    qreal headLength = 0.0;
    qreal headHalfWidth = 0.0;
    qreal bodyHalfWidth = 0.0;
};

/**
 * 计算二维向量长度。
 * @param vector 待计算的二维向量。
 * @return 向量长度。
 */
qreal vectorLength(QPointF vector)
{
    return QLineF(QPointF(0.0, 0.0), vector).length();
}

/**
 * 归一化二维向量，退化时使用备用方向。
 * @param vector 优先使用的二维向量。
 * @param fallback 优先向量退化时使用的备用向量。
 * @return 单位方向向量。
 */
QPointF normalizedVector(QPointF vector, QPointF fallback = QPointF(1.0, 0.0))
{
    const qreal length = vectorLength(vector);
    if (length > kMinimumVectorLength) {
        return QPointF(vector.x() / length, vector.y() / length);
    }

    const qreal fallbackLength = vectorLength(fallback);
    if (fallbackLength > kMinimumVectorLength) {
        return QPointF(fallback.x() / fallbackLength, fallback.y() / fallbackLength);
    }

    return QPointF(1.0, 0.0);
}

/**
 * 计算方向向量的左侧法线。
 * @param direction 已归一化或接近归一化的方向向量。
 * @return 与方向垂直的左侧法线。
 */
QPointF normalForDirection(QPointF direction)
{
    return QPointF(-direction.y(), direction.x());
}

/**
 * 追加与上一个点不重合的点。
 * @param points 待写入的点列表。
 * @param point 候选点。
 * @return 无返回值。
 */
void appendDistinctPoint(QVector<QPointF> *points, QPointF point)
{
    if (!points || (!points->isEmpty() && vectorLength(points->last() - point) <= kDistinctPointDistance)) {
        return;
    }
    points->append(point);
}

/**
 * 计算折线总长度。
 * @param points 折线点列表。
 * @return 折线总长度。
 */
qreal polylineLength(const QVector<QPointF> &points)
{
    qreal length = 0.0;
    for (int i = 1; i < points.size(); ++i) {
        length += vectorLength(points.at(i) - points.at(i - 1));
    }
    return length;
}

/**
 * 计算折线指定长度位置上的点。
 * @param points 折线点列表。
 * @param distanceFromStart 距离折线起点的路径长度。
 * @return 指定路径长度位置上的点。
 */
QPointF pointAtDistance(const QVector<QPointF> &points, qreal distanceFromStart)
{
    if (points.isEmpty()) {
        return QPointF();
    }
    if (distanceFromStart <= 0.0) {
        return points.first();
    }

    qreal walked = 0.0;
    for (int i = 1; i < points.size(); ++i) {
        const QPointF previous = points.at(i - 1);
        const QPointF current = points.at(i);
        const qreal segmentLength = vectorLength(current - previous);
        if (segmentLength <= kMinimumVectorLength) {
            continue;
        }

        if (distanceFromStart <= walked + segmentLength) {
            const qreal ratio = (distanceFromStart - walked) / segmentLength;
            return previous + (current - previous) * ratio;
        }
        walked += segmentLength;
    }

    return points.last();
}

/**
 * 计算折线指定长度位置上的切线方向。
 * @param points 折线点列表。
 * @param distanceFromStart 距离折线起点的路径长度。
 * @return 指定位置的单位切线方向。
 */
QPointF tangentAtDistance(const QVector<QPointF> &points, qreal distanceFromStart)
{
    if (points.size() < 2) {
        return QPointF(1.0, 0.0);
    }

    const qreal totalLength = polylineLength(points);
    const qreal targetDistance = std::clamp(distanceFromStart, 0.0, totalLength);
    qreal walked = 0.0;
    QPointF fallback = points.last() - points.first();

    for (int i = 1; i < points.size(); ++i) {
        const QPointF previous = points.at(i - 1);
        const QPointF current = points.at(i);
        const QPointF segment = current - previous;
        const qreal segmentLength = vectorLength(segment);
        if (segmentLength <= kMinimumVectorLength) {
            continue;
        }

        fallback = segment;
        if (targetDistance <= walked + segmentLength) {
            return normalizedVector(segment, fallback);
        }
        walked += segmentLength;
    }

    return normalizedVector(fallback);
}

/**
 * 提取折线中指定长度范围内的线段。
 * @param points 原始折线点列表。
 * @param startDistance 起始路径长度。
 * @param endDistance 结束路径长度。
 * @return 截取后的折线点列表。
 */
QVector<QPointF> centerLineSection(const QVector<QPointF> &points, qreal startDistance, qreal endDistance)
{
    QVector<QPointF> section;
    const qreal totalLength = polylineLength(points);
    if (points.isEmpty() || totalLength <= kMinimumVectorLength) {
        return section;
    }

    const qreal boundedStart = std::clamp(startDistance, 0.0, totalLength);
    const qreal boundedEnd = std::clamp(endDistance, boundedStart, totalLength);
    appendDistinctPoint(&section, pointAtDistance(points, boundedStart));

    qreal walked = 0.0;
    for (int i = 1; i < points.size(); ++i) {
        const QPointF previous = points.at(i - 1);
        const QPointF current = points.at(i);
        const qreal segmentLength = vectorLength(current - previous);
        const qreal segmentEnd = walked + segmentLength;
        if (segmentEnd > boundedStart && segmentEnd < boundedEnd) {
            appendDistinctPoint(&section, current);
        }
        walked = segmentEnd;
    }

    appendDistinctPoint(&section, pointAtDistance(points, boundedEnd));
    return section;
}

/**
 * 计算折线采样点上的法线。
 * @param points 折线点列表。
 * @param index 采样点索引。
 * @return 指定采样点的左侧单位法线。
 */
QPointF normalAtIndex(const QVector<QPointF> &points, int index)
{
    if (points.size() < 2) {
        return QPointF(0.0, 1.0);
    }

    QPointF tangent;
    if (index <= 0) {
        tangent = points.at(1) - points.first();
    } else if (index >= points.size() - 1) {
        tangent = points.last() - points.at(points.size() - 2);
    } else {
        tangent = points.at(index + 1) - points.at(index - 1);
    }
    return normalForDirection(normalizedVector(tangent));
}

/**
 * 计算燕尾箭头的头部和主体尺寸。
 * @param totalLength 箭头中心线总长度。
 * @param width 当前标注线宽。
 * @param bidirectional 是否为双向箭头。
 * @return 燕尾箭头尺寸集合。
 */
FletchedArrowMetrics fletchedArrowMetrics(qreal totalLength, qreal width, bool bidirectional)
{
    FletchedArrowMetrics metrics;
    metrics.bodyHalfWidth = width * 0.5;
    metrics.headLength = std::clamp(totalLength * 0.18, width * 5.0, width * 9.0);
    metrics.headLength = std::clamp(metrics.headLength, 12.0, 60.0);
    metrics.headLength = std::min(metrics.headLength, totalLength * (bidirectional ? 0.42 : 0.62));
    metrics.headHalfWidth = std::max(metrics.headLength * 0.28, metrics.bodyHalfWidth * 1.5);
    return metrics;
}

/**
 * 计算 KDE 开口箭头的头部长度。
 * @param totalLength 箭头中心线总长度。
 * @param width 当前标注线宽。
 * @param bidirectional 是否为双向箭头。
 * @return KDE 开口箭头头部长度。
 */
qreal kdeHeadLength(qreal totalLength, qreal width, bool bidirectional)
{
    qreal headLength = std::clamp(totalLength * 0.32, width * 2.6, width * 6.0);
    headLength = std::min(headLength, totalLength * (bidirectional ? 0.42 : 1.0));
    return std::max<qreal>(0.0, headLength);
}

/**
 * 向路径追加 KDE 开口箭头头部。
 * @param path 待写入的箭头头部路径。
 * @param tip 箭头尖端坐标。
 * @param direction 从主体指向尖端的方向。
 * @param headLength 箭头头部长度。
 * @param headHalfWidth 箭头头部半宽。
 * @return 无返回值。
 */
void appendOpenArrowHead(QPainterPath *path,
                         QPointF tip,
                         QPointF direction,
                         qreal headLength,
                         qreal headHalfWidth)
{
    if (!path || headLength <= kMinimumVectorLength) {
        return;
    }

    const QPointF unitDirection = normalizedVector(direction);
    const QPointF normal = normalForDirection(unitDirection);
    const QPointF base = tip - unitDirection * headLength;
    path->moveTo(base + normal * headHalfWidth);
    path->lineTo(tip);
    path->lineTo(base - normal * headHalfWidth);
}

/**
 * 构造单向燕尾箭头路径。
 * @param centerLine 原始箭头中心线。
 * @param shaftCenterLine 已截断到箭头头部基线的主体中心线。
 * @param metrics 燕尾箭头尺寸集合。
 * @return 单向燕尾箭头填充路径。
 */
QPainterPath singleFletchedArrowPath(const QVector<QPointF> &centerLine,
                                     const QVector<QPointF> &shaftCenterLine,
                                     const FletchedArrowMetrics &metrics)
{
    QPainterPath arrow;
    if (centerLine.isEmpty() || shaftCenterLine.size() < 2) {
        return arrow;
    }

    const QPointF tip = centerLine.last();
    const QPointF headBase = shaftCenterLine.last();
    const QPointF headDirection = tangentAtDistance(centerLine, polylineLength(centerLine));
    const QPointF headNormal = normalForDirection(headDirection);

    arrow.moveTo(shaftCenterLine.first());
    for (int i = 1; i < shaftCenterLine.size(); ++i) {
        arrow.lineTo(shaftCenterLine.at(i) + normalAtIndex(shaftCenterLine, i) * metrics.bodyHalfWidth);
    }
    arrow.lineTo(headBase + headNormal * metrics.headHalfWidth);
    arrow.lineTo(tip);
    arrow.lineTo(headBase - headNormal * metrics.headHalfWidth);
    for (int i = shaftCenterLine.size() - 1; i >= 1; --i) {
        arrow.lineTo(shaftCenterLine.at(i) - normalAtIndex(shaftCenterLine, i) * metrics.bodyHalfWidth);
    }
    arrow.closeSubpath();
    return arrow;
}

/**
 * 构造双向燕尾箭头路径。
 * @param centerLine 原始箭头中心线。
 * @param shaftCenterLine 已截断到两端头部基线的主体中心线。
 * @param metrics 燕尾箭头尺寸集合。
 * @return 双向燕尾箭头填充路径。
 */
QPainterPath bidirectionalFletchedArrowPath(const QVector<QPointF> &centerLine,
                                            const QVector<QPointF> &shaftCenterLine,
                                            const FletchedArrowMetrics &metrics)
{
    QPainterPath arrow;
    if (centerLine.isEmpty() || shaftCenterLine.size() < 2) {
        return arrow;
    }

    const QPointF startTip = centerLine.first();
    const QPointF endTip = centerLine.last();
    const QPointF startBase = shaftCenterLine.first();
    const QPointF endBase = shaftCenterLine.last();
    const QPointF startNormal = normalForDirection(tangentAtDistance(centerLine, 0.0));
    const QPointF endNormal = normalForDirection(tangentAtDistance(centerLine, polylineLength(centerLine)));

    arrow.moveTo(startTip);
    arrow.lineTo(startBase - startNormal * metrics.headHalfWidth);
    arrow.lineTo(startBase - startNormal * metrics.bodyHalfWidth);
    for (int i = 1; i < shaftCenterLine.size(); ++i) {
        arrow.lineTo(shaftCenterLine.at(i) - normalAtIndex(shaftCenterLine, i) * metrics.bodyHalfWidth);
    }
    arrow.lineTo(endBase - endNormal * metrics.headHalfWidth);
    arrow.lineTo(endTip);
    arrow.lineTo(endBase + endNormal * metrics.headHalfWidth);
    arrow.lineTo(endBase + endNormal * metrics.bodyHalfWidth);
    for (int i = shaftCenterLine.size() - 2; i >= 0; --i) {
        arrow.lineTo(shaftCenterLine.at(i) + normalAtIndex(shaftCenterLine, i) * metrics.bodyHalfWidth);
    }
    arrow.lineTo(startBase + startNormal * metrics.headHalfWidth);
    arrow.closeSubpath();
    return arrow;
}

/**
 * 绘制 KDE 开口箭头。
 * @param painter 当前绘制器。
 * @param points 箭头骨架点列表。
 * @param width 当前标注线宽。
 * @param bidirectional 是否为双向箭头。
 * @param centerLine 箭头中心线采样点。
 * @return 无返回值。
 */
void drawKdeArrow(QPainter &painter,
                  const QVector<QPointF> &points,
                  qreal width,
                  bool bidirectional,
                  const QVector<QPointF> &centerLine)
{
    if (centerLine.size() < 2) {
        return;
    }

    const qreal totalLength = polylineLength(centerLine);
    const qreal headLength = kdeHeadLength(totalLength, width, bidirectional);
    const qreal headHalfWidth = headLength * 0.62;
    const QColor color = painter.pen().color();
    const QPointF start = centerLine.first();
    const QPointF end = centerLine.last();
    const QPainterPath shaft = lineSkeletonPath(points);

    QPainterPath head;
    appendOpenArrowHead(&head, end, tangentAtDistance(centerLine, totalLength), headLength, headHalfWidth);
    if (bidirectional) {
        const QPointF startTangent = tangentAtDistance(centerLine, 0.0);
        appendOpenArrowHead(&head,
                            start,
                            QPointF(-startTangent.x(), -startTangent.y()),
                            headLength,
                            headHalfWidth);
    }

    painter.save();
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(color, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.drawPath(shaft);
    painter.drawPath(head);
    painter.restore();
}

/**
 * 绘制燕尾箭头。
 * @param painter 当前绘制器。
 * @param width 当前标注线宽。
 * @param bidirectional 是否为双向箭头。
 * @param centerLine 箭头中心线采样点。
 * @return 无返回值。
 */
void drawFletchedArrow(QPainter &painter,
                       qreal width,
                       bool bidirectional,
                       const QVector<QPointF> &centerLine)
{
    const qreal totalLength = polylineLength(centerLine);
    const FletchedArrowMetrics metrics = fletchedArrowMetrics(totalLength, width, bidirectional);
    const qreal startDistance = bidirectional ? metrics.headLength : 0.0;
    const qreal endDistance = totalLength - metrics.headLength;
    const QVector<QPointF> shaftCenterLine = centerLineSection(centerLine, startDistance, endDistance);
    const QPainterPath arrow = bidirectional
        ? bidirectionalFletchedArrowPath(centerLine, shaftCenterLine, metrics)
        : singleFletchedArrowPath(centerLine, shaftCenterLine, metrics);
    const QColor color = painter.pen().color();

    painter.save();
    painter.setPen(Qt::NoPen);
    painter.setBrush(color);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.drawPath(arrow);
    painter.restore();
}

} // namespace

void ShotWindow::drawArrow(QPainter &painter,
                           const QVector<QPointF> &points,
                           qreal width,
                           ArrowStyle style) const
{
    const QVector<QPointF> centerLine = sampledLineSkeletonCenterLine(points);
    if (polylineLength(centerLine) < 1.0) {
        return;
    }

    const bool kdeStyle = style == ArrowStyle::Kde || style == ArrowStyle::BidirectionalKde;
    const bool bidirectional = style == ArrowStyle::BidirectionalFletched || style == ArrowStyle::BidirectionalKde;
    if (kdeStyle) {
        drawKdeArrow(painter, points, width, bidirectional, centerLine);
        return;
    }

    drawFletchedArrow(painter, width, bidirectional, centerLine);
}
