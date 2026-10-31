#include "shot_window_ruler_tool.h"

#include "shot_window_module.h"

#include "shot_window_ruler_painter.h"

#include "annotation/rotation_drag.h"
#include "ruler/ruler_geometry.h"

using namespace markshot::shot;
using markshot::shot::types::Tool;

namespace {

// 旋转把手的命中半径相对控制点命中半径的放大倍数，与其他标注一致
constexpr qreal kRotationHandleHitScale = 1.4;

}  // namespace

bool ShotWindowRulerTool::updateDrag(QPointF imagePoint, bool keepAspectRatio, bool snap)
{
    ShotWindow &window = m_window;
    const Annotation &before = window.m_annotationBeforeDrag;
    if (before.tool != Tool::Ruler || window.selectedAnnotationIds().size() != 1) {
        return false;
    }
    Annotation *annotation = window.annotationById(before.id);
    if (!annotation) {
        return false;
    }

    // 1. 角度尺的旋转改写三个定点，长度尺的旋转交给通用旋转
    if (window.m_annotationDrag == SelectionDrag::Rotate) {
        if (before.rulerMode != RulerMode::Angle) {
            return false;
        }
        rotateAngle(*annotation, imagePoint, snap);
        return true;
    }
    if (!markshot::ruler::isRulerHandleDrag(window.m_annotationDrag)) {
        return false;
    }

    // 2. 角度尺选中框的四角和四边缩放三个定点
    const bool pointDrag = window.m_annotationDrag == SelectionDrag::AngleVertex
        || window.m_annotationDrag == SelectionDrag::AngleFirstArm
        || window.m_annotationDrag == SelectionDrag::AngleSecondArm;
    if (before.rulerMode == RulerMode::Angle && !pointDrag) {
        resizeAngle(*annotation, imagePoint, keepAspectRatio);
        return true;
    }

    // 3. 按住 Shift 拖动角度尺的边端点时，这条边从顶点出发吸附到 45 度倍数
    QPointF point = window.clampImagePoint(imagePoint);
    const bool armDrag = pointDrag && window.m_annotationDrag != SelectionDrag::AngleVertex;
    if (snap && armDrag && !before.points.isEmpty()) {
        point = window.clampImagePoint(constrainedLineEnd(before.points.first(), point));
    }
    *annotation = markshot::ruler::draggedRuler(before, window.m_annotationDrag, point);
    return true;
}

/**
 * 拖动角度尺选中框的角或边缩放三个定点：选中框跟着指针，定点按外接矩形内的相对位置换算。
 * @param annotation 待缩放的角度尺，原地修改。
 * @param imagePoint 指针位置，图像像素坐标。
 * @param keepAspectRatio 为 true 时宽高等比缩放，夹角不变。
 * @return 无返回值。
 */
void ShotWindowRulerTool::resizeAngle(Annotation &annotation, QPointF imagePoint, bool keepAspectRatio) const
{
    const ShotWindow &window = m_window;
    const Annotation &before = window.m_annotationBeforeDrag;
    const SelectionDrag drag = window.m_annotationDrag;
    const QRectF oldBounds = markshot::ruler::rulerBounds(before);
    const QRectF frame = ShotWindowRulerPainter(window).angleFrame(before);

    // 1. 指针拖的是选中框的边，扣掉留白换算成外接矩形的边，外接矩形的边缘不会跳到指针上
    const qreal padding = oldBounds.left() - frame.left();
    QPointF point = imagePoint;
    if (drag == SelectionDrag::Left || drag == SelectionDrag::TopLeft || drag == SelectionDrag::BottomLeft) {
        point.rx() += padding;
    } else if (drag == SelectionDrag::Right || drag == SelectionDrag::TopRight || drag == SelectionDrag::BottomRight) {
        point.rx() -= padding;
    }
    if (drag == SelectionDrag::Top || drag == SelectionDrag::TopLeft || drag == SelectionDrag::TopRight) {
        point.ry() += padding;
    } else if (drag == SelectionDrag::Bottom || drag == SelectionDrag::BottomLeft || drag == SelectionDrag::BottomRight) {
        point.ry() -= padding;
    }

    // 2. 三点共线时外接矩形在一个方向上没有尺寸，这个方向保持不变，等比缩放也随之关闭
    const bool flatX = oldBounds.width() <= 1.0;
    const bool flatY = oldBounds.height() <= 1.0;
    QRectF newBounds = window.resizedBounds(oldBounds, drag, point, keepAspectRatio && !flatX && !flatY);
    if (flatX) {
        newBounds.setLeft(oldBounds.left());
        newBounds.setRight(oldBounds.right());
    }
    if (flatY) {
        newBounds.setTop(oldBounds.top());
        newBounds.setBottom(oldBounds.bottom());
    }
    window.transformAnnotation(annotation, oldBounds, newBounds);
}

