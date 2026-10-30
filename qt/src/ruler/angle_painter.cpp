#include "ruler/angle_painter.h"

#include "ruler/ruler_painter.h"

#include <QFontMetricsF>
#include <QLineF>
#include <QPainter>
#include <QPainterPath>
#include <QPen>

#include <algorithm>
#include <cmath>

namespace markshot::ruler {

namespace {

// 以下尺寸均为默认粗细下的屏幕像素，绘制时乘以缩放倍数
// 顶点处圆弧半径的上下限，以及默认粗细下占较短一条边的比例
constexpr qreal kAngleArcMinRadius = 14.0;
constexpr qreal kAngleArcMaxRadius = 44.0;
constexpr qreal kAngleArcEdgeRatio = 0.4;
// 圆弧放大后最多占到较短一条边的比例，边的末端仍露在圆弧外
constexpr qreal kAngleArcEdgeLimit = 0.8;
// 角度说明与圆弧的距离，以及与边界的间距
constexpr qreal kAngleLabelGap = 10.0;
constexpr qreal kAngleFloatingGap = 8.0;
// 顶点和端点圆点的半径
constexpr qreal kAngleVertexRadius = 4.0;
constexpr qreal kAngleEndRadius = 3.0;

/**
 * 计算顶点处圆弧的半径：随较短一条边变短、随粗细放大，但不超出上下限，也不越过边长的限定比例。
 * @param angle 三点齐全的角度尺。
 * @return 圆弧半径，绘制坐标。
 */
qreal angleArcRadius(const AngleGeometry &angle)
{
    const qreal shorter = std::min(QLineF(angle.paintPoints.at(0), angle.paintPoints.at(1)).length(),
                                   QLineF(angle.paintPoints.at(0), angle.paintPoints.at(2)).length());
    const qreal radius = std::clamp(shorter * kAngleArcEdgeRatio * angle.sizeScale,
                                    kAngleArcMinRadius * angle.unit,
                                    kAngleArcMaxRadius * angle.unit);
    return std::min(radius, shorter * kAngleArcEdgeLimit);
}

/**
 * 计算从第一条边转到第二条边的有向角，取较小的一侧。
 * @param angle 三点齐全的角度尺。
 * @param startDegrees 第一条边的方向，Qt 角度约定，向右为 0、逆时针为正。
 * @param sweepDegrees 转到第二条边的转角，介于 -180 和 180 之间。
 * @return 无返回值。
 */
void angleArcSpan(const AngleGeometry &angle, qreal &startDegrees, qreal &sweepDegrees)
{
    const QPointF first = angle.paintPoints.at(1) - angle.paintPoints.at(0);
    const QPointF second = angle.paintPoints.at(2) - angle.paintPoints.at(0);
    // 1. 屏幕 y 轴向下，取负号换成 Qt 的逆时针为正
    startDegrees = std::atan2(-first.y(), first.x()) * 180.0 / M_PI;
    const qreal endDegrees = std::atan2(-second.y(), second.x()) * 180.0 / M_PI;
    sweepDegrees = std::remainder(endDegrees - startDegrees, 360.0);
}

}  // namespace

qreal angleDegrees(QPointF vertex, QPointF first, QPointF second)
{
    const QPointF a = first - vertex;
    const QPointF b = second - vertex;
    if (std::hypot(a.x(), a.y()) <= 0.0 || std::hypot(b.x(), b.y()) <= 0.0) {
        return 0.0;
    }
    const qreal radians = std::atan2(std::abs(a.x() * b.y() - a.y() * b.x()), a.x() * b.x() + a.y() * b.y());
    return radians * 180.0 / M_PI;
}

QString angleInfoText(qreal degrees)
{
    return QStringLiteral("%1°").arg(degrees, 0, 'f', 1);
}

QRectF angleInfoLabelRect(const AngleGeometry &angle, const QRectF &bounds)
{
    if (angle.imagePoints.size() < 3 || angle.paintPoints.size() < 3) {
        return {};
    }
    const qreal degrees = angleDegrees(angle.imagePoints.at(0), angle.imagePoints.at(1), angle.imagePoints.at(2));
    const QSizeF labelSize = rulerLabelSize(QFontMetricsF(rulerFont(angle.unit)), angleInfoText(degrees), angle.unit);

    // 1. 沿角平分线向外放到圆弧之外，两条边重合或反向时改用垂直方向
    qreal startDegrees = 0.0;
    qreal sweepDegrees = 0.0;
    angleArcSpan(angle, startDegrees, sweepDegrees);
    const qreal bisector = (startDegrees + sweepDegrees / 2.0) * M_PI / 180.0;
    const QPointF direction(std::cos(bisector), -std::sin(bisector));
    const qreal halfDiagonal = std::hypot(labelSize.width(), labelSize.height()) / 2.0;
    const qreal distance = angleArcRadius(angle) + kAngleLabelGap * angle.unit + halfDiagonal;
    const QPointF center = angle.paintPoints.at(0) + direction * distance;
    const QRectF rect(center - QPointF(labelSize.width() / 2.0, labelSize.height() / 2.0), labelSize);
    return clampRulerFloatingRect(rect, bounds, kAngleFloatingGap * angle.unit);
}

void drawAngleBody(QPainter &painter, const AngleGeometry &angle)
{
    if (angle.paintPoints.size() < 2) {
        return;
    }
    const QVector<QPointF> &points = angle.paintPoints;
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    // 1. 顶点处画扇形填充和圆弧，三点齐全后才有夹角
    const qreal lineWidth = std::max<qreal>(1.0, 2.0 * angle.unit);
    if (points.size() >= 3) {
        qreal startDegrees = 0.0;
        qreal sweepDegrees = 0.0;
        angleArcSpan(angle, startDegrees, sweepDegrees);
        const qreal radius = angleArcRadius(angle);
        const QRectF arcRect(points.at(0) - QPointF(radius, radius), QSizeF(radius * 2.0, radius * 2.0));
        QPainterPath wedge(points.at(0));
        wedge.arcTo(arcRect, startDegrees, sweepDegrees);
        wedge.closeSubpath();
        QColor fill = angle.color;
        fill.setAlphaF(angle.color.alphaF() * 0.16);
        painter.setPen(Qt::NoPen);
        painter.setBrush(fill);
        painter.drawPath(wedge);
        QPainterPath arc;
        arc.arcMoveTo(arcRect, startDegrees);
        arc.arcTo(arcRect, startDegrees, sweepDegrees);
        painter.setPen(QPen(angle.color, std::max<qreal>(1.0, 1.5 * angle.unit), Qt::SolidLine, Qt::RoundCap));
        painter.setBrush(Qt::NoBrush);
        painter.drawPath(arc);
    }

    // 2. 两条边从顶点出发
    painter.setPen(QPen(angle.color, lineWidth, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    for (int i = 1; i < points.size() && i <= 2; ++i) {
        painter.drawLine(points.at(0), points.at(i));
    }

    // 3. 顶点和端点画实心圆点
    painter.setPen(Qt::NoPen);
    painter.setBrush(angle.color);
    const qreal vertexRadius = kAngleVertexRadius * angle.unit;
    const qreal endRadius = kAngleEndRadius * angle.unit;
    painter.drawEllipse(points.at(0), vertexRadius, vertexRadius);
    for (int i = 1; i < points.size() && i <= 2; ++i) {
        painter.drawEllipse(points.at(i), endRadius, endRadius);
    }
    painter.restore();
}

}  // namespace markshot::ruler
