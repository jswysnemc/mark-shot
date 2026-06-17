#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

void ShotWindow::drawSelectedAnnotationFrame(QPainter &painter) const
{
    if (m_imageNavigationEnabled && m_tool == Tool::Select && m_imageSelected && selectedAnnotationIds().isEmpty()) {
        painter.save();
        painter.setPen(QPen(QColor(45, 212, 191), 2.0, Qt::DashLine));
        painter.setBrush(Qt::NoBrush);
        painter.drawRoundedRect(m_frozenImageRect.adjusted(2.0, 2.0, -2.0, -2.0), 6.0, 6.0);
        painter.restore();
        return;
    }

    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty()) {
        return;
    }

    const QRectF bounds = imageRectToWidget(selectedAnnotationsBounds());
    if (bounds.isEmpty()) {
        return;
    }

    painter.save();
    const Annotation *singleSelectedAnnotation = selectedIds.size() == 1
        ? annotationById(selectedIds.first())
        : nullptr;
    const bool rotatedSingleSelection =
        singleSelectedAnnotation && annotationSupportsRotation(*singleSelectedAnnotation);
    auto drawNumberPointHandles = [this, &painter](const Annotation &annotation,
                                                   QPointF center,
                                                   qreal angle,
                                                   bool rotateHandles) {
        if (annotation.tool != Tool::Number || annotation.points.isEmpty()) {
            return;
        }
        auto handlePoint = [this, center, angle, rotateHandles](QPointF imagePoint) {
            QPointF widgetPoint = imageToWidget(imagePoint);
            return rotateHandles ? rotatedPoint(widgetPoint, center, angle) : widgetPoint;
        };
        const QPointF tip = handlePoint(annotation.points.first());
        const QPointF bubble = handlePoint(annotation.points.size() >= 2 ? annotation.points.last() : annotation.points.first());
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setPen(QPen(QColor(255, 255, 255), 2.0));
        painter.setBrush(QColor(251, 146, 60));
        painter.drawEllipse(QRectF(bubble.x() - 6.0,
                                   bubble.y() - 6.0,
                                   12.0,
                                   12.0));
        painter.setPen(QPen(QColor(251, 146, 60), 2.0));
        painter.setBrush(QColor(255, 255, 255));
        painter.drawEllipse(QRectF(tip.x() - 5.5,
                                   tip.y() - 5.5,
                                   11.0,
                                   11.0));
        painter.restore();
    };
    if (selectedIds.size() > 1) {
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(QColor(255, 255, 255, 190), 3.0, Qt::DashLine));
        for (int id : selectedIds) {
            if (const Annotation *annotation = annotationById(id)) {
                painter.drawRoundedRect(imageRectToWidget(annotationBounds(*annotation)), 3.0, 3.0);
            }
        }
        painter.setPen(QPen(QColor(251, 146, 60, 170), 1.0, Qt::DashLine));
        for (int id : selectedIds) {
            if (const Annotation *annotation = annotationById(id)) {
                painter.drawRoundedRect(imageRectToWidget(annotationBounds(*annotation)), 3.0, 3.0);
            }
        }
    }
    if (rotatedSingleSelection) {
        const QRectF localBounds = imageRectToWidget(annotationUnrotatedBounds(*singleSelectedAnnotation));
        const QPointF center = localBounds.center();
        const qreal angle = singleSelectedAnnotation->rotationDegrees;
        const QVector<QPointF> corners = {
            rotatedPoint(localBounds.topLeft(), center, angle),
            rotatedPoint(localBounds.topRight(), center, angle),
            rotatedPoint(localBounds.bottomRight(), center, angle),
            rotatedPoint(localBounds.bottomLeft(), center, angle),
        };

        QPolygonF frame;
        for (const QPointF &corner : corners) {
            frame.append(corner);
        }
        painter.setPen(QPen(QColor(251, 146, 60), 2.0, Qt::DashLine));
        painter.setBrush(Qt::NoBrush);
        painter.drawPolygon(frame);

        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(251, 146, 60));
        for (const QPointF &handle : selectionHandlePoints(localBounds)) {
            const QPointF rotatedHandle = rotatedPoint(handle, center, angle);
            painter.drawRoundedRect(QRectF(rotatedHandle.x() - 4.5,
                                           rotatedHandle.y() - 4.5,
                                           9.0,
                                           9.0),
                                    2.0,
                                    2.0);
        }

        drawLineAnchorHandles(painter, *singleSelectedAnnotation, center, angle, true);
        drawNumberPointHandles(*singleSelectedAnnotation, center, angle, true);

        const QPointF topCenter =
            rotatedPoint(QPointF(localBounds.center().x(), localBounds.top()), center, angle);
        const QPointF rotateHandle = annotationRotationHandlePoint(*singleSelectedAnnotation, true);
        painter.setPen(QPen(QColor(251, 146, 60), 1.8, Qt::SolidLine, Qt::RoundCap));
        painter.drawLine(topCenter, rotateHandle);
        painter.setBrush(QColor(251, 146, 60));
        painter.setPen(QPen(QColor(255, 255, 255), 1.5));
        painter.drawEllipse(QRectF(rotateHandle.x() - 6.0,
                                   rotateHandle.y() - 6.0,
                                   12.0,
                                   12.0));
    } else {
        if (selectedIds.size() > 1) {
            painter.setPen(QPen(QColor(255, 255, 255, 230), 4.0, Qt::DashLine));
            painter.setBrush(Qt::NoBrush);
            painter.drawRoundedRect(bounds, 4.0, 4.0);
        }
        painter.setPen(QPen(QColor(251, 146, 60), 2.0, Qt::DashLine));
        painter.setBrush(Qt::NoBrush);
        painter.drawRoundedRect(bounds, 4.0, 4.0);
        for (const QPointF &handle : selectionHandlePoints(bounds)) {
            if (selectedIds.size() > 1) {
                painter.setPen(QPen(QColor(255, 255, 255), 1.5));
                painter.setBrush(QColor(255, 255, 255));
                painter.drawRoundedRect(QRectF(handle.x() - 5.8, handle.y() - 5.8, 11.6, 11.6), 2.6, 2.6);
            }
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(251, 146, 60));
            painter.drawRoundedRect(QRectF(handle.x() - 4.5, handle.y() - 4.5, 9.0, 9.0), 2.0, 2.0);
        }
        if (singleSelectedAnnotation) {
            drawLineAnchorHandles(painter, *singleSelectedAnnotation, {}, 0.0, false);
        }
        if (selectedIds.size() > 1) {
            const QPointF topCenter(bounds.center().x(), bounds.top());
            const QPointF rotateHandle = selectionRotationHandlePoint(selectedAnnotationsBounds(), true);
            if (!rotateHandle.isNull()) {
                painter.setPen(QPen(QColor(251, 146, 60), 1.8, Qt::SolidLine, Qt::RoundCap));
                painter.drawLine(topCenter, rotateHandle);
                painter.setBrush(QColor(251, 146, 60));
                painter.setPen(QPen(QColor(255, 255, 255), 1.5));
                painter.drawEllipse(QRectF(rotateHandle.x() - 6.0,
                                           rotateHandle.y() - 6.0,
                                           12.0,
                                           12.0));
            }
        }
    }
    if (selectedIds.size() == 1 && !rotatedSingleSelection) {
        if (const Annotation *annotation = annotationById(selectedIds.first());
            annotation && annotation->tool == Tool::Magnifier) {
            const QRectF sourceRect = imageRectToWidget(magnifierSourceRect(*annotation));
            const QRectF lensRect = imageRectToWidget(annotation->rect.normalized());
            painter.setPen(QPen(QColor(251, 146, 60), 2.0, Qt::DashLine));
            painter.setBrush(Qt::NoBrush);
            if (annotation->magnifierShape == MagnifierShape::Rectangle) {
                painter.drawRect(sourceRect);
                painter.drawRect(lensRect);
            } else {
                painter.drawEllipse(sourceRect);
                painter.drawEllipse(lensRect);
            }
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(251, 146, 60));
            for (const QPointF &center : {sourceRect.center(), lensRect.center()}) {
                painter.drawEllipse(QRectF(center.x() - 5.0,
                                           center.y() - 5.0,
                                           10.0,
                                           10.0));
            }
            // 矩形放大镜:source 小框与 lens 大框各自绘制 8 个 resize 把手
            // 圆形放大镜:source 小框与 lens 大框各自绘制 4 个 resize 把手(上下左右)
            painter.setBrush(QColor(251, 146, 60));
            painter.setPen(QPen(QColor(255, 255, 255), 1.5));
            const qreal handleSize = 8.0;
            auto drawHandles = [&](const QRectF &rect, bool eightHandles) {
                QVector<QPointF> handles;
                if (eightHandles) {
                    handles = {rect.topLeft(), rect.topRight(),
                               rect.bottomLeft(), rect.bottomRight(),
                               QPointF(rect.center().x(), rect.top()),
                               QPointF(rect.center().x(), rect.bottom()),
                               QPointF(rect.left(), rect.center().y()),
                               QPointF(rect.right(), rect.center().y())};
                } else {
                    handles = {QPointF(rect.center().x(), rect.top()),
                               QPointF(rect.center().x(), rect.bottom()),
                               QPointF(rect.left(), rect.center().y()),
                               QPointF(rect.right(), rect.center().y())};
                }
                for (const QPointF &corner : handles) {
                    painter.drawRect(QRectF(corner.x() - handleSize / 2.0,
                                            corner.y() - handleSize / 2.0,
                                            handleSize,
                                            handleSize));
                }
            };
            const bool eightHandles =
                annotation->magnifierShape == MagnifierShape::Rectangle;
            drawHandles(lensRect, eightHandles);
            drawHandles(sourceRect, eightHandles);
        } else if (annotation && annotation->tool == Tool::Number) {
            drawNumberPointHandles(*annotation, {}, 0.0, false);
        }
    }
    const QRectF deleteButton = selectedAnnotationDeleteButtonRect();
    if (!deleteButton.isEmpty()) {
        painter.setBrush(QColor(239, 68, 68));
        painter.setPen(QPen(QColor(255, 255, 255), 2.0, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        painter.drawEllipse(deleteButton);
        painter.drawLine(deleteButton.center() + QPointF(-4.5, -4.5), deleteButton.center() + QPointF(4.5, 4.5));
        painter.drawLine(deleteButton.center() + QPointF(4.5, -4.5), deleteButton.center() + QPointF(-4.5, 4.5));
    }
    painter.restore();
}

ShotWindow::SelectionDrag ShotWindow::selectedAnnotationsDragAt(QPointF imagePoint) const
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    const QRectF bounds = selectedAnnotationsBounds();
    if (selectedIds.size() > 1) {
        const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
        const QPointF rotationHandle = selectionRotationHandlePoint(bounds, false);
        if (!rotationHandle.isNull() && QLineF(imagePoint, rotationHandle).length() <= imageTolerance * 1.4) {
            return SelectionDrag::Rotate;
        }
    }

    return annotationBoundsDragAt(imagePoint, bounds);
}

