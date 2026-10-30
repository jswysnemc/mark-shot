#include "ruler/ruler_geometry.h"

#include <QLineF>
#include <QtMath>

#include <algorithm>
#include <cmath>
#include <limits>

namespace markshot::ruler {

namespace {

using markshot::shot::types::RulerMode;

// 控制点命中半径相对绘制半径的放大倍数，与直线端点一致
constexpr qreal kHandleHitScale = 1.4;

/**
 * 把宽或高不足 1 像素的范围以中心为准补足到 1 像素。
 * @param rect 原始范围。
 * @return 补足后的范围。
 */
QRectF inflatedToPixel(QRectF rect)
{
    rect = rect.normalized();
    if (rect.width() < 1.0) {
        rect.adjust(-(1.0 - rect.width()) / 2.0, 0.0, (1.0 - rect.width()) / 2.0, 0.0);
    }
    if (rect.height() < 1.0) {
        rect.adjust(0.0, -(1.0 - rect.height()) / 2.0, 0.0, (1.0 - rect.height()) / 2.0);
    }
    return rect;
}

/**
 * 计算点到线段的距离。
 * @param point 待测点。
 * @param start 线段起点。
 * @param end 线段终点。
 * @return 距离。
 */
qreal distanceToSegment(QPointF point, QPointF start, QPointF end)
{
    const QPointF segment = end - start;
    const qreal lengthSquared = segment.x() * segment.x() + segment.y() * segment.y();
    if (lengthSquared <= 0.0) {
        return QLineF(point, start).length();
    }
    const QPointF offset = point - start;
    const qreal t = std::clamp((offset.x() * segment.x() + offset.y() * segment.y()) / lengthSquared, 0.0, 1.0);
    return QLineF(point, start + segment * t).length();
}

/**
 * 把点绕轴心旋转，与 QPainter::rotate 同向，屏幕上顺时针为正。
 * @param point 待旋转的点。
 * @param center 旋转轴心。
 * @param degrees 旋转角度。
 * @return 旋转后的点。
 */
QPointF rotatedAround(QPointF point, QPointF center, qreal degrees)
{
    if (qFuzzyIsNull(degrees)) {
        return point;
    }
    const qreal radians = qDegreesToRadians(degrees);
    const qreal c = std::cos(radians);
    const qreal s = std::sin(radians);
    const QPointF delta = point - center;
    return center + QPointF(delta.x() * c - delta.y() * s, delta.x() * s + delta.y() * c);
}

/**
 * 计算尺子的旋转角度，只有长度尺随测量框旋转，角度尺的旋转已写进三个定点。
 * @param annotation 尺子标注。
 * @return 旋转角度。
 */
qreal rulerRotation(const Annotation &annotation)
{
    return annotation.rulerMode == RulerMode::Angle ? 0.0 : annotation.rotationDegrees;
}

}  // namespace

bool rulerMeasures(const Annotation &annotation)
{
    if (annotation.rulerMode == RulerMode::Angle) {
        return annotation.points.size() >= 3
            && QLineF(annotation.points.at(0), annotation.points.at(1)).length() >= 1.0
            && QLineF(annotation.points.at(0), annotation.points.at(2)).length() >= 1.0;
    }
    return std::abs(annotation.rect.width()) >= 1.0 || std::abs(annotation.rect.height()) >= 1.0;
}

QRectF rulerBounds(const Annotation &annotation)
{
    if (annotation.rulerMode != RulerMode::Angle) {
        return inflatedToPixel(annotation.rect);
    }
    if (annotation.points.isEmpty()) {
        return {};
    }
    // 1. 角度尺取三点的外接矩形
    qreal left = annotation.points.first().x();
    qreal right = left;
    qreal top = annotation.points.first().y();
    qreal bottom = top;
    for (const QPointF &point : annotation.points) {
        left = std::min(left, point.x());
        right = std::max(right, point.x());
        top = std::min(top, point.y());
        bottom = std::max(bottom, point.y());
    }
    return inflatedToPixel(QRectF(QPointF(left, top), QPointF(right, bottom)));
}

QVector<RulerHandle> rulerHandles(const Annotation &annotation, qreal tolerance)
{
    QVector<RulerHandle> handles;
    if (annotation.rulerMode == RulerMode::Angle) {
        const SelectionDrag drags[] = {SelectionDrag::AngleVertex, SelectionDrag::AngleFirstArm,
                                       SelectionDrag::AngleSecondArm};
        for (int i = 0; i < annotation.points.size() && i < 3; ++i) {
            handles.append({annotation.points.at(i), drags[i]});
        }
        return handles;
    }

    // 1. 四角始终可拖，平直的尺子靠角点拉成矩形
    const QRectF rect = annotation.rect.normalized();
    const QPointF center = rulerBounds(annotation).center();
    const qreal rotation = rulerRotation(annotation);
    handles = {
        {rect.topLeft(), SelectionDrag::TopLeft},
        {rect.topRight(), SelectionDrag::TopRight},
        {rect.bottomLeft(), SelectionDrag::BottomLeft},
        {rect.bottomRight(), SelectionDrag::BottomRight},
    };
    // 2. 中点与角点、对边中点都拉得开时才提供，避免平直的尺子上中点和角点叠在一起
    if (rect.width() > tolerance * 3.0 && rect.height() > tolerance * 2.0) {
        handles.append({QPointF(rect.center().x(), rect.top()), SelectionDrag::Top});
        handles.append({QPointF(rect.center().x(), rect.bottom()), SelectionDrag::Bottom});
    }
    if (rect.height() > tolerance * 3.0 && rect.width() > tolerance * 2.0) {
        handles.append({QPointF(rect.left(), rect.center().y()), SelectionDrag::Left});
        handles.append({QPointF(rect.right(), rect.center().y()), SelectionDrag::Right});
    }

    // 3. 控制点随测量框一起旋转
    for (RulerHandle &handle : handles) {
        handle.point = rotatedAround(handle.point, center, rotation);
    }
    return handles;
}

void rotateAnglePoints(Annotation &annotation, QPointF center, qreal degrees)
{
    for (QPointF &point : annotation.points) {
        point = rotatedAround(point, center, degrees);
    }
}

qreal angleFirstArmDegrees(const Annotation &annotation)
{
    if (annotation.points.size() < 2) {
        return 0.0;
    }
    const QPointF arm = annotation.points.at(1) - annotation.points.at(0);
    if (qFuzzyIsNull(arm.x()) && qFuzzyIsNull(arm.y())) {
        return 0.0;
    }
    return qRadiansToDegrees(std::atan2(arm.y(), arm.x()));
}

SelectionDrag rulerDragAt(const Annotation &annotation, QPointF point, qreal tolerance, bool interiorMoves)
{
    // 1. 最近的控制点优先
    SelectionDrag best = SelectionDrag::None;
    qreal bestDistance = std::numeric_limits<qreal>::max();
    for (const RulerHandle &handle : rulerHandles(annotation, tolerance)) {
        const qreal distance = QLineF(point, handle.point).length();
        if (distance <= tolerance * kHandleHitScale && distance < bestDistance) {
            best = handle.drag;
            bestDistance = distance;
        }
    }
    if (best != SelectionDrag::None) {
        return best;
    }

    // 2. 角度尺按两条边命中，长度尺按边框命中，内部按参数决定
    if (annotation.rulerMode == RulerMode::Angle) {
        for (int i = 1; i < annotation.points.size() && i < 3; ++i) {
            if (distanceToSegment(point, annotation.points.at(0), annotation.points.at(i)) <= tolerance) {
                return SelectionDrag::Move;
            }
        }
        return SelectionDrag::None;
    }
    // 3. 旋转的长度尺把指针换算回测量框未旋转时的位置
    const QRectF bounds = rulerBounds(annotation);
    point = rotatedAround(point, bounds.center(), -rulerRotation(annotation));
    if (!bounds.adjusted(-tolerance, -tolerance, tolerance, tolerance).contains(point)) {
        return SelectionDrag::None;
    }
    const QRectF interior = bounds.adjusted(tolerance, tolerance, -tolerance, -tolerance);
    const bool insideInterior = interior.width() > 0.0 && interior.height() > 0.0 && interior.contains(point);
    return !insideInterior || interiorMoves ? SelectionDrag::Move : SelectionDrag::None;
}

bool isRulerHandleDrag(SelectionDrag drag)
{
    switch (drag) {
    case SelectionDrag::Left:
    case SelectionDrag::Right:
    case SelectionDrag::Top:
    case SelectionDrag::Bottom:
    case SelectionDrag::TopLeft:
    case SelectionDrag::TopRight:
    case SelectionDrag::BottomLeft:
    case SelectionDrag::BottomRight:
    case SelectionDrag::AngleVertex:
    case SelectionDrag::AngleFirstArm:
    case SelectionDrag::AngleSecondArm:
        return true;
    default:
        return false;
    }
}

Annotation draggedRuler(const Annotation &before, SelectionDrag drag, QPointF point)
{
    Annotation after = before;
    if (before.rulerMode == RulerMode::Angle) {
        // 1. 角度尺只移动被拖的点，其他两点保持原位
        const int index = drag == SelectionDrag::AngleVertex ? 0 : drag == SelectionDrag::AngleFirstArm ? 1
            : drag == SelectionDrag::AngleSecondArm ? 2 : -1;
        if (index >= 0 && index < after.points.size()) {
            after.points[index] = point;
        }
        return after;
    }

    // 2. 长度尺拖角时对角不动，拖边时对边不动，越过对侧时自然翻转；旋转时在测量框未旋转的坐标中计算
    const QRectF rect = before.rect.normalized();
    const QPointF center = rulerBounds(before).center();
    const qreal rotation = rulerRotation(before);
    QPointF anchor;
    QPointF moved = rotatedAround(point, center, -rotation);
    switch (drag) {
    case SelectionDrag::TopLeft: anchor = rect.bottomRight(); break;
    case SelectionDrag::TopRight: anchor = rect.bottomLeft(); break;
    case SelectionDrag::BottomLeft: anchor = rect.topRight(); break;
    case SelectionDrag::BottomRight: anchor = rect.topLeft(); break;
    case SelectionDrag::Left: anchor = rect.bottomRight(); moved.setY(rect.top()); break;
    case SelectionDrag::Right: anchor = rect.topLeft(); moved.setY(rect.bottom()); break;
    case SelectionDrag::Top: anchor = rect.bottomRight(); moved.setX(rect.left()); break;
    case SelectionDrag::Bottom: anchor = rect.topLeft(); moved.setX(rect.right()); break;
    default: return after;
    }
    after.rect = QRectF(anchor, moved).normalized();
    if (qFuzzyIsNull(rotation)) {
        return after;
    }

    // 3. 新测量框绕自身中心旋转，中心取屏幕上不动的点与指针的中点，不动的角或边才留在原处
    const QPointF visualCenter =
        (rotatedAround(anchor, center, rotation) + rotatedAround(moved, center, rotation)) / 2.0;
    after.rect.moveCenter(visualCenter);
    return after;
}

}  // namespace markshot::ruler
