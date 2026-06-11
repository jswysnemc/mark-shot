#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

/**
 * 判断标注是否支持直线骨架点编辑。
 * @param annotation 待检测标注。
 * @return 支持时返回 true，否则返回 false。
 */
bool ShotWindow::annotationSupportsLineAnchors(const Annotation &annotation) const
{
    return annotationSupportsLineControl(annotation);
}

/**
 * 查找命中的直线骨架点索引。
 * @param annotation 待检测标注。
 * @param imagePoint 图像坐标位置。
 * @return 命中时返回标注点索引，否则返回 -1。
 */
int ShotWindow::lineAnchorPointIndexAt(const Annotation &annotation, QPointF imagePoint) const
{
    if (!annotationSupportsLineAnchors(annotation) || annotation.points.size() < 2) {
        return -1;
    }

    const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    qreal bestDistance = std::numeric_limits<qreal>::max();
    int bestIndex = -1;
    for (int i = 0; i < annotation.points.size(); ++i) {
        const qreal handleScale = i < kLineSkeletonFirstPointIndex ? 1.5 : 1.4;
        const qreal distance = QLineF(imagePoint, annotation.points.at(i)).length();
        if (distance <= imageTolerance * handleScale && distance < bestDistance) {
            bestDistance = distance;
            bestIndex = i;
        }
    }
    return bestIndex;
}

/**
 * 判断鼠标位置命中的直线骨架拖拽类型。
 * @param annotation 待检测标注。
 * @param imagePoint 图像坐标位置。
 * @return 命中的拖拽类型。
 */
ShotWindow::SelectionDrag ShotWindow::lineAnchorDragAt(const Annotation &annotation, QPointF imagePoint) const
{
    const int pointIndex = lineAnchorPointIndexAt(annotation, imagePoint);
    if (pointIndex < 0) {
        return SelectionDrag::None;
    }
    if (pointIndex == 0) {
        return SelectionDrag::LineStart;
    }
    if (pointIndex == 1) {
        return SelectionDrag::LineEnd;
    }
    return SelectionDrag::LineControl;
}

/**
 * 绘制直线骨架点手柄。
 * @param painter 当前绘制器。
 * @param annotation 待绘制标注。
 * @param center 旋转中心。
 * @param angle 旋转角度。
 * @param rotateHandles 是否按标注角度旋转手柄。
 * @return 无返回值。
 */
void ShotWindow::drawLineAnchorHandles(QPainter &painter,
                                       const Annotation &annotation,
                                       QPointF center,
                                       qreal angle,
                                       bool rotateHandles) const
{
    if (!annotationSupportsLineAnchors(annotation) || annotation.points.size() < 2) {
        return;
    }

    auto handlePoint = [this, center, angle, rotateHandles](QPointF imagePoint) {
        const QPointF widgetPoint = imageToWidget(imagePoint);
        return rotateHandles ? rotatedPoint(widgetPoint, center, angle) : widgetPoint;
    };

    const QPointF start = handlePoint(annotation.points.first());
    const QPointF end = handlePoint(annotation.points.at(1));

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(QColor(255, 255, 255), 2.0));
    painter.setBrush(QColor(251, 146, 60));
    painter.drawEllipse(QRectF(start.x() - 6.0, start.y() - 6.0, 12.0, 12.0));
    painter.drawEllipse(QRectF(end.x() - 6.0, end.y() - 6.0, 12.0, 12.0));

    painter.setPen(QPen(QColor(251, 146, 60), 2.0));
    painter.setBrush(QColor(255, 255, 255));
    for (int i = kLineSkeletonFirstPointIndex; i < annotation.points.size(); ++i) {
        const QPointF control = handlePoint(annotation.points.at(i));
        const qreal radius = i == m_lineSkeletonDragPointIndex ? 6.5 : 5.5;
        painter.drawEllipse(QRectF(control.x() - radius,
                                   control.y() - radius,
                                   radius * 2.0,
                                   radius * 2.0));
    }
    painter.restore();
}

/**
 * 更新直线骨架点拖拽结果。
 * @param imagePoint 当前鼠标图像坐标位置。
 * @return 当前拖拽属于直线骨架点时返回 true，否则返回 false。
 */