ShotWindow::SelectionDrag ShotWindow::magnifierDragAt(const Annotation &annotation, QPointF imagePoint) const
{
    if (annotation.tool != Tool::Magnifier) {
        return SelectionDrag::None;
    }

    const QRectF lensRect = annotation.rect.normalized();
    const QRectF sourceRect = magnifierSourceRect(annotation);
    if (lensRect.isEmpty() || sourceRect.isEmpty()) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    const qreal handleTolerance = imageTolerance * 1.2;
    const auto cornerHit = [imagePoint, handleTolerance](QPointF handle) {
        return QLineF(imagePoint, handle).length() <= handleTolerance;
    };

    if (annotation.magnifierShape == MagnifierShape::Rectangle) {
        // 1. 矩形放大镜:lens 大框 8 把手优先匹配
        if (cornerHit(lensRect.topLeft())) return SelectionDrag::TopLeft;
        if (cornerHit(lensRect.topRight())) return SelectionDrag::TopRight;
        if (cornerHit(lensRect.bottomLeft())) return SelectionDrag::BottomLeft;
        if (cornerHit(lensRect.bottomRight())) return SelectionDrag::BottomRight;
        if (cornerHit(QPointF(lensRect.center().x(), lensRect.top()))) return SelectionDrag::Top;
        if (cornerHit(QPointF(lensRect.center().x(), lensRect.bottom()))) return SelectionDrag::Bottom;
        if (cornerHit(QPointF(lensRect.left(), lensRect.center().y()))) return SelectionDrag::Left;
        if (cornerHit(QPointF(lensRect.right(), lensRect.center().y()))) return SelectionDrag::Right;
        // 2. source 小框 8 把手次之
        if (cornerHit(sourceRect.topLeft())) return SelectionDrag::MagnifierSourceTopLeft;
        if (cornerHit(sourceRect.topRight())) return SelectionDrag::MagnifierSourceTopRight;
        if (cornerHit(sourceRect.bottomLeft())) return SelectionDrag::MagnifierSourceBottomLeft;
        if (cornerHit(sourceRect.bottomRight())) return SelectionDrag::MagnifierSourceBottomRight;
        if (cornerHit(QPointF(sourceRect.center().x(), sourceRect.top()))) return SelectionDrag::MagnifierSourceTop;
        if (cornerHit(QPointF(sourceRect.center().x(), sourceRect.bottom()))) return SelectionDrag::MagnifierSourceBottom;
        if (cornerHit(QPointF(sourceRect.left(), sourceRect.center().y()))) return SelectionDrag::MagnifierSourceLeft;
        if (cornerHit(QPointF(sourceRect.right(), sourceRect.center().y()))) return SelectionDrag::MagnifierSourceRight;
        // 3. 内部命中:source 在内层,优先于 lens
        if (sourceRect.contains(imagePoint)) {
            return SelectionDrag::MagnifierSource;
        }
        if (lensRect.contains(imagePoint)) {
            return SelectionDrag::MagnifierLens;
        }
        return SelectionDrag::None;
    }

    // 圆形放大镜:lens 与 source 各 4 把手(上下左右),用同一套优先级
    // 1. lens 4 把手
    if (cornerHit(QPointF(lensRect.center().x(), lensRect.top()))) return SelectionDrag::Top;
    if (cornerHit(QPointF(lensRect.center().x(), lensRect.bottom()))) return SelectionDrag::Bottom;
    if (cornerHit(QPointF(lensRect.left(), lensRect.center().y()))) return SelectionDrag::Left;
    if (cornerHit(QPointF(lensRect.right(), lensRect.center().y()))) return SelectionDrag::Right;
    // 2. source 4 把手
    if (cornerHit(QPointF(sourceRect.center().x(), sourceRect.top()))) return SelectionDrag::MagnifierSourceTop;
    if (cornerHit(QPointF(sourceRect.center().x(), sourceRect.bottom()))) return SelectionDrag::MagnifierSourceBottom;
    if (cornerHit(QPointF(sourceRect.left(), sourceRect.center().y()))) return SelectionDrag::MagnifierSourceLeft;
    if (cornerHit(QPointF(sourceRect.right(), sourceRect.center().y()))) return SelectionDrag::MagnifierSourceRight;
    // 3. 区域命中
    if (ellipseContainsPoint(sourceRect, imagePoint, imageTolerance)) {
        return SelectionDrag::MagnifierSource;
    }
    if (ellipseContainsPoint(lensRect, imagePoint, imageTolerance)) {
        return SelectionDrag::MagnifierLens;
    }
    return SelectionDrag::None;
}

