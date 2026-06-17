#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

void ShotWindow::moveAnnotation(Annotation &annotation, QPointF delta) const
{
    annotation.rect.translate(delta);
    for (QPointF &point : annotation.points) {
        point = clampImagePoint(point + delta);
    }
}

void ShotWindow::transformAnnotation(Annotation &annotation, QRectF oldBounds, QRectF newBounds) const
{
    oldBounds = oldBounds.normalized();
    newBounds = newBounds.normalized();
    if (oldBounds.width() <= 0.0 || oldBounds.height() <= 0.0) {
        moveAnnotation(annotation, newBounds.topLeft() - oldBounds.topLeft());
        return;
    }

    auto mapPoint = [this, oldBounds, newBounds](QPointF point) {
        const qreal xRatio = (point.x() - oldBounds.left()) / oldBounds.width();
        const qreal yRatio = (point.y() - oldBounds.top()) / oldBounds.height();
        return clampImagePoint(QPointF(newBounds.left() + xRatio * newBounds.width(),
                                      newBounds.top() + yRatio * newBounds.height()));
    };
    const qreal scaleFactor = std::max(newBounds.width() / oldBounds.width(),
                                       newBounds.height() / oldBounds.height());

    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        return;
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Mosaic:
        annotation.rect = QRectF(mapPoint(annotation.rect.normalized().topLeft()),
                                 mapPoint(annotation.rect.normalized().bottomRight())).normalized();
        break;
    case Tool::Magnifier:
        annotation.rect = QRectF(mapPoint(annotation.rect.normalized().topLeft()),
                                 mapPoint(annotation.rect.normalized().bottomRight())).normalized();
        for (QPointF &point : annotation.points) {
            point = mapPoint(point);
        }
        break;
    case Tool::Text:
        annotation.rect = QRectF(mapPoint(annotation.rect.normalized().topLeft()),
                                 mapPoint(annotation.rect.normalized().bottomRight())).normalized();
        if (m_annotationDrag == SelectionDrag::TopLeft ||
            m_annotationDrag == SelectionDrag::BottomRight ||
            m_annotationDrag == SelectionDrag::TopRight ||
            m_annotationDrag == SelectionDrag::BottomLeft) {
            annotation.width = std::clamp((19.0 + annotation.width) * scaleFactor - 19.0, 1.0, 1000.0);
            if (!annotation.points.isEmpty()) {
                annotation.points[0] = annotation.rect.topLeft();
            }
        }
        break;
    case Tool::Pen:
    case Tool::Highlighter:
    case Tool::Line:
    case Tool::Arrow:
        for (QPointF &point : annotation.points) {
            point = mapPoint(point);
        }
        break;
    case Tool::Number:
        if (!annotation.points.isEmpty()) {
            for (QPointF &point : annotation.points) {
                point = mapPoint(point);
            }
            annotation.width = std::clamp(annotation.width * scaleFactor, kMinNumberWidth, kMaxNumberWidth);
        }
        break;
    }
}

