#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

// 在 normalized rect 上按指定把手 drag 计算新矩形
// minSize 限制最小边长;keepSquare=true 时强制为正方形(用于圆形放大镜)
QRectF ShotWindow::magnifierResizeRectWithHandle(const QRectF &before,
                                                  SelectionDrag handle,
                                                  QPointF point,
                                                  qreal minSize,
                                                  bool keepSquare)
{
    if (keepSquare && before.width() > 0.0 && before.height() > 0.0) {
        QPointF anchor;
        qreal xSign = 0.0;
        qreal ySign = 0.0;
        switch (handle) {
        case SelectionDrag::TopLeft:
            anchor = before.bottomRight(); xSign = -1.0; ySign = -1.0; break;
        case SelectionDrag::TopRight:
            anchor = before.bottomLeft(); xSign = 1.0; ySign = -1.0; break;
        case SelectionDrag::BottomLeft:
            anchor = before.topRight(); xSign = -1.0; ySign = 1.0; break;
        case SelectionDrag::BottomRight:
            anchor = before.topLeft(); xSign = 1.0; ySign = 1.0; break;
        case SelectionDrag::Left:
            anchor = QPointF(before.right(), before.center().y());
            xSign = -1.0; break;
        case SelectionDrag::Right:
            anchor = QPointF(before.left(), before.center().y());
            xSign = 1.0; break;
        case SelectionDrag::Top:
            anchor = QPointF(before.center().x(), before.bottom());
            ySign = -1.0; break;
        case SelectionDrag::Bottom:
            anchor = QPointF(before.center().x(), before.top());
            ySign = 1.0; break;
        default:
            return before;
        }
        switch (handle) {
        case SelectionDrag::TopLeft:
        case SelectionDrag::TopRight:
        case SelectionDrag::BottomLeft:
        case SelectionDrag::BottomRight: {
            const qreal dx = std::abs(point.x() - anchor.x());
            const qreal dy = std::abs(point.y() - anchor.y());
            const qreal side = std::max({dx, dy, minSize});
            return QRectF(QPointF(anchor.x(), anchor.y()),
                          QPointF(anchor.x() + xSign * side,
                                  anchor.y() + ySign * side))
                .normalized();
        }
        case SelectionDrag::Left:
        case SelectionDrag::Right: {
            const qreal dx = std::abs(point.x() - anchor.x());
            const qreal side = std::max(dx, minSize);
            const QPointF center(anchor.x() + xSign * side / 2.0, anchor.y());
            return QRectF(center.x() - side / 2.0,
                          center.y() - side / 2.0,
                          side, side);
        }
        case SelectionDrag::Top:
        case SelectionDrag::Bottom: {
            const qreal dy = std::abs(point.y() - anchor.y());
            const qreal side = std::max(dy, minSize);
            const QPointF center(anchor.x(), anchor.y() + ySign * side / 2.0);
            return QRectF(center.x() - side / 2.0,
                          center.y() - side / 2.0,
                          side, side);
        }
        default:
            return before;
        }
    }

    QRectF r = before;
    switch (handle) {
    case SelectionDrag::Left:
        r.setLeft(std::min(point.x(), r.right() - minSize));
        break;
    case SelectionDrag::Right:
        r.setRight(std::max(point.x(), r.left() + minSize));
        break;
    case SelectionDrag::Top:
        r.setTop(std::min(point.y(), r.bottom() - minSize));
        break;
    case SelectionDrag::Bottom:
        r.setBottom(std::max(point.y(), r.top() + minSize));
        break;
    case SelectionDrag::TopLeft:
        r.setLeft(std::min(point.x(), r.right() - minSize));
        r.setTop(std::min(point.y(), r.bottom() - minSize));
        break;
    case SelectionDrag::TopRight:
        r.setRight(std::max(point.x(), r.left() + minSize));
        r.setTop(std::min(point.y(), r.bottom() - minSize));
        break;
    case SelectionDrag::BottomLeft:
        r.setLeft(std::min(point.x(), r.right() - minSize));
        r.setBottom(std::max(point.y(), r.top() + minSize));
        break;
    case SelectionDrag::BottomRight:
        r.setRight(std::max(point.x(), r.left() + minSize));
        r.setBottom(std::max(point.y(), r.top() + minSize));
        break;
    default:
        break;
    }
    return r.normalized();
}