ShotWindow::SelectionDrag ShotWindow::numberDragAt(const Annotation &annotation, QPointF imagePoint) const
{
    if (annotation.tool != Tool::Number || annotation.points.isEmpty()) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    const qreal radius = std::max<qreal>(13.0, 13.0 + annotation.width * 1.35);
    const QPointF tip = annotation.points.first();
    const QPointF bubble = annotation.points.size() >= 2 ? annotation.points.last() : tip;
    const QRectF bubbleRect(bubble.x() - radius, bubble.y() - radius, radius * 2.0, radius * 2.0);
    if (ellipseContainsPoint(bubbleRect, imagePoint, imageTolerance)) {
        return SelectionDrag::NumberBubble;
    }
    if (QLineF(imagePoint, tip).length() <= imageTolerance * 1.6) {
        return SelectionDrag::NumberTip;
    }
    return SelectionDrag::None;
}

QVector<QPointF> ShotWindow::selectionHandlePoints(QRectF rect) const
{
    rect = rect.normalized();
    return {
        rect.topLeft(), QPointF(rect.center().x(), rect.top()), rect.topRight(),
        QPointF(rect.left(), rect.center().y()), QPointF(rect.right(), rect.center().y()),
        rect.bottomLeft(), QPointF(rect.center().x(), rect.bottom()), rect.bottomRight(),
    };
}