void ShotWindow::beginAnnotationDrag(int annotationId, SelectionDrag drag, QPointF imagePoint)
{
    Annotation *annotation = annotationById(annotationId);
    if (!annotation || drag == SelectionDrag::None) {
        return;
    }
    if (!selectedAnnotationIds().contains(annotationId)) {
        setSelectedAnnotations({annotationId});
    }
    m_annotationDrag = drag;
    m_lineSkeletonDragPointIndex = -1;
    if (drag == SelectionDrag::LineStart) {
        m_lineSkeletonDragPointIndex = 0;
    } else if (drag == SelectionDrag::LineEnd) {
        m_lineSkeletonDragPointIndex = 1;
    } else if (drag == SelectionDrag::LineControl && annotationSupportsLineAnchors(*annotation)) {
        const QRectF beforeBounds = annotationUnrotatedBounds(*annotation);
        const QPointF localPoint = beforeBounds.isEmpty()
            ? clampImagePoint(imagePoint)
            : rotatedPoint(clampImagePoint(imagePoint), beforeBounds.center(), -annotation->rotationDegrees);
        m_lineSkeletonDragPointIndex = lineAnchorPointIndexAt(*annotation, localPoint);
    }
    m_annotationBeforeDrag = *annotation;
    m_annotationsBeforeDrag.clear();
    for (int id : selectedAnnotationIds()) {
        if (const Annotation *selected = annotationById(id)) {
            m_annotationsBeforeDrag.append(*selected);
        }
    }
    m_annotationBoundsBeforeDrag = selectedAnnotationsBounds();
    m_dragStart = imagePoint;
    m_dragging = true;
    m_annotationHistoryCaptured = false;
    updateCursor();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::updateAnnotationDrag(QPointF imagePoint, bool keepAspectRatio)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty() || m_annotationDrag == SelectionDrag::None) {
        return;
    }
    if (!m_annotationHistoryCaptured) {
        pushHistorySnapshot();
        m_annotationHistoryCaptured = true;
    }

    for (const Annotation &before : m_annotationsBeforeDrag) {
        if (Annotation *annotation = annotationById(before.id)) {
            *annotation = before;
        }
    }

    if (m_annotationDrag == SelectionDrag::Rotate) {
        const QPointF center = selectedIds.size() == 1
            ? annotationRotationCenter(m_annotationBeforeDrag, false)
            : m_annotationBoundsBeforeDrag.center();
        const QPointF startVector = m_dragStart - center;
        const QPointF currentVector = clampImagePoint(imagePoint) - center;
        if (QLineF(QPointF(0, 0), startVector).length() <= 0.1
            || QLineF(QPointF(0, 0), currentVector).length() <= 0.1) {
            return;
        }
        const qreal startAngle = std::atan2(startVector.y(), startVector.x());
        const qreal currentAngle = std::atan2(currentVector.y(), currentVector.x());
        const qreal deltaDegrees = (currentAngle - startAngle) * 180.0 / M_PI;
        if (selectedIds.size() == 1) {
            Annotation *annotation = annotationById(selectedIds.first());
            if (!annotation || !annotationSupportsRotation(m_annotationBeforeDrag)) {
                return;
            }
            annotation->rotationDegrees =
                normalizedRotationDegrees(m_annotationBeforeDrag.rotationDegrees + deltaDegrees);
        } else {
            for (const Annotation &before : m_annotationsBeforeDrag) {
                Annotation *annotation = annotationById(before.id);
                if (!annotation || !annotationSupportsRotation(before)) {
                    continue;
                }
                const QRectF beforeBounds = annotationUnrotatedBounds(before);
                if (beforeBounds.isEmpty()) {
                    continue;
                }
                const QPointF beforeCenter = beforeBounds.center();
                const QPointF rotatedCenter = rotatedPoint(beforeCenter, center, deltaDegrees);
                moveAnnotation(*annotation, rotatedCenter - beforeCenter);
                annotation->rotationDegrees = normalizedRotationDegrees(before.rotationDegrees + deltaDegrees);
            }
        }
    } else if (selectedIds.size() == 1 && isMagnifierResizeOrMoveDrag()) {
        // 放大镜的小框/大框 resize 与平移交互独立处理,逻辑见 shot_window_magnifier_drag.cpp
        updateMagnifierDrag(imagePoint);
    } else if (updateLineAnchorDrag(imagePoint)) {
    } else if (selectedIds.size() == 1
        && (m_annotationDrag == SelectionDrag::NumberTip
            || m_annotationDrag == SelectionDrag::NumberBubble)) {
        Annotation *annotation = annotationById(selectedIds.first());
        if (!annotation || annotation->tool != Tool::Number
            || m_annotationBeforeDrag.tool != Tool::Number
            || m_annotationBeforeDrag.points.isEmpty()) {
            return;
        }
        const QRectF beforeBounds = annotationUnrotatedBounds(m_annotationBeforeDrag);
        const QPointF localPoint = beforeBounds.isEmpty()
            ? clampImagePoint(imagePoint)
            : rotatedPoint(clampImagePoint(imagePoint),
                           beforeBounds.center(),
                           -m_annotationBeforeDrag.rotationDegrees);
        while (annotation->points.size() < 2) {
            annotation->points.append(m_annotationBeforeDrag.points.last());
        }
        if (m_annotationDrag == SelectionDrag::NumberTip) {
            annotation->points[0] = clampImagePoint(localPoint);
        } else {
            annotation->points[1] = clampImagePoint(localPoint);
        }
        annotation->rotationDegrees = m_annotationBeforeDrag.rotationDegrees;
    } else if (m_annotationDrag == SelectionDrag::Move) {
        const QRectF startBounds = m_annotationBoundsBeforeDrag;
        QPointF delta = clampImagePoint(imagePoint) - m_dragStart;
        delta.setX(std::clamp(delta.x(), -startBounds.left(), m_frozenFrame.width() - startBounds.right()));
        delta.setY(std::clamp(delta.y(), -startBounds.top(), m_frozenFrame.height() - startBounds.bottom()));
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                moveAnnotation(*annotation, delta);
            }
        }
    } else if (selectedIds.size() == 1 && annotationSupportsRotation(m_annotationBeforeDrag)) {
        Annotation *annotation = annotationById(selectedIds.first());
        if (!annotation) {
            return;
        }
        bool lockAspectRatio = keepAspectRatio || annotation->tool == Tool::Magnifier;
        const QRectF oldBounds = annotationUnrotatedBounds(m_annotationBeforeDrag);
        const QPointF localPoint =
            rotatedPoint(clampImagePoint(imagePoint),
                         oldBounds.center(),
                         -m_annotationBeforeDrag.rotationDegrees);
        const QRectF newBounds = resizedBounds(oldBounds,
                                               m_annotationDrag,
                                               localPoint,
                                               lockAspectRatio);
        transformAnnotation(*annotation, oldBounds, newBounds);
        annotation->rotationDegrees = m_annotationBeforeDrag.rotationDegrees;
    } else {
        bool lockAspectRatio = keepAspectRatio;
        if (selectedIds.size() == 1) {
            const Annotation *annotation = annotationById(selectedIds.first());
            lockAspectRatio = lockAspectRatio
                || (annotation && annotation->tool == Tool::Magnifier);
        }
        const QRectF newBounds = resizedBounds(m_annotationBoundsBeforeDrag,
                                               m_annotationDrag,
                                               imagePoint,
                                               lockAspectRatio);
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                transformAnnotation(*annotation, m_annotationBoundsBeforeDrag, newBounds);
            }
        }
    }
    update();
}