bool ShotWindow::updateLineAnchorDrag(QPointF imagePoint)
{
    if (m_annotationDrag != SelectionDrag::LineStart
        && m_annotationDrag != SelectionDrag::LineEnd
        && m_annotationDrag != SelectionDrag::LineControl) {
        return false;
    }

    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.size() != 1 || !annotationSupportsLineAnchors(m_annotationBeforeDrag)
        || m_annotationBeforeDrag.points.size() < 2) {
        return true;
    }

    Annotation *annotation = annotationById(selectedIds.first());
    if (!annotation) {
        return true;
    }

    const QRectF beforeBounds = annotationUnrotatedBounds(m_annotationBeforeDrag);
    const QPointF localPoint = beforeBounds.isEmpty()
        ? clampImagePoint(imagePoint)
        : rotatedPoint(clampImagePoint(imagePoint),
                       beforeBounds.center(),
                       -m_annotationBeforeDrag.rotationDegrees);
    while (annotation->points.size() < 2) {
        annotation->points.append(localPoint);
    }

    int targetIndex = m_lineSkeletonDragPointIndex;
    if (m_annotationDrag == SelectionDrag::LineStart) {
        targetIndex = 0;
    } else if (m_annotationDrag == SelectionDrag::LineEnd) {
        targetIndex = 1;
    } else if (targetIndex < kLineSkeletonFirstPointIndex || targetIndex >= annotation->points.size()) {
        targetIndex = kLineSkeletonFirstPointIndex;
        while (annotation->points.size() <= targetIndex) {
            annotation->points.append(annotationLineControlPoint(m_annotationBeforeDrag));
        }
    }

    if (targetIndex >= 0 && targetIndex < annotation->points.size()) {
        annotation->points[targetIndex] = clampImagePoint(localPoint);
    }
    annotation->rotationDegrees = m_annotationBeforeDrag.rotationDegrees;
    return true;
}

/**
 * 在直线类标注路径上插入新的骨架点。
 * @param annotationId 目标标注编号。
 * @param imagePoint 点击位置，使用图像坐标。
 * @return 插入成功时返回 true，否则返回 false。
 */
bool ShotWindow::insertLineSkeletonPointAt(int annotationId, QPointF imagePoint)
{
    Annotation *annotation = annotationById(annotationId);
    if (!annotation || !annotationSupportsLineAnchors(*annotation) || annotation->points.size() < 2) {
        return false;
    }

    const QRectF beforeBounds = annotationUnrotatedBounds(*annotation);
    const QPointF localPoint = beforeBounds.isEmpty()
        ? clampImagePoint(imagePoint)
        : rotatedPoint(clampImagePoint(imagePoint), beforeBounds.center(), -annotation->rotationDegrees);
    if (lineAnchorPointIndexAt(*annotation, localPoint) >= 0) {
        return false;
    }

    const qreal imageTolerance = 10.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    const qreal pathTolerance = std::max(imageTolerance, annotation->width * 0.5 + imageTolerance);
    const std::optional<LineSkeletonInsertion> insertion =
        lineSkeletonInsertionAt(annotation->points, localPoint, pathTolerance);
    if (!insertion.has_value()) {
        return false;
    }

    pushHistorySnapshot();
    annotation->points.insert(insertion->pointIndex, clampImagePoint(insertion->point));
    m_lineSkeletonDragPointIndex = insertion->pointIndex;
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
    return true;
}

/**
 * 删除当前选中的直线骨架点。
 * @return 删除成功时返回 true，否则返回 false。
 */
bool ShotWindow::removeSelectedLineSkeletonPoint()
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.size() != 1 || m_lineSkeletonDragPointIndex < kLineSkeletonFirstPointIndex) {
        return false;
    }

    Annotation *annotation = annotationById(selectedIds.first());
    if (!annotation || !annotationSupportsLineAnchors(*annotation)
        || !canRemoveLineSkeletonPoint(annotation->points, m_lineSkeletonDragPointIndex)) {
        return false;
    }

    pushHistorySnapshot();
    annotation->points.removeAt(m_lineSkeletonDragPointIndex);
    if (!canRemoveLineSkeletonPoint(annotation->points, m_lineSkeletonDragPointIndex)) {
        m_lineSkeletonDragPointIndex = annotation->points.size() > kLineSkeletonFirstPointIndex
            ? annotation->points.size() - 1
            : -1;
    }
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
    return true;
}