QRectF ShotWindow::selectedAnnotationDeleteButtonRect() const
{
    constexpr qreal buttonSize = 20.0;
    constexpr qreal margin = 8.0;
    auto clampedButtonRect = [this](QPointF center) {
        constexpr qreal buttonSize = 20.0;
        constexpr qreal margin = 8.0;
        const qreal x = std::clamp(center.x() - buttonSize / 2.0,
                                   margin,
                                   std::max<qreal>(margin, width() - buttonSize - margin));
        const qreal y = std::clamp(center.y() - buttonSize / 2.0,
                                   margin,
                                   std::max<qreal>(margin, height() - buttonSize - margin));
        return QRectF(x, y, buttonSize, buttonSize);
    };

    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.size() == 1) {
        if (const Annotation *annotation = annotationById(selectedIds.first());
            annotation && annotationSupportsRotation(*annotation)) {
            const QRectF localBounds = imageRectToWidget(annotationUnrotatedBounds(*annotation));
            if (!localBounds.isEmpty()) {
                const QPointF center = localBounds.center();
                const QPointF corner = rotatedPoint(localBounds.topRight(), center, annotation->rotationDegrees);
                QPointF direction = corner - center;
                const qreal length = QLineF(center, corner).length();
                if (length <= 0.1) {
                    direction = QPointF(1.0, -1.0);
                } else {
                    direction /= length;
                }
                return clampedButtonRect(corner + direction * (buttonSize * 1.2));
            }
        }
    }

    const QRectF bounds = imageRectToWidget(selectedAnnotationsBounds());
    if (bounds.isEmpty()) {
        return {};
    }
    return clampedButtonRect(QPointF(bounds.right() + buttonSize / 2.0 + margin,
                                     bounds.top() - buttonSize / 2.0 - margin));
}