void ShotWindow::beginAnnotationSelectionBox(QPointF imagePoint)
{
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = true;
    m_dragging = true;
    m_dragStart = clampImagePoint(imagePoint);
    m_annotationSelectionBox = QRectF(m_dragStart, m_dragStart);
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::updateAnnotationSelectionBox(QPointF imagePoint)
{
    m_annotationSelectionBox = QRectF(m_dragStart, clampImagePoint(imagePoint)).normalized();
    update();
}

void ShotWindow::commitAnnotationSelectionBox()
{
    m_annotationSelectionBoxActive = false;
    setSelectedAnnotations(annotationsInRect(m_annotationSelectionBox));
    m_annotationSelectionBox = {};
    updateAnnotationPropertyPanel();
}

ShotWindow::HistorySnapshot ShotWindow::currentHistorySnapshot() const
{
    return {m_annotations, m_selectedAnnotationId, selectedAnnotationIds(), m_nextNumber, m_nextAnnotationId};
}

void ShotWindow::restoreHistorySnapshot(const HistorySnapshot &snapshot)
{
    m_annotations = snapshot.annotations;
    setSelectedAnnotations(snapshot.selectedAnnotationIds.isEmpty()
                               ? (snapshot.selectedAnnotationId.has_value() ? QVector<int>{*snapshot.selectedAnnotationId} : QVector<int>{})
                               : snapshot.selectedAnnotationIds);
    m_nextNumber = snapshot.nextNumber;
    m_nextAnnotationId = snapshot.nextAnnotationId;
    m_draft.reset();
    m_annotationDrag = SelectionDrag::None;
    m_lineSkeletonDragPointIndex = -1;
    m_annotationSelectionBoxActive = false;
    m_annotationHistoryCaptured = false;
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

/// @brief 把指定快照追加到撤销栈
/// @param snapshot 要作为撤销目标保存的历史快照
/// @return 无返回值
void ShotWindow::appendHistorySnapshot(const HistorySnapshot &snapshot)
{
    m_undoStack.append(snapshot);
    if (m_undoStack.size() > 100) {
        m_undoStack.removeFirst();
    }
    m_redoStack.clear();
}

/// @brief 把当前标注状态追加到撤销栈
/// @return 无返回值
void ShotWindow::pushHistorySnapshot()
{
    commitAnnotationWidthWheelHistory();
    appendHistorySnapshot(currentHistorySnapshot());
}

/// @brief 记录一次滚轮改宽开始前的快照,并用防抖合并后续滚轮事件
/// @param context 当前滚轮改宽上下文,包含目标工具和选中标注
/// @param snapshot 第一次改动前的历史快照
/// @return 无返回值
void ShotWindow::queueAnnotationWidthWheelHistory(int context, const HistorySnapshot &snapshot)
{
    if (m_annotationWidthWheelHistoryContext != context) {
        commitAnnotationWidthWheelHistory();
        m_annotationWidthWheelHistoryContext = context;
        m_annotationWidthWheelHistorySnapshot = snapshot;
    } else if (!m_annotationWidthWheelHistorySnapshot.has_value()) {
        m_annotationWidthWheelHistorySnapshot = snapshot;
    }

    if (m_annotationWidthWheelHistoryTimer) {
        m_annotationWidthWheelHistoryTimer->start();
    }
}

/// @brief 提交挂起的滚轮改宽历史快照
/// @return 无返回值
void ShotWindow::commitAnnotationWidthWheelHistory()
{
    if (m_annotationWidthWheelHistoryTimer) {
        m_annotationWidthWheelHistoryTimer->stop();
    }
    if (!m_annotationWidthWheelHistorySnapshot.has_value()) {
        return;
    }

    appendHistorySnapshot(*m_annotationWidthWheelHistorySnapshot);
    m_annotationWidthWheelHistorySnapshot.reset();
    m_annotationWidthWheelHistoryContext = 0;
}

void ShotWindow::undoAnnotationEdit()
{
    commitAnnotationWidthWheelHistory();
    if (m_undoStack.isEmpty()) {
        return;
    }
    const HistorySnapshot current = currentHistorySnapshot();
    const HistorySnapshot previous = m_undoStack.takeLast();
    m_redoStack.append(current);
    restoreHistorySnapshot(previous);
}

/// @brief 重做最近一次被撤销的标注编辑
/// @return 无返回值
void ShotWindow::redoAnnotation()
{
    commitAnnotationWidthWheelHistory();
    if (m_redoStack.isEmpty()) {
        return;
    }

    const HistorySnapshot current = currentHistorySnapshot();
    const HistorySnapshot next = m_redoStack.takeLast();
    m_undoStack.append(current);
    restoreHistorySnapshot(next);
}

void ShotWindow::setCurrentColor(QColor color)
{
    if (!color.isValid()) {
        return;
    }

    m_currentColor = color;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (m_tool == Tool::Select && !selectedIds.isEmpty()) {
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color = color;
            }
        }
        updateAnnotationPropertyPanel();
    }
    if (m_draft.has_value()) {
        m_draft->color = color;
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_textEditor && m_textEditor->isVisible() && !m_editingTextAnnotationId.has_value()) {
        m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(m_currentColor, m_textBackgroundColor, qRound(20.0 + m_textSize)));
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::revealSelectionInfo()
{
    m_showSelectionInfo = true;
    m_selectionInfoTimer.restart();
    QTimer::singleShot(1000, this, [this] {
        if (m_selectionDrag == SelectionDrag::None
            && m_selectionInfoTimer.isValid()
            && m_selectionInfoTimer.elapsed() >= 1000) {
            m_showSelectionInfo = false;
            update();
        }
    });
}

QRectF ShotWindow::normalizedSelection() const
{
    return m_selection.normalized().intersected(QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size())));
}

