#pragma once

#include "shot_window.h"

#include <QPainterPath>
#include <QPainterPathStroker>
#include <QRectF>
#include <QString>
#include <QTransform>
#include <QtMath>

#include <array>

namespace markshot::marker {
namespace {

/// @brief 将中心线路径加粗为可填充轮廓。
/// @param centerline 中心线路径。
/// @param width 线宽（单位正方形坐标）。
/// @return 填充路径。
QPainterPath strokeOutline(const QPainterPath &centerline, qreal width)
{
    QPainterPathStroker stroker;
    stroker.setWidth(width);
    stroker.setCapStyle(Qt::RoundCap);
    stroker.setJoinStyle(Qt::RoundJoin);
    return stroker.createStroke(centerline);
}

/// @brief 生成正多边形路径。
/// @param sides 边数。
/// @param radius 外接圆半径。
/// @param startAngle 起始角（弧度，0 为右向，逆时针）。
/// @return 单位坐标系路径。
QPainterPath regularPolygon(int sides, qreal radius, qreal startAngle)
{
    QPainterPath path;
    for (int i = 0; i < sides; ++i) {
        const qreal angle = startAngle + i * (2.0 * M_PI / sides);
        const QPointF point(0.5 + radius * qCos(angle), 0.5 + radius * qSin(angle));
        if (i == 0) {
            path.moveTo(point);
        } else {
            path.lineTo(point);
        }
    }
    path.closeSubpath();
    return path;
}

/// @brief 生成尖顶星形路径。
/// @param points 尖角数量。
/// @param outer 外半径。
/// @param inner 内半径。
/// @return 单位坐标系路径。
QPainterPath starPolygon(int points, qreal outer, qreal inner)
{
    QPainterPath path;
    for (int i = 0; i < points * 2; ++i) {
        const qreal radius = (i % 2 == 0) ? outer : inner;
        const qreal angle = -M_PI_2 + i * (M_PI / points);
        const QPointF point(0.5 + radius * qCos(angle), 0.5 + radius * qSin(angle));
        if (i == 0) {
            path.moveTo(point);
        } else {
            path.lineTo(point);
        }
    }
    path.closeSubpath();
    return path;
}

}  // namespace

/// @brief 在单位矩形 [0,1]x[0,1] 内构造形状路径，再映射到目标矩形。
/// @param shape 形状种类。
/// @param rect 目标图像/控件坐标系矩形。
/// @return 可填充的闭合路径。
inline QPainterPath pathForShape(ShotWindow::MarkerShape shape, const QRectF &rect)
{
    const QRectF bounds = rect.normalized();
    if (bounds.isEmpty()) {
        return {};
    }

    // 1. 先在单位正方形内生成路径，便于后续等比映射
    QPainterPath unit;
    switch (shape) {
    case ShotWindow::MarkerShape::Triangle:
        unit.moveTo(0.50, 0.08);
        unit.lineTo(0.92, 0.90);
        unit.lineTo(0.08, 0.90);
        unit.closeSubpath();
        break;
    case ShotWindow::MarkerShape::Star:
        unit = starPolygon(5, 0.46, 0.19);
        break;
    case ShotWindow::MarkerShape::Check: {
        // 对钩：中心线 ✓ 再加粗成填充体，避免手写多边形走样
        QPainterPath centerline;
        centerline.moveTo(0.16, 0.52);
        centerline.lineTo(0.40, 0.76);
        centerline.lineTo(0.86, 0.24);
        unit = strokeOutline(centerline, 0.16);
        break;
    }
    case ShotWindow::MarkerShape::Cross: {
        // 叉叉：两条对角中心线加粗，形成清晰 X
        QPainterPath a;
        a.moveTo(0.20, 0.20);
        a.lineTo(0.80, 0.80);
        QPainterPath b;
        b.moveTo(0.80, 0.20);
        b.lineTo(0.20, 0.80);
        unit = strokeOutline(a, 0.15);
        unit.addPath(strokeOutline(b, 0.15));
        break;
    }
    case ShotWindow::MarkerShape::Diamond:
        unit.moveTo(0.50, 0.06);
        unit.lineTo(0.92, 0.50);
        unit.lineTo(0.50, 0.94);
        unit.lineTo(0.08, 0.50);
        unit.closeSubpath();
        break;
    case ShotWindow::MarkerShape::Heart: {
        unit.moveTo(0.50, 0.88);
        unit.cubicTo(0.14, 0.68, 0.08, 0.42, 0.22, 0.26);
        unit.cubicTo(0.32, 0.14, 0.44, 0.16, 0.50, 0.30);
        unit.cubicTo(0.56, 0.16, 0.68, 0.14, 0.78, 0.26);
        unit.cubicTo(0.92, 0.42, 0.86, 0.68, 0.50, 0.88);
        unit.closeSubpath();
        break;
    }
    case ShotWindow::MarkerShape::Hexagon:
        unit = regularPolygon(6, 0.46, -M_PI_2);
        break;
    case ShotWindow::MarkerShape::Circle:
        unit.addEllipse(QRectF(0.08, 0.08, 0.84, 0.84));
        break;
    case ShotWindow::MarkerShape::Square:
        unit.addRoundedRect(QRectF(0.12, 0.12, 0.76, 0.76), 0.08, 0.08);
        break;
    case ShotWindow::MarkerShape::Pentagon:
        unit = regularPolygon(5, 0.46, -M_PI_2);
        break;
    case ShotWindow::MarkerShape::Plus: {
        // 加号：横竖两条中心线加粗
        QPainterPath h;
        h.moveTo(0.16, 0.50);
        h.lineTo(0.84, 0.50);
        QPainterPath v;
        v.moveTo(0.50, 0.16);
        v.lineTo(0.50, 0.84);
        unit = strokeOutline(h, 0.18);
        unit.addPath(strokeOutline(v, 0.18));
        break;
    }
    case ShotWindow::MarkerShape::ArrowUp: {
        // 上箭头：三角头 + 竖杆
        unit.moveTo(0.50, 0.08);
        unit.lineTo(0.86, 0.46);
        unit.lineTo(0.66, 0.46);
        unit.lineTo(0.66, 0.90);
        unit.lineTo(0.34, 0.90);
        unit.lineTo(0.34, 0.46);
        unit.lineTo(0.14, 0.46);
        unit.closeSubpath();
        break;
    }
    case ShotWindow::MarkerShape::Spade: {
        // 黑桃：倒心形 + 短柄
        unit.moveTo(0.50, 0.08);
        unit.cubicTo(0.18, 0.28, 0.10, 0.48, 0.22, 0.62);
        unit.cubicTo(0.32, 0.74, 0.42, 0.70, 0.50, 0.58);
        unit.cubicTo(0.58, 0.70, 0.68, 0.74, 0.78, 0.62);
        unit.cubicTo(0.90, 0.48, 0.82, 0.28, 0.50, 0.08);
        unit.closeSubpath();
        unit.addRect(QRectF(0.44, 0.58, 0.12, 0.28));
        unit.moveTo(0.28, 0.92);
        unit.lineTo(0.72, 0.92);
        unit.lineTo(0.62, 0.80);
        unit.lineTo(0.38, 0.80);
        unit.closeSubpath();
        break;
    }
    case ShotWindow::MarkerShape::Club: {
        // 梅花：三圆 + 柄
        unit.addEllipse(QPointF(0.50, 0.28), 0.18, 0.18);
        unit.addEllipse(QPointF(0.30, 0.50), 0.18, 0.18);
        unit.addEllipse(QPointF(0.70, 0.50), 0.18, 0.18);
        unit.addRect(QRectF(0.44, 0.52, 0.12, 0.28));
        unit.moveTo(0.28, 0.92);
        unit.lineTo(0.72, 0.92);
        unit.lineTo(0.62, 0.80);
        unit.lineTo(0.38, 0.80);
        unit.closeSubpath();
        break;
    }
    case ShotWindow::MarkerShape::Lightning: {
        // 闪电折线
        unit.moveTo(0.58, 0.06);
        unit.lineTo(0.28, 0.50);
        unit.lineTo(0.48, 0.50);
        unit.lineTo(0.36, 0.94);
        unit.lineTo(0.78, 0.42);
        unit.lineTo(0.56, 0.42);
        unit.closeSubpath();
        break;
    }
    case ShotWindow::MarkerShape::Ban: {
        // 禁止：圆环 + 斜杠
        QPainterPath outer;
        outer.addEllipse(QRectF(0.08, 0.08, 0.84, 0.84));
        QPainterPath hole;
        hole.addEllipse(QRectF(0.20, 0.20, 0.60, 0.60));
        unit = outer.subtracted(hole);
        QPainterPath slash;
        slash.moveTo(0.24, 0.72);
        slash.lineTo(0.76, 0.28);
        unit.addPath(strokeOutline(slash, 0.12));
        break;
    }
    case ShotWindow::MarkerShape::Octagon:
        unit = regularPolygon(8, 0.46, M_PI / 8.0);
        break;
    case ShotWindow::MarkerShape::Crescent: {
        // 新月：大圆减去偏移小圆
        QPainterPath full;
        full.addEllipse(QRectF(0.10, 0.10, 0.80, 0.80));
        QPainterPath cut;
        cut.addEllipse(QRectF(0.28, 0.08, 0.72, 0.72));
        unit = full.subtracted(cut);
        break;
    }
    }

    // 2. 映射到目标矩形
    QTransform transform;
    transform.translate(bounds.left(), bounds.top());
    transform.scale(bounds.width(), bounds.height());
    return transform.map(unit);
}

/// @brief 返回形状的稳定配置名。
/// @param shape 形状种类。
/// @return 小写英文名。
inline QString shapeName(ShotWindow::MarkerShape shape)
{
    switch (shape) {
    case ShotWindow::MarkerShape::Triangle:
        return QStringLiteral("triangle");
    case ShotWindow::MarkerShape::Star:
        return QStringLiteral("star");
    case ShotWindow::MarkerShape::Check:
        return QStringLiteral("check");
    case ShotWindow::MarkerShape::Cross:
        return QStringLiteral("cross");
    case ShotWindow::MarkerShape::Diamond:
        return QStringLiteral("diamond");
    case ShotWindow::MarkerShape::Heart:
        return QStringLiteral("heart");
    case ShotWindow::MarkerShape::Hexagon:
        return QStringLiteral("hexagon");
    case ShotWindow::MarkerShape::Circle:
        return QStringLiteral("circle");
    case ShotWindow::MarkerShape::Square:
        return QStringLiteral("square");
    case ShotWindow::MarkerShape::Pentagon:
        return QStringLiteral("pentagon");
    case ShotWindow::MarkerShape::Plus:
        return QStringLiteral("plus");
    case ShotWindow::MarkerShape::ArrowUp:
        return QStringLiteral("arrow-up");
    case ShotWindow::MarkerShape::Spade:
        return QStringLiteral("spade");
    case ShotWindow::MarkerShape::Club:
        return QStringLiteral("club");
    case ShotWindow::MarkerShape::Lightning:
        return QStringLiteral("lightning");
    case ShotWindow::MarkerShape::Ban:
        return QStringLiteral("ban");
    case ShotWindow::MarkerShape::Octagon:
        return QStringLiteral("octagon");
    case ShotWindow::MarkerShape::Crescent:
        return QStringLiteral("crescent");
    }
    return QStringLiteral("triangle");
}

/// @brief 返回形状的展示名（英文源串，走 translate）。
/// @param shape 形状种类。
/// @return 展示名。
inline QString shapeLabel(ShotWindow::MarkerShape shape)
{
    switch (shape) {
    case ShotWindow::MarkerShape::Triangle:
        return QStringLiteral("Triangle");
    case ShotWindow::MarkerShape::Star:
        return QStringLiteral("Star");
    case ShotWindow::MarkerShape::Check:
        return QStringLiteral("Check");
    case ShotWindow::MarkerShape::Cross:
        return QStringLiteral("Cross");
    case ShotWindow::MarkerShape::Diamond:
        return QStringLiteral("Diamond");
    case ShotWindow::MarkerShape::Heart:
        return QStringLiteral("Heart");
    case ShotWindow::MarkerShape::Hexagon:
        return QStringLiteral("Hexagon");
    case ShotWindow::MarkerShape::Circle:
        return QStringLiteral("Circle");
    case ShotWindow::MarkerShape::Square:
        return QStringLiteral("Square");
    case ShotWindow::MarkerShape::Pentagon:
        return QStringLiteral("Pentagon");
    case ShotWindow::MarkerShape::Plus:
        return QStringLiteral("Plus");
    case ShotWindow::MarkerShape::ArrowUp:
        return QStringLiteral("Arrow Up");
    case ShotWindow::MarkerShape::Spade:
        return QStringLiteral("Spade");
    case ShotWindow::MarkerShape::Club:
        return QStringLiteral("Club");
    case ShotWindow::MarkerShape::Lightning:
        return QStringLiteral("Lightning");
    case ShotWindow::MarkerShape::Ban:
        return QStringLiteral("Ban");
    case ShotWindow::MarkerShape::Octagon:
        return QStringLiteral("Octagon");
    case ShotWindow::MarkerShape::Crescent:
        return QStringLiteral("Crescent");
    }
    return QStringLiteral("Triangle");
}

/// @brief 全部可用形状，顺序即工具栏弹层网格顺序。
inline std::array<ShotWindow::MarkerShape, 18> allShapes()
{
    return {
        ShotWindow::MarkerShape::Triangle,
        ShotWindow::MarkerShape::Star,
        ShotWindow::MarkerShape::Check,
        ShotWindow::MarkerShape::Cross,
        ShotWindow::MarkerShape::Diamond,
        ShotWindow::MarkerShape::Heart,
        ShotWindow::MarkerShape::Spade,
        ShotWindow::MarkerShape::Club,
        ShotWindow::MarkerShape::Circle,
        ShotWindow::MarkerShape::Square,
        ShotWindow::MarkerShape::Pentagon,
        ShotWindow::MarkerShape::Hexagon,
        ShotWindow::MarkerShape::Octagon,
        ShotWindow::MarkerShape::Plus,
        ShotWindow::MarkerShape::ArrowUp,
        ShotWindow::MarkerShape::Lightning,
        ShotWindow::MarkerShape::Ban,
        ShotWindow::MarkerShape::Crescent,
    };
}

}  // namespace markshot::marker