ShotWindow::SelectionDrag ShotWindow::magnifierSourceHandleToGenericHandle(SelectionDrag handle)
{
    switch (handle) {
    case SelectionDrag::MagnifierSourceLeft: return SelectionDrag::Left;
    case SelectionDrag::MagnifierSourceRight: return SelectionDrag::Right;
    case SelectionDrag::MagnifierSourceTop: return SelectionDrag::Top;
    case SelectionDrag::MagnifierSourceBottom: return SelectionDrag::Bottom;
    case SelectionDrag::MagnifierSourceTopLeft: return SelectionDrag::TopLeft;
    case SelectionDrag::MagnifierSourceTopRight: return SelectionDrag::TopRight;
    case SelectionDrag::MagnifierSourceBottomLeft: return SelectionDrag::BottomLeft;
    case SelectionDrag::MagnifierSourceBottomRight: return SelectionDrag::BottomRight;
    default: return handle;
    }
}

bool ShotWindow::isMagnifierLensCornerHandle(SelectionDrag drag)
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
        return true;
    default:
        return false;
    }
}

bool ShotWindow::isMagnifierSourceCornerHandle(SelectionDrag drag)
{
    switch (drag) {
    case SelectionDrag::MagnifierSourceLeft:
    case SelectionDrag::MagnifierSourceRight:
    case SelectionDrag::MagnifierSourceTop:
    case SelectionDrag::MagnifierSourceBottom:
    case SelectionDrag::MagnifierSourceTopLeft:
    case SelectionDrag::MagnifierSourceTopRight:
    case SelectionDrag::MagnifierSourceBottomLeft:
    case SelectionDrag::MagnifierSourceBottomRight:
        return true;
    default:
        return false;
    }
}

/// @brief 判断当前是否处于放大镜的 resize/平移交互
/// @return 命中 lens/source 的任意把手或区域平移时返回 true
bool ShotWindow::isMagnifierResizeOrMoveDrag() const
{
    if (m_annotationBeforeDrag.tool != Tool::Magnifier) {
        return false;
    }
    if (m_annotationDrag == SelectionDrag::MagnifierSource
        || m_annotationDrag == SelectionDrag::MagnifierLens) {
        return true;
    }
    return isMagnifierLensCornerHandle(m_annotationDrag)
        || isMagnifierSourceCornerHandle(m_annotationDrag);
}