QRect ShotWindow::selectionGlobalRect() const
{
    if (!hasUsableSelection()) {
        return {};
    }

    const QRect sourceBounds(QPoint(0, 0), m_frozenFrame.size());
    const QRectF selection = normalizedSelection();
    QRect selectionRect = selection.toAlignedRect().intersected(sourceBounds);
    if (selectionRect.isEmpty()) {
        return {};
    }

    if (m_sourceGeometry.isValid() && !m_sourceGeometry.isEmpty()) {
        const QSize imageSize = m_frozenFrame.size();
        if (imageSize.width() <= 0 || imageSize.height() <= 0) {
            return {};
        }

        // Wayland captures can be in output pixels while backend geometry is
        // logical. Map the image-space selection back before follow-up captures
        // and extension geometry placeholders use it.
        selectionRect = markshot::capture::geometryFromImageRect(selectionRect,
                                                                 m_sourceGeometry,
                                                                 imageSize);
        if (selectionRect.isEmpty()) {
            return {};
        }
    }
    return selectionRect;
}

QString ShotWindow::slurpSelectionGeometry() const
{
    const QRect selectionRect = selectionGlobalRect();
    if (selectionRect.isEmpty()) {
        return {};
    }
    return slurpGeometry(selectionRect);
}

QPointF ShotWindow::widgetToImage(QPointF point) const
{
    if (m_frozenImageRect.isEmpty() || m_frozenFrame.isNull()) {
        return {};
    }

    const qreal x = (point.x() - m_frozenImageRect.left()) * m_frozenFrame.width() / m_frozenImageRect.width();
    const qreal y = (point.y() - m_frozenImageRect.top()) * m_frozenFrame.height() / m_frozenImageRect.height();
    return clampImagePoint({x, y});
}