QRectF ShotWindow::resizedBounds(QRectF start, SelectionDrag drag, QPointF imagePoint, bool keepAspectRatio) const
{
    start = start.normalized();
    const QPointF clamped = clampImagePoint(imagePoint);
    qreal left = start.left();
    qreal top = start.top();
    qreal right = start.right();
    qreal bottom = start.bottom();
    const qreal maxWidth = m_frozenFrame.width();
    const qreal maxHeight = m_frozenFrame.height();

    if (keepAspectRatio && drag != SelectionDrag::Move && start.width() > 0.0 && start.height() > 0.0) {
        const qreal minScale = std::max(kMinSelectionSize / start.width(), kMinSelectionSize / start.height());

        auto boundedScale = [minScale](qreal rawScale, qreal maxScale) {
            maxScale = std::max<qreal>(0.0, maxScale);
            const qreal lower = std::min(minScale, maxScale);
            return std::clamp(rawScale, lower, maxScale);
        };

        auto rectFromCorner = [&](QPointF anchor, qreal xSign, qreal ySign) {
            const qreal xDistance = std::abs(clamped.x() - anchor.x());
            const qreal yDistance = std::abs(clamped.y() - anchor.y());
            const qreal rawScale = std::max(xDistance / start.width(), yDistance / start.height());
            const qreal maxXScale = (xSign > 0.0 ? maxWidth - anchor.x() : anchor.x()) / start.width();
            const qreal maxYScale = (ySign > 0.0 ? maxHeight - anchor.y() : anchor.y()) / start.height();
            const qreal scale = boundedScale(rawScale, std::min(maxXScale, maxYScale));
            return QRectF(anchor,
                          QPointF(anchor.x() + xSign * start.width() * scale,
                                  anchor.y() + ySign * start.height() * scale)).normalized();
        };

        auto rectFromHorizontalEdge = [&](qreal anchorX, qreal xSign, qreal centerY) {
            const qreal rawScale = std::abs(clamped.x() - anchorX) / start.width();
            const qreal maxXScale = (xSign > 0.0 ? maxWidth - anchorX : anchorX) / start.width();
            const qreal maxYScale = (2.0 * std::min(centerY, maxHeight - centerY)) / start.height();
            const qreal scale = boundedScale(rawScale, std::min(maxXScale, maxYScale));
            const qreal newWidth = start.width() * scale;
            const qreal newHeight = start.height() * scale;
            return QRectF(QPointF(anchorX, centerY - newHeight / 2.0),
                          QPointF(anchorX + xSign * newWidth, centerY + newHeight / 2.0)).normalized();
        };

        auto rectFromVerticalEdge = [&](qreal anchorY, qreal ySign, qreal centerX) {
            const qreal rawScale = std::abs(clamped.y() - anchorY) / start.height();
            const qreal maxYScale = (ySign > 0.0 ? maxHeight - anchorY : anchorY) / start.height();
            const qreal maxXScale = (2.0 * std::min(centerX, maxWidth - centerX)) / start.width();
            const qreal scale = boundedScale(rawScale, std::min(maxXScale, maxYScale));
            const qreal newWidth = start.width() * scale;
            const qreal newHeight = start.height() * scale;
            return QRectF(QPointF(centerX - newWidth / 2.0, anchorY),
                          QPointF(centerX + newWidth / 2.0, anchorY + ySign * newHeight)).normalized();
        };

        switch (drag) {
        case SelectionDrag::TopLeft:
            return rectFromCorner(start.bottomRight(), -1.0, -1.0);
        case SelectionDrag::TopRight:
            return rectFromCorner(start.bottomLeft(), 1.0, -1.0);
        case SelectionDrag::BottomLeft:
            return rectFromCorner(start.topRight(), -1.0, 1.0);
        case SelectionDrag::BottomRight:
            return rectFromCorner(start.topLeft(), 1.0, 1.0);
        case SelectionDrag::Left:
            return rectFromHorizontalEdge(start.right(), -1.0, start.center().y());
        case SelectionDrag::Right:
            return rectFromHorizontalEdge(start.left(), 1.0, start.center().y());
        case SelectionDrag::Top:
            return rectFromVerticalEdge(start.bottom(), -1.0, start.center().x());
        case SelectionDrag::Bottom:
            return rectFromVerticalEdge(start.top(), 1.0, start.center().x());
        case SelectionDrag::MagnifierSource:
        case SelectionDrag::MagnifierLens:
        case SelectionDrag::MagnifierSourceLeft:
        case SelectionDrag::MagnifierSourceRight:
        case SelectionDrag::MagnifierSourceTop:
        case SelectionDrag::MagnifierSourceBottom:
        case SelectionDrag::MagnifierSourceTopLeft:
        case SelectionDrag::MagnifierSourceTopRight:
        case SelectionDrag::MagnifierSourceBottomLeft:
        case SelectionDrag::MagnifierSourceBottomRight:
        case SelectionDrag::Rotate:
        case SelectionDrag::LineControl:
        case SelectionDrag::LineStart:
        case SelectionDrag::LineEnd:
        case SelectionDrag::NumberTip:
        case SelectionDrag::NumberBubble:
        case SelectionDrag::Move:
        case SelectionDrag::None:
            break;
        }
    }

    if (drag == SelectionDrag::Left || drag == SelectionDrag::TopLeft || drag == SelectionDrag::BottomLeft) {
        left = std::clamp(clamped.x(), 0.0, right - kMinSelectionSize);
    }
    if (drag == SelectionDrag::Right || drag == SelectionDrag::TopRight || drag == SelectionDrag::BottomRight) {
        right = std::clamp(clamped.x(), left + kMinSelectionSize, maxWidth);
    }
    if (drag == SelectionDrag::Top || drag == SelectionDrag::TopLeft || drag == SelectionDrag::TopRight) {
        top = std::clamp(clamped.y(), 0.0, bottom - kMinSelectionSize);
    }
    if (drag == SelectionDrag::Bottom || drag == SelectionDrag::BottomLeft || drag == SelectionDrag::BottomRight) {
        bottom = std::clamp(clamped.y(), top + kMinSelectionSize, maxHeight);
    }

    return QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
}