/// @brief 在按比例联动下更新放大镜小框/大框的几何
/// @param imagePoint 当前指针在图像坐标系中的位置
void ShotWindow::updateMagnifierDrag(QPointF imagePoint)
{
    // 1. 校验当前选中的标注确为放大镜
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.size() != 1) {
        return;
    }
    Annotation *annotation = annotationById(selectedIds.first());
    if (!annotation || annotation->tool != Tool::Magnifier
        || m_annotationBeforeDrag.tool != Tool::Magnifier) {
        return;
    }

    const QRectF beforeLensRect = m_annotationBeforeDrag.rect.normalized();
    if (beforeLensRect.isEmpty()) {
        return;
    }

    // 2. 准备公共上下文
    const QRectF beforeSourceRect = magnifierSourceRect(m_annotationBeforeDrag);
    const qreal scale = clampedMagnifierScale(m_annotationBeforeDrag.magnifierScale);
    const QPointF clampedPoint = clampImagePoint(imagePoint);
    const QPointF delta = clampedPoint - m_dragStart;
    const bool isCircular = annotation->magnifierShape == MagnifierShape::Circle;

    auto ensureTwoPoints = [&](QPointF sourceCenter, QPointF lensCenter) {
        if (annotation->points.isEmpty()) {
            annotation->points.append(sourceCenter);
        } else {
            annotation->points[0] = sourceCenter;
        }
        if (annotation->points.size() < 2) {
            annotation->points.append(lensCenter);
        } else {
            annotation->points[1] = lensCenter;
        }
    };

    if (isMagnifierLensCornerHandle(m_annotationDrag)) {
        // 3a. resize lens 大框:scale 不变,source 大小由 magnifierSourceRect 自动联动
        const qreal minLensSide = kMinMagnifierDiameter;
        const QRectF newLens = clampedMagnifierRect(
            magnifierResizeRectWithHandle(beforeLensRect, m_annotationDrag, clampedPoint,
                                          minLensSide, isCircular));
        annotation->rect = newLens;
        QPointF sourceCenter = beforeSourceRect.center();
        if (isCircular) {
            const qreal sourceDiameter = std::min(newLens.width(), newLens.height()) / scale;
            sourceCenter = clampedMagnifierCircleCenter(sourceCenter, sourceDiameter);
        } else {
            const qreal sourceWidth = newLens.width() / scale;
            const qreal sourceHeight = newLens.height() / scale;
            sourceCenter = clampedMagnifierRect(
                               QRectF(sourceCenter.x() - sourceWidth / 2.0,
                                      sourceCenter.y() - sourceHeight / 2.0,
                                      sourceWidth, sourceHeight))
                               .center();
        }
        ensureTwoPoints(sourceCenter, newLens.center());
        return;
    }

    if (isMagnifierSourceCornerHandle(m_annotationDrag)) {
        // 3b. resize source 小框:scale 不变,lens 大小 = source * scale,lens 中心保持 before
        const qreal minSourceSide = kMinMagnifierDiameter / scale;
        const SelectionDrag generic = magnifierSourceHandleToGenericHandle(m_annotationDrag);
        const QRectF newSourceRaw =
            magnifierResizeRectWithHandle(beforeSourceRect, generic, clampedPoint,
                                          minSourceSide, isCircular);
        QRectF newSource;
        if (isCircular) {
            const qreal sourceDiameter = std::min(newSourceRaw.width(), newSourceRaw.height());
            const QPointF clampedCenter =
                clampedMagnifierCircleCenter(newSourceRaw.center(), sourceDiameter);
            newSource = QRectF(clampedCenter.x() - sourceDiameter / 2.0,
                               clampedCenter.y() - sourceDiameter / 2.0,
                               sourceDiameter, sourceDiameter);
        } else {
            newSource = clampedMagnifierRect(newSourceRaw);
        }
        const QPointF lensCenter = beforeLensRect.center();
        QRectF newLens;
        if (isCircular) {
            const qreal lensDiameter = newSource.width() * scale;
            newLens = magnifierCircleRect(lensCenter, lensDiameter);
        } else {
            const qreal lensWidth = newSource.width() * scale;
            const qreal lensHeight = newSource.height() * scale;
            newLens = clampedMagnifierRect(
                QRectF(lensCenter.x() - lensWidth / 2.0,
                       lensCenter.y() - lensHeight / 2.0,
                       lensWidth, lensHeight));
        }
        annotation->rect = newLens;
        ensureTwoPoints(newSource.center(), newLens.center());
        return;
    }

    if (m_annotationDrag == SelectionDrag::MagnifierSource) {
        // 3c. 平移 source 中心,lens 不动,scale 不变 -> source 大小不变
        QPointF sourceCenter = beforeSourceRect.center() + delta;
        if (isCircular) {
            const qreal lensDiameter =
                std::min(beforeLensRect.width(), beforeLensRect.height());
            const qreal sourceDiameter = lensDiameter / scale;
            sourceCenter = clampedMagnifierCircleCenter(sourceCenter, sourceDiameter);
        } else {
            const qreal sourceWidth = beforeLensRect.width() / scale;
            const qreal sourceHeight = beforeLensRect.height() / scale;
            sourceCenter = clampedMagnifierRect(
                               QRectF(sourceCenter.x() - sourceWidth / 2.0,
                                      sourceCenter.y() - sourceHeight / 2.0,
                                      sourceWidth, sourceHeight))
                               .center();
        }
        ensureTwoPoints(sourceCenter, beforeLensRect.center());
        return;
    }

    if (m_annotationDrag == SelectionDrag::MagnifierLens) {
        // 3d. 平移 lens 整体,lens 大小不变,source 大小不变,source 中心保持 before
        QRectF newLens;
        if (isCircular) {
            const qreal lensDiameter =
                std::min(beforeLensRect.width(), beforeLensRect.height());
            newLens = magnifierCircleRect(beforeLensRect.center() + delta, lensDiameter);
        } else {
            newLens = clampedMagnifierRect(
                QRectF(beforeLensRect.x() + delta.x(),
                       beforeLensRect.y() + delta.y(),
                       beforeLensRect.width(),
                       beforeLensRect.height()));
        }
        annotation->rect = newLens;
        ensureTwoPoints(beforeSourceRect.center(), newLens.center());
        return;
    }
}