QPointF ShotWindow::imageToWidget(QPointF point) const
{
    if (m_frozenImageRect.isEmpty() || m_frozenFrame.isNull()) {
        return {};
    }

    const qreal x = m_frozenImageRect.left() + point.x() * m_frozenImageRect.width() / m_frozenFrame.width();
    const qreal y = m_frozenImageRect.top() + point.y() * m_frozenImageRect.height() / m_frozenFrame.height();
    return {x, y};
}

QPointF ShotWindow::clampImagePoint(QPointF point) const
{
    return {
        std::clamp(point.x(), 0.0, static_cast<qreal>(std::max(0, m_frozenFrame.width() - 1))),
        std::clamp(point.y(), 0.0, static_cast<qreal>(std::max(0, m_frozenFrame.height() - 1))),
    };
}

QString ShotWindow::currentToolName() const
{
    switch (m_tool) {
    case Tool::Move:
        return QStringLiteral("Move");
    case Tool::Select:
        return QStringLiteral("Select");
    case Tool::Pen:
        return QStringLiteral("Pen");
    case Tool::Line:
        return QStringLiteral("Line");
    case Tool::Highlighter:
        return QStringLiteral("Highlighter");
    case Tool::Rectangle:
        return QStringLiteral("Rect");
    case Tool::Ellipse:
        return QStringLiteral("Ellipse");
    case Tool::Arrow:
        return QStringLiteral("Arrow");
    case Tool::Text:
        return QStringLiteral("Text");
    case Tool::Number:
        return QStringLiteral("Number");
    case Tool::Mosaic:
        return QStringLiteral("Mosaic");
    case Tool::Magnifier:
        return QStringLiteral("Magnifier");
    case Tool::Laser:
        return QStringLiteral("Laser");
    }

    return QStringLiteral("Tool");
}

ShotWindow::Tool ShotWindow::defaultEditingTool() const
{
    const Tool tool = m_fullscreenAnnotation ? m_fullscreenDefaultTool : m_defaultTool;
    if (m_fullscreenAnnotation && tool == Tool::Move) {
        return Tool::Select;
    }
    return tool;
}

QImage ShotWindow::mosaicImage(QRect sourceRect, int blockSize) const
{
    sourceRect = sourceRect.normalized().intersected(QRect(QPoint(0, 0), m_frozenFrame.size()));
    if (sourceRect.isEmpty()) {
        return {};
    }

    blockSize = std::clamp(blockSize, 2, 96);
    const QImage source = m_frozenFrame.copy(sourceRect).convertToFormat(QImage::Format_ARGB32_Premultiplied);
    QImage output(source.size(), QImage::Format_ARGB32_Premultiplied);
    output.fill(Qt::transparent);

    QPainter blockPainter(&output);
    blockPainter.setPen(Qt::NoPen);
    blockPainter.setRenderHint(QPainter::Antialiasing, false);

    for (int y = 0; y < source.height(); y += blockSize) {
        const int blockHeight = std::min(blockSize, source.height() - y);
        for (int x = 0; x < source.width(); x += blockSize) {
            const int blockWidth = std::min(blockSize, source.width() - x);
            quint64 red = 0;
            quint64 green = 0;
            quint64 blue = 0;
            quint64 alpha = 0;
            for (int py = y; py < y + blockHeight; ++py) {
                const QRgb *line = reinterpret_cast<const QRgb *>(source.constScanLine(py));
                for (int px = x; px < x + blockWidth; ++px) {
                    const QRgb pixel = line[px];
                    red += qRed(pixel);
                    green += qGreen(pixel);
                    blue += qBlue(pixel);
                    alpha += qAlpha(pixel);
                }
            }

            const int count = blockWidth * blockHeight;
            QColor average(qRound(static_cast<double>(red) / count),
                           qRound(static_cast<double>(green) / count),
                           qRound(static_cast<double>(blue) / count),
                           qRound(static_cast<double>(alpha) / count));
            blockPainter.setBrush(average);
            blockPainter.drawRect(QRect(x, y, blockWidth, blockHeight));
        }
    }

    blockPainter.end();
    return output;
}

