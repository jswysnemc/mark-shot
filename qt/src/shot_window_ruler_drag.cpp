#include "shot_window_ruler_tool.h"

#include "shot_window_module.h"

#include "annotation/rotation_drag.h"
#include "ruler/ruler_geometry.h"

using namespace markshot::shot;
using markshot::shot::types::Tool;

namespace {

// 旋转把手的命中半径相对控制点命中半径的放大倍数，与其他标注一致
constexpr qreal kRotationHandleHitScale = 1.4;

}  // namespace

bool ShotWindowRulerTool::updateDrag(QPointF imagePoint, bool snap)
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

    // 2. 按住 Shift 拖动角度尺的边端点时，这条边从顶点出发吸附到 45 度倍数
    QPointF point = window.clampImagePoint(imagePoint);
    const bool armDrag = window.m_annotationDrag == SelectionDrag::AngleFirstArm
        || window.m_annotationDrag == SelectionDrag::AngleSecondArm;
    if (snap && armDrag && !before.points.isEmpty()) {
        point = window.clampImagePoint(constrainedLineEnd(before.points.first(), point));
    }
    *annotation = markshot::ruler::draggedRuler(before, window.m_annotationDrag, point);
    return true;
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
    return markshot::ruler::rulerDragAt(annotation, imagePoint, imageTolerance(window), window.m_tool != Tool::Ruler);
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