/**
 * 拖动旋转把手转动单把角度尺：三个定点绕外接矩形中心转动；按住 Shift 时第一条边吸附到 45 度倍数。
 * @param annotation 待旋转的角度尺，原地修改。
 * @param imagePoint 指针位置，图像像素坐标。
 * @param snap 是否吸附到 45 度倍数。
 * @return 无返回值。
 */
void ShotWindowRulerTool::rotateAngle(Annotation &annotation, QPointF imagePoint, bool snap) const
{
    const ShotWindow &window = m_window;
    const Annotation &before = window.m_annotationBeforeDrag;
    const QPointF center = markshot::ruler::rulerBounds(before).center();
    std::optional<qreal> degrees =
        markshot::annotation::rotationDragDegrees(center, window.m_dragStart, window.clampImagePoint(imagePoint));
    if (!degrees) {
        return;
    }
    if (snap) {
        // 1. 角度尺没有旋转角度，以第一条边的方向为准吸附
        const qreal arm = markshot::ruler::angleFirstArmDegrees(before);
        *degrees = markshot::annotation::snappedRotationDegrees(arm + *degrees) - arm;
    }
    markshot::ruler::rotateAnglePoints(annotation, center, *degrees);
    for (QPointF &point : annotation.points) {
        point = window.clampImagePoint(point);
    }
}

bool ShotWindowRulerTool::rotateInGroup(Annotation &annotation, const Annotation &before, QPointF center,
                                        qreal degrees) const
{
    if (before.tool != Tool::Ruler || before.rulerMode != RulerMode::Angle) {
        return false;
    }
    annotation.points = before.points;
    markshot::ruler::rotateAnglePoints(annotation, center, degrees);
    for (QPointF &point : annotation.points) {
        point = m_window.clampImagePoint(point);
    }
    return true;
}

/**
 * 判断指针是否落在尺子的旋转把手上。
 * @param window 截图窗口。
 * @param annotation 尺子标注。
 * @param imagePoint 指针位置，图像像素坐标。
 * @return 落在旋转把手上时返回 true。
 */
bool ShotWindowRulerTool::rotationHandleHit(const ShotWindow &window, const Annotation &annotation,
                                            QPointF imagePoint)
{
    const QPointF handle = window.annotationRotationHandlePoint(annotation, false);
    return !handle.isNull()
        && QLineF(imagePoint, handle).length() <= imageTolerance(window) * kRotationHandleHitScale;
}

ShotWindowRulerTool::SelectionDrag ShotWindowRulerTool::dragAt(const ShotWindow &window,
                                                              const Annotation &annotation,
                                                              QPointF imagePoint)
{
    // 1. 调用方只对选中的尺子取调整方式，旋转把手优先
    if (rotationHandleHit(window, annotation, imagePoint)) {
        return SelectionDrag::Rotate;
    }
    const SelectionDrag drag =
        markshot::ruler::rulerDragAt(annotation, imagePoint, imageTolerance(window), window.m_tool != Tool::Ruler);
    if (drag != SelectionDrag::None || annotation.rulerMode != RulerMode::Angle) {
        return drag;
    }
    // 2. 选中的角度尺画出了选中框：框的四角和四边缩放，框内任意位置整体拖动
    return window.annotationBoundsDragAt(imagePoint, ShotWindowRulerPainter(window).angleFrame(annotation));
}

ShotWindowRulerTool::SelectionDrag ShotWindowRulerTool::hoverDragAt(QPointF imagePoint, qreal *rotationDegrees) const
{
    if (anglePending()) {
        return SelectionDrag::None;
    }
    const auto hit = hitAt(imagePoint);
    if (!hit) {
        return SelectionDrag::None;
    }
    if (rotationDegrees) {
        const Annotation *annotation = m_window.annotationById(hit->first);
        *rotationDegrees = annotation && m_window.annotationSupportsRotation(*annotation)
            ? annotation->rotationDegrees
            : 0.0;
    }
    return hit->second;
}