QRectF ShotWindow::imageRectToWidget(QRectF rect) const
{
    const QPointF topLeft = imageToWidget(rect.topLeft());
    const QPointF bottomRight = imageToWidget(rect.bottomRight());
    return QRectF(topLeft, bottomRight).normalized();
}

QPointF ShotWindow::clampedMagnifierCircleCenter(QPointF center, qreal diameter) const
{
    const qreal radius = std::max<qreal>(0.0, diameter / 2.0);
    const qreal frameWidth = m_frozenFrame.width();
    const qreal frameHeight = m_frozenFrame.height();
    if (frameWidth <= diameter) {
        center.setX(frameWidth / 2.0);
    } else {
        center.setX(std::clamp(center.x(), radius, frameWidth - radius));
    }
    if (frameHeight <= diameter) {
        center.setY(frameHeight / 2.0);
    } else {
        center.setY(std::clamp(center.y(), radius, frameHeight - radius));
    }
    return center;
}

QRectF ShotWindow::magnifierCircleRect(QPointF center, qreal diameter) const
{
    const QPointF clampedCenter = clampedMagnifierCircleCenter(center, diameter);
    return QRectF(clampedCenter.x() - diameter / 2.0,
                  clampedCenter.y() - diameter / 2.0,
                  diameter,
                  diameter);
}

QRectF ShotWindow::magnifierSourceRect(const Annotation &annotation) const
{
    const QRectF lensRect = annotation.rect.normalized();
    if (lensRect.isEmpty()) {
        return {};
    }

    const qreal scale = clampedMagnifierScale(annotation.magnifierScale);
    const QPointF requestedCenter = annotation.points.isEmpty()
        ? lensRect.center()
        : annotation.points.first();

    if (annotation.magnifierShape == MagnifierShape::Rectangle) {
        const qreal sourceWidth = lensRect.width() / scale;
        const qreal sourceHeight = lensRect.height() / scale;
        QRectF sourceRect(requestedCenter.x() - sourceWidth / 2.0,
                          requestedCenter.y() - sourceHeight / 2.0,
                          sourceWidth,
                          sourceHeight);
        return clampedMagnifierRect(sourceRect);
    }

    const qreal diameter = std::min(lensRect.width(), lensRect.height()) / scale;
    return magnifierCircleRect(requestedCenter, diameter);
}

QRectF ShotWindow::clampedMagnifierRect(QRectF rect) const
{
    if (rect.isEmpty()) {
        return rect;
    }
    const qreal frameWidth = m_frozenFrame.width();
    const qreal frameHeight = m_frozenFrame.height();
    qreal x = rect.x();
    qreal y = rect.y();
    qreal w = rect.width();
    qreal h = rect.height();
    if (w >= frameWidth) {
        x = 0.0;
        w = frameWidth;
    } else {
        x = std::clamp(x, 0.0, std::max<qreal>(0.0, frameWidth - w));
    }
    if (h >= frameHeight) {
        y = 0.0;
        h = frameHeight;
    } else {
        y = std::clamp(y, 0.0, std::max<qreal>(0.0, frameHeight - h));
    }
    return QRectF(x, y, w, h);
}

QPainterPath ShotWindow::magnifierLensPath(const Annotation &annotation) const
{
    QPainterPath path;
    const QRectF lensRect = annotation.rect.normalized();
    if (lensRect.isEmpty()) {
        return path;
    }
    if (annotation.magnifierShape == MagnifierShape::Rectangle) {
        path.addRect(lensRect);
    } else {
        path.addEllipse(lensRect);
    }
    return path;
}

QPainterPath ShotWindow::magnifierSourcePath(const Annotation &annotation) const
{
    QPainterPath path;
    const QRectF sourceRect = magnifierSourceRect(annotation);
    if (sourceRect.isEmpty()) {
        return path;
    }
    if (annotation.magnifierShape == MagnifierShape::Rectangle) {
        path.addRect(sourceRect);
    } else {
        path.addEllipse(sourceRect);
    }
    return path;
}
