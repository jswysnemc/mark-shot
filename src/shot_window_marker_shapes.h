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

/// @brief 将中心线路径加粗为圆角填充轮廓（曲线端帽）。
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

/// @brief 构造水平条路径（加号/禁止斜杠的矩形近似）。
/// @param x1 左。
/// @param y1 上。
/// @param x2 右。
/// @param y2 下。
/// @return 矩形路径。
QPainterPath axisAlignedBar(qreal x1, qreal y1, qreal x2, qreal y2)
{
    QPainterPath path;
    path.addRect(QRectF(QPointF(x1, y1), QPointF(x2, y2)).normalized());
    return path;
}

/// @brief 构造任意角度的厚线段多边形。
/// @param a 起点。
/// @param b 终点。
/// @param halfWidth 半宽。
/// @return 填充多边形。
QPainterPath thickSegment(QPointF a, QPointF b, qreal halfWidth)
{
    const QPointF delta = b - a;
    const qreal len = qHypot(delta.x(), delta.y());
    if (len < 1e-6) {
        return {};
    }
    const QPointF n(-delta.y() / len * halfWidth, delta.x() / len * halfWidth);
    QPainterPath path;
    path.moveTo(a + n);
    path.lineTo(b + n);
    path.lineTo(b - n);
    path.lineTo(a - n);
    path.closeSubpath();
    return path;
}

