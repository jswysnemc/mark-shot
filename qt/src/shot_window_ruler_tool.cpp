#include "shot_window_ruler_tool.h"

#include "shot_window_module.h"

#include "ruler/angle_draft.h"
#include "ruler/ruler_geometry.h"

using namespace markshot::shot;
using markshot::ruler::AngleDraft;
using markshot::ruler::AngleDraftStep;
using markshot::shot::types::Mode;
using markshot::shot::types::Tool;

namespace {

// 角度尺一条边的最短长度，屏幕像素；短于它的松开视为单击
constexpr qreal kMinimumAngleArmPixels = 4.0;

}  // namespace

ShotWindowRulerTool::ShotWindowRulerTool(ShotWindow &window)
    : m_window(window)
{
}

qreal ShotWindowRulerTool::imageTolerance(const ShotWindow &window)
{
    return 8.0 * window.m_frozenFrame.width() / std::max<qreal>(1.0, window.m_frozenImageRect.width());
}

/** 判断当前是否处于编辑阶段的尺子工具，无参数，返回判断结果。 */
bool ShotWindowRulerTool::active() const
{
    return m_window.m_mode == Mode::Editing && m_window.m_tool == Tool::Ruler;
}

/** 计算角度尺一条边的最短长度，无参数，返回图像像素长度。 */
qreal ShotWindowRulerTool::minimumAngleArm() const
{
    return kMinimumAngleArmPixels * imageTolerance(m_window) / 8.0;
}

/**
 * 计算角度尺草稿下一点的位置：按住 Shift 时从顶点出发的边吸附到 45 度倍数。
 * @param points 草稿点列表，为空时表示正要放下顶点。
 * @param imagePoint 指针位置，图像像素坐标。
 * @param modifiers 按住的修饰键。
 * @return 限制在截图范围内的位置，图像像素坐标。
 */
QPointF ShotWindowRulerTool::anglePoint(const QVector<QPointF> &points, QPointF imagePoint,
                                        Qt::KeyboardModifiers modifiers) const
{
    const QPointF point = m_window.clampImagePoint(imagePoint);
    if (points.isEmpty() || !modifiers.testFlag(Qt::ShiftModifier)) {
        return point;
    }
    return m_window.clampImagePoint(constrainedLineEnd(points.first(), point));
}

/**
 * 查找尺子工具下命中的已有尺子：选中的尺子最先，旋转把手和角度尺选中框只在选中时命中；
 * 其余尺子从上到下，内部不算命中。
 * @param imagePoint 指针位置，图像像素坐标。
 * @return 命中的尺子编号和调整方式，未命中时为空。
 */
std::optional<std::pair<int, ShotWindowRulerTool::SelectionDrag>> ShotWindowRulerTool::hitAt(QPointF imagePoint) const
{
    // 1. 选中的尺子画出了旋转把手，角度尺还画出了选中框，先按选中尺子判断
    const QVector<int> selected = m_window.selectedAnnotationIds();
    if (selected.size() == 1) {
        const Annotation *annotation = m_window.annotationById(selected.first());
        if (annotation && annotation->tool == Tool::Ruler) {
            const SelectionDrag drag = dragAt(m_window, *annotation, imagePoint);
            if (drag != SelectionDrag::None) {
                return std::make_pair(annotation->id, drag);
            }
        }
    }
    // 2. 其余尺子从上到下只按控制点、边框和边命中
    const qreal tolerance = imageTolerance(m_window);
    for (int i = m_window.m_annotations.size() - 1; i >= 0; --i) {
        const Annotation &annotation = m_window.m_annotations.at(i);
        if (annotation.tool != Tool::Ruler) {
            continue;
        }
        const SelectionDrag drag = markshot::ruler::rulerDragAt(annotation, imagePoint, tolerance, false);
        if (drag != SelectionDrag::None) {
            return std::make_pair(annotation.id, drag);
        }
    }
    return std::nullopt;
}