/// @brief 用布尔并集合并多条实心路径，消除重叠处的挖空。
/// @param parts 子路径。
/// @return 合并后的单一外轮廓路径。
QPainterPath uniteParts(std::initializer_list<QPainterPath> parts)
{
    QPainterPath result;
    bool hasResult = false;
    for (const QPainterPath &part : parts) {
        if (part.isEmpty()) {
            continue;
        }
        if (!hasResult) {
            result = part;
            hasResult = true;
            continue;
        }
        // united() 做真正的区域并集，比 addPath + WindingFill 更稳
        result = result.united(part);
    }
    result.setFillRule(Qt::WindingFill);
    return result;
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

    // 1. 单位正方形内生成路径；多部件统一 WindingFill，避免重叠区域被 OddEven 挖空
    QPainterPath unit;
    unit.setFillRule(Qt::WindingFill);

    switch (shape) {
    case ShotWindow::MarkerShape::Triangle:
        unit.moveTo(0.50, 0.10);
        unit.lineTo(0.90, 0.88);
        unit.lineTo(0.10, 0.88);
        unit.closeSubpath();
        break;

    case ShotWindow::MarkerShape::Star:
        unit = starPolygon(5, 0.45, 0.18);
        unit.setFillRule(Qt::WindingFill);
        break;

    case ShotWindow::MarkerShape::Check: {
        // 对钩：圆角中心线加粗成 ✓，端帽与拐角圆润
        QPainterPath line;
        line.moveTo(0.16, 0.54);
        line.lineTo(0.40, 0.76);
        line.lineTo(0.86, 0.24);
        unit = strokeOutline(line, 0.17);
        unit.setFillRule(Qt::WindingFill);
        break;
    }

    case ShotWindow::MarkerShape::Cross: {
        // 叉叉：两条圆角对角线，端帽圆润
        QPainterPath a;
        a.moveTo(0.24, 0.24);
        a.lineTo(0.76, 0.76);
        QPainterPath b;
        b.moveTo(0.76, 0.24);
        b.lineTo(0.24, 0.76);
        unit = uniteParts({strokeOutline(a, 0.16), strokeOutline(b, 0.16)});
        break;
    }

    case ShotWindow::MarkerShape::Diamond:
        unit.moveTo(0.50, 0.08);
        unit.lineTo(0.90, 0.50);
        unit.lineTo(0.50, 0.92);
        unit.lineTo(0.10, 0.50);
        unit.closeSubpath();
        break;

    case ShotWindow::MarkerShape::Heart: {
        // 红心：顶部两瓣 + 底部尖角，单路径避免挖空
        unit.moveTo(0.50, 0.86);
        unit.cubicTo(0.18, 0.68, 0.08, 0.44, 0.20, 0.28);
        unit.cubicTo(0.28, 0.16, 0.42, 0.16, 0.50, 0.30);
        unit.cubicTo(0.58, 0.16, 0.72, 0.16, 0.80, 0.28);
        unit.cubicTo(0.92, 0.44, 0.82, 0.68, 0.50, 0.86);
        unit.closeSubpath();
        break;
    }

    case ShotWindow::MarkerShape::Hexagon:
        unit = regularPolygon(6, 0.45, -M_PI_2);
        break;

    case ShotWindow::MarkerShape::Circle:
        unit.addEllipse(QRectF(0.10, 0.10, 0.80, 0.80));
        break;

    case ShotWindow::MarkerShape::Square:
        unit.addRoundedRect(QRectF(0.14, 0.14, 0.72, 0.72), 0.08, 0.08);
        break;

    case ShotWindow::MarkerShape::Pentagon:
        unit = regularPolygon(5, 0.45, -M_PI_2);
        break;

    case ShotWindow::MarkerShape::Plus: {
        // 加号：横竖两矩形合并，交叉处不会被 OddEven 挖空
        const QPainterPath h = axisAlignedBar(0.14, 0.40, 0.86, 0.60);
        const QPainterPath v = axisAlignedBar(0.40, 0.14, 0.60, 0.86);
        unit = uniteParts({h, v});
        break;
    }

    case ShotWindow::MarkerShape::ArrowUp: {
        // 上箭头：三角头 + 竖杆，单闭合路径
        unit.moveTo(0.50, 0.10);
        unit.lineTo(0.84, 0.46);
        unit.lineTo(0.66, 0.46);
        unit.lineTo(0.66, 0.88);
        unit.lineTo(0.34, 0.88);
        unit.lineTo(0.34, 0.46);
        unit.lineTo(0.16, 0.46);
        unit.closeSubpath();
        break;
    }

    case ShotWindow::MarkerShape::Spade: {
        // 黑桃：整枚用三次贝塞尔画成连续曲线轮廓（牌面风格，无直线多边形）
        QPainterPath path;
        // 1. 顶部尖端
        path.moveTo(0.50, 0.05);
        // 2. 左瓣
        path.cubicTo(0.28, 0.18, 0.10, 0.34, 0.10, 0.50);
        path.cubicTo(0.10, 0.64, 0.22, 0.74, 0.38, 0.68);
        // 3. 左侧入柄
        path.cubicTo(0.42, 0.74, 0.43, 0.80, 0.38, 0.86);
        // 4. 左脚（圆润）
        path.cubicTo(0.30, 0.90, 0.28, 0.95, 0.34, 0.97);
        path.cubicTo(0.40, 0.99, 0.46, 0.97, 0.50, 0.93);
        // 5. 右脚
        path.cubicTo(0.54, 0.97, 0.60, 0.99, 0.66, 0.97);
        path.cubicTo(0.72, 0.95, 0.70, 0.90, 0.62, 0.86);
        // 6. 右侧出柄
        path.cubicTo(0.57, 0.80, 0.58, 0.74, 0.62, 0.68);
        // 7. 右瓣回到尖端
        path.cubicTo(0.78, 0.74, 0.90, 0.64, 0.90, 0.50);
        path.cubicTo(0.90, 0.34, 0.72, 0.18, 0.50, 0.05);
        path.closeSubpath();
        unit = path;
        break;
    }

    case ShotWindow::MarkerShape::Club: {
        // 梅花：三瓣圆（曲线）+ 圆角柄 + 圆润底座，全部曲线
        QPainterPath top;
        top.addEllipse(QPointF(0.50, 0.28), 0.20, 0.20);
        QPainterPath left;
        left.addEllipse(QPointF(0.33, 0.50), 0.20, 0.20);
        QPainterPath right;
        right.addEllipse(QPointF(0.67, 0.50), 0.20, 0.20);
        QPainterPath core;
        core.addEllipse(QPointF(0.50, 0.45), 0.12, 0.12);

        // 竖柄：中心线加粗成圆角胶囊（RoundCap 曲线）
        QPainterPath stemLine;
        stemLine.moveTo(0.50, 0.52);
        stemLine.lineTo(0.50, 0.80);
        const QPainterPath stem = strokeOutline(stemLine, 0.11);

        // 底座：全三次曲线的圆润脚
        QPainterPath base;
        base.moveTo(0.34, 0.84);
        base.cubicTo(0.30, 0.88, 0.30, 0.94, 0.36, 0.97);
        base.cubicTo(0.42, 0.99, 0.48, 0.99, 0.50, 0.99);
        base.cubicTo(0.52, 0.99, 0.58, 0.99, 0.64, 0.97);
        base.cubicTo(0.70, 0.94, 0.70, 0.88, 0.66, 0.84);
        base.cubicTo(0.60, 0.80, 0.54, 0.80, 0.50, 0.80);
        base.cubicTo(0.46, 0.80, 0.40, 0.80, 0.34, 0.84);
        base.closeSubpath();

        unit = uniteParts({top, left, right, core, stem, base});
        break;
    }

    case ShotWindow::MarkerShape::Lightning: {
        // 闪电：更饱满的经典闪电轮廓，折点分明且不自交
        unit.moveTo(0.62, 0.06);
        unit.lineTo(0.24, 0.54);
        unit.lineTo(0.44, 0.54);
        unit.lineTo(0.36, 0.94);
        unit.lineTo(0.78, 0.42);
        unit.lineTo(0.56, 0.42);
        unit.closeSubpath();
        break;
    }

    case ShotWindow::MarkerShape::Ban: {
        // 禁止：外圆/内圆用相反绕组形成圆环，斜杠用圆角曲线加粗
        // 不使用 united(ring, slash)，否则部分 Qt 版本会把中空填实
        QPainterPath path;
        path.setFillRule(Qt::WindingFill);

        // 1. 外圆（正向）
        path.addEllipse(QRectF(0.08, 0.08, 0.84, 0.84));
        // 2. 内圆（反向）形成中空
        QPainterPath hole;
        hole.addEllipse(QRectF(0.24, 0.24, 0.52, 0.52));
        path.addPath(hole.toReversed());

        // 3. 圆角斜杠（RoundCap 曲线）
        QPainterPath slashLine;
        slashLine.moveTo(0.26, 0.74);
        slashLine.lineTo(0.74, 0.26);
        path.addPath(strokeOutline(slashLine, 0.14));

        unit = path;
        unit.setFillRule(Qt::WindingFill);
        break;
    }

    case ShotWindow::MarkerShape::Octagon:
        unit = regularPolygon(8, 0.45, M_PI / 8.0);
        break;

    case ShotWindow::MarkerShape::Crescent: {
        // 新月：大圆减偏移圆；裁剪圆上移右移让月牙更饱满弯曲
        QPainterPath full;
        full.addEllipse(QRectF(0.10, 0.10, 0.80, 0.80));
        QPainterPath cut;
        cut.addEllipse(QRectF(0.32, 0.02, 0.74, 0.74));
        unit = full.subtracted(cut);
        unit.setFillRule(Qt::WindingFill);
        break;
    }

    case ShotWindow::MarkerShape::Pin: {
        // 定位针：圆头 + 底部尖端合并，再挖出中心孔
        QPainterPath head;
        head.addEllipse(QPointF(0.50, 0.38), 0.30, 0.30);
        QPainterPath tail;
        tail.moveTo(0.50, 0.96);
        tail.lineTo(0.27, 0.54);
        tail.lineTo(0.73, 0.54);
        tail.closeSubpath();
        QPainterPath hole;
        hole.addEllipse(QPointF(0.50, 0.38), 0.115, 0.115);
        unit = uniteParts({head, tail}).subtracted(hole);
        unit.setFillRule(Qt::WindingFill);
        break;
    }

    case ShotWindow::MarkerShape::Flag: {
        // 旗帜：圆角旗杆 + 三角旗面
        QPainterPath poleLine;
        poleLine.moveTo(0.24, 0.10);
        poleLine.lineTo(0.24, 0.92);
        const QPainterPath pole = strokeOutline(poleLine, 0.09);
        QPainterPath banner;
        banner.moveTo(0.28, 0.12);
        banner.lineTo(0.86, 0.28);
        banner.lineTo(0.28, 0.46);
        banner.closeSubpath();
        unit = uniteParts({pole, banner});
        break;
    }

    case ShotWindow::MarkerShape::Bookmark: {
        // 书签：竖直圆角带 + 底部 V 形缺口
        unit.moveTo(0.30, 0.10);
        unit.cubicTo(0.30, 0.07, 0.33, 0.06, 0.36, 0.06);
        unit.lineTo(0.64, 0.06);
        unit.cubicTo(0.67, 0.06, 0.70, 0.07, 0.70, 0.10);
        unit.lineTo(0.70, 0.93);
        unit.lineTo(0.50, 0.74);
        unit.lineTo(0.30, 0.93);
        unit.closeSubpath();
        break;
    }

    case ShotWindow::MarkerShape::Shield: {
        // 盾牌：顶部平缓曲线，两侧收腰，底部圆尖
        unit.moveTo(0.50, 0.05);
        unit.cubicTo(0.62, 0.12, 0.76, 0.15, 0.88, 0.16);
        unit.cubicTo(0.88, 0.48, 0.78, 0.76, 0.50, 0.95);
        unit.cubicTo(0.22, 0.76, 0.12, 0.48, 0.12, 0.16);
        unit.cubicTo(0.24, 0.15, 0.38, 0.12, 0.50, 0.05);
        unit.closeSubpath();
        break;
    }

    case ShotWindow::MarkerShape::Exclamation: {
        // 感叹号：上粗下细的圆角竖条 + 底部圆点
        QPainterPath bar;
        bar.moveTo(0.40, 0.12);
        bar.cubicTo(0.40, 0.06, 0.60, 0.06, 0.60, 0.12);
        bar.lineTo(0.55, 0.60);
        bar.cubicTo(0.55, 0.66, 0.45, 0.66, 0.45, 0.60);
        bar.closeSubpath();
        QPainterPath dot;
        dot.addEllipse(QPointF(0.50, 0.84), 0.095, 0.095);
        unit = uniteParts({bar, dot});
        break;
    }

    case ShotWindow::MarkerShape::SpeechBubble: {
        // 对话气泡：圆角矩形 + 左下小尾巴
        QPainterPath body;
        body.addRoundedRect(QRectF(0.08, 0.12, 0.84, 0.58), 0.14, 0.14);
        QPainterPath tailPath;
        tailPath.moveTo(0.26, 0.66);
        tailPath.lineTo(0.20, 0.92);
        tailPath.lineTo(0.46, 0.69);
        tailPath.closeSubpath();
        unit = uniteParts({body, tailPath});
        break;
    }
    }

    // 2. 映射到目标矩形；保留各形状自身 fillRule（禁止图标需要 OddEvenFill）
    QTransform transform;
    transform.translate(bounds.left(), bounds.top());
    transform.scale(bounds.width(), bounds.height());
    const Qt::FillRule fillRule = unit.fillRule();
    QPainterPath mapped = transform.map(unit);
    mapped.setFillRule(fillRule);
    return mapped;
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
    case ShotWindow::MarkerShape::Pin:
        return QStringLiteral("pin");
    case ShotWindow::MarkerShape::Flag:
        return QStringLiteral("flag");
    case ShotWindow::MarkerShape::Bookmark:
        return QStringLiteral("bookmark");
    case ShotWindow::MarkerShape::Shield:
        return QStringLiteral("shield");
    case ShotWindow::MarkerShape::Exclamation:
        return QStringLiteral("exclamation");
    case ShotWindow::MarkerShape::SpeechBubble:
        return QStringLiteral("speech-bubble");
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
    case ShotWindow::MarkerShape::Pin:
        return QStringLiteral("Pin");
    case ShotWindow::MarkerShape::Flag:
        return QStringLiteral("Flag");
    case ShotWindow::MarkerShape::Bookmark:
        return QStringLiteral("Bookmark");
    case ShotWindow::MarkerShape::Shield:
        return QStringLiteral("Shield");
    case ShotWindow::MarkerShape::Exclamation:
        return QStringLiteral("Exclamation");
    case ShotWindow::MarkerShape::SpeechBubble:
        return QStringLiteral("Speech Bubble");
    }
    return QStringLiteral("Triangle");
}

/// @brief 全部可用形状，顺序即工具栏弹层网格顺序。
inline std::array<ShotWindow::MarkerShape, 24> allShapes()
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
        ShotWindow::MarkerShape::Pin,
        ShotWindow::MarkerShape::Flag,
        ShotWindow::MarkerShape::Bookmark,
        ShotWindow::MarkerShape::Shield,
        ShotWindow::MarkerShape::Exclamation,
        ShotWindow::MarkerShape::SpeechBubble,
    };
}

}  // namespace markshot::marker