bool ShotWindowRulerTool::press(QPointF widgetPoint, QPointF imagePoint, Qt::KeyboardModifiers modifiers)
{
    if (!active()) {
        return false;
    }
    ShotWindow &window = m_window;
    const QPointF point = window.clampImagePoint(imagePoint);

    // 1. 角度尺等待定点时，按下即放下跟随点，按住拖动可继续微调
    if (anglePending()) {
        AngleDraft::press(window.m_draft->points, anglePoint(window.m_draft->points, imagePoint, modifiers));
        window.m_dragging = true;
        window.update();
        return true;
    }

    // 2. 选中尺子的删除按钮
    if (window.selectedAnnotationDeleteButtonRect().contains(widgetPoint)) {
        window.deleteSelectedAnnotation();
        return true;
    }
    if (!window.m_frozenImageRect.contains(widgetPoint)) {
        return true;
    }

    // 3. 命中选中尺子的旋转把手、角度尺选中框，或已有尺子的控制点、边框时调整这把尺子
    if (const auto hit = hitAt(point)) {
        window.beginAnnotationDrag(hit->first, hit->second, point);
        return true;
    }

    // 4. 其余位置开始新的测量，尺子内部也可以再量一把
    window.setSelectedAnnotations({});
    Annotation annotation;
    annotation.tool = Tool::Ruler;
    annotation.rulerMode = window.m_ruler.mode;
    annotation.color = window.m_currentColor;
    annotation.width = window.m_ruler.width;
    if (annotation.rulerMode == RulerMode::Angle) {
        AngleDraft::press(annotation.points, point);
    } else {
        annotation.rect = QRectF(point, point);
    }
    window.m_dragStart = point;
    window.m_dragging = true;
    window.m_draft = annotation;
    window.updateAnnotationPropertyPanel();
    window.update();
    return true;
}

bool ShotWindowRulerTool::move(QPointF widgetPoint, QPointF imagePoint, Qt::KeyboardModifiers modifiers)
{
    if (!active()) {
        return false;
    }
    ShotWindow &window = m_window;
    const QPointF point = window.clampImagePoint(imagePoint);

    // 1. 悬停十字线跟随截图上的指针，拖动中即使移出截图也保持
    const bool overCanvas = window.m_frozenImageRect.contains(widgetPoint) && !window.mouseOverUiWidget(widgetPoint);
    window.m_ruler.hoverValid = overCanvas || window.m_dragging;
    if (window.m_ruler.hoverValid) {
        window.m_ruler.hoverImagePoint = point;
    }

    // 2. 拖动已有尺子的控制点、旋转把手或整把尺子
    if (window.m_dragging && window.m_annotationDrag != SelectionDrag::None) {
        window.updateAnnotationDrag(imagePoint, false, modifiers.testFlag(Qt::ShiftModifier));
        window.updateAnnotationPropertyPanelGeometry();
        window.update();
        return true;
    }

    // 3. 草稿跟随指针：长度尺拉开测量框，角度尺移动跟随点
    if (window.m_draft.has_value() && window.m_draft->tool == Tool::Ruler && (window.m_dragging || anglePending())) {
        if (window.m_draft->rulerMode == RulerMode::Angle) {
            AngleDraft::follow(window.m_draft->points, anglePoint(window.m_draft->points, imagePoint, modifiers));
        } else {
            window.m_draft->rect = normalizedRect(window.m_dragStart, point);
        }
    }
    window.update();
    return true;
}

bool ShotWindowRulerTool::release(QPointF imagePoint, Qt::KeyboardModifiers modifiers)
{
    if (!active()) {
        return false;
    }
    ShotWindow &window = m_window;

    // 1. 结束控制点或整把尺子的拖动
    if (window.m_annotationDrag != SelectionDrag::None) {
        window.m_annotationDrag = SelectionDrag::None;
        window.m_annotationHistoryCaptured = false;
        window.updateAnnotationPropertyPanel();
        window.update();
        return true;
    }
    if (!window.m_draft.has_value() || window.m_draft->tool != Tool::Ruler) {
        return true;
    }

    // 2. 角度尺定下一点后继续定点，三点齐全才提交；长度尺松开即提交
    const QPointF point = window.clampImagePoint(imagePoint);
    if (window.m_draft->rulerMode == RulerMode::Angle) {
        const QPointF anglePosition = anglePoint(window.m_draft->points, imagePoint, modifiers);
        if (AngleDraft::release(window.m_draft->points, anglePosition, minimumAngleArm()) == AngleDraftStep::Continue) {
            window.update();
            return true;
        }
    } else {
        window.m_draft->rect = normalizedRect(window.m_dragStart, point);
    }

    // 3. 只单击没有量到内容时不留下尺子
    if (!markshot::ruler::rulerMeasures(*window.m_draft)) {
        window.m_draft.reset();
        window.update();
        return true;
    }
    window.commitDraft();
    return true;
}

bool ShotWindowRulerTool::anglePending() const
{
    const ShotWindow &window = m_window;
    return !window.m_dragging && window.m_draft.has_value() && window.m_draft->tool == Tool::Ruler
        && window.m_draft->rulerMode == RulerMode::Angle;
}

bool ShotWindowRulerTool::cancelDraft()
{
    if (!m_window.m_draft.has_value() || m_window.m_draft->tool != Tool::Ruler) {
        return false;
    }
    m_window.cancelPointerInteraction();
    return true;
}

void ShotWindowRulerTool::clearHover()
{
    if (m_window.m_ruler.hoverValid) {
        m_window.m_ruler.hoverValid = false;
        m_window.update();
    }
}
