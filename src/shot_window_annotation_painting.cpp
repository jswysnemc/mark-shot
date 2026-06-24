#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

void ShotWindow::drawAnnotation(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const
{
    auto mapPoint = [this, widgetCoordinates](QPointF point) {
        return widgetCoordinates ? imageToWidget(point) : point;
    };

    auto mapRect = [this, widgetCoordinates](QRectF rect) {
        return widgetCoordinates ? imageRectToWidget(rect) : rect;
    };

    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal penWidth = std::max<qreal>(1.5, annotation.width * scale);

    painter.save();
    if (annotationSupportsRotation(annotation) && !qFuzzyIsNull(annotation.rotationDegrees)) {
        const QPointF center = annotationRotationCenter(annotation, widgetCoordinates);
        painter.translate(center);
        painter.rotate(annotation.rotationDegrees);
        painter.translate(-center);
    }

    QPen pen(annotation.color, penWidth, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
    painter.setPen(pen);
    painter.setBrush(Qt::NoBrush);

    auto drawLinePath = [this, &painter, &mapPoint](const Annotation &lineAnnotation) {
        if (lineAnnotation.points.size() < 2) {
            return;
        }
        QVector<QPointF> mappedPoints;
        mappedPoints.reserve(lineAnnotation.points.size());
        for (const QPointF &point : lineAnnotation.points) {
            mappedPoints.append(mapPoint(point));
        }
        painter.drawPath(lineSkeletonPath(mappedPoints));
    };

    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        break;
    case Tool::Pen: {
        if (annotation.points.size() < 2) {
            break;
        }
        QVector<QPointF> mapped;
        mapped.reserve(annotation.points.size());
        for (const QPointF &point : annotation.points) {
            mapped.append(mapPoint(point));
        }
        painter.drawPath(smoothedStrokePath(mapped));
        break;
    }
    case Tool::Highlighter: {
        if (annotation.points.size() < 2) {
            break;
        }
        // 荧光笔采用乘法混合模拟真实高亮笔:白底着色、深色文字保留可读性、笔画重叠不泛白
        QColor color = annotation.color;
        color.setAlpha(qRound(annotation.color.alphaF() * 200.0));
        painter.save();
        painter.setCompositionMode(QPainter::CompositionMode_Multiply);
        painter.setPen(QPen(color, std::max<qreal>(6.0, penWidth), Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        if (annotation.highlighterStyle == HighlighterStyle::StraightLine) {
            drawLinePath(annotation);
        } else {
            QVector<QPointF> mapped;
            mapped.reserve(annotation.points.size());
            for (const QPointF &point : annotation.points) {
                mapped.append(mapPoint(point));
            }
            painter.drawPath(smoothedStrokePath(mapped));
        }
        painter.restore();
        break;
    }
    case Tool::Line:
        if (annotation.points.size() >= 2) {
            drawLinePath(annotation);
        }
        break;
    case Tool::Rectangle: {
        drawRectangle(painter, annotation, widgetCoordinates);
        break;
    }
    case Tool::Ellipse:
        painter.setBrush(annotation.filled ? QBrush(annotation.color) : QBrush(Qt::NoBrush));
        painter.drawEllipse(mapRect(annotation.rect));
        break;
    case Tool::Arrow:
        if (annotation.points.size() >= 2) {
            QVector<QPointF> mappedPoints;
            mappedPoints.reserve(annotation.points.size());
            for (const QPointF &point : annotation.points) {
                mappedPoints.append(mapPoint(point));
            }
            drawArrow(painter,
                      mappedPoints,
                      penWidth,
                      annotation.arrowStyle);
        }
        break;
    case Tool::Text: {
        QFont font = markshot::theme::textFont(qRound((19.0 + annotation.width) * scale),
                                               QFont::DemiBold,
                                               annotation.fontFamily);
        QRectF backgroundRect = textContentRect(annotation, widgetCoordinates);
        QRectF textRect = backgroundRect.adjusted(kTextBackgroundPaddingX * scale,
                                                  kTextBackgroundPaddingY * scale,
                                                  -kTextBackgroundPaddingX * scale,
                                                  -kTextBackgroundPaddingY * scale);
        QTextOption option;
        option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
        option.setAlignment(Qt::AlignLeft | Qt::AlignTop);
        painter.save();
        painter.setFont(font);
        if (annotation.backgroundColor.alpha() > 0) {
            painter.setPen(Qt::NoPen);
            painter.setBrush(annotation.backgroundColor);
            painter.drawRoundedRect(backgroundRect, 4.0 * scale, 4.0 * scale);
        }
        painter.setPen(annotation.color);
        painter.setBrush(Qt::NoBrush);
        painter.drawText(textRect, annotation.text, option);
        painter.restore();
        break;
    }
    case Tool::Number:
        if (!annotation.points.isEmpty()) {
            const QPointF tipPoint = annotation.points.first();
            const QPointF bubblePoint = annotation.points.size() >= 2 ? annotation.points.last() : tipPoint;
            drawNumber(painter,
                       tipPoint,
                       bubblePoint,
                       numberLabelText(annotation.number, annotation.numberStyle),
                       annotation.color,
                       annotation.width,
                       widgetCoordinates);
        }
        break;
    case Tool::Mosaic:
        painter.setOpacity(annotation.color.alphaF());
        drawMosaic(painter, annotation.rect, annotation.width, widgetCoordinates);
        break;
    case Tool::Magnifier:
        drawMagnifier(painter, annotation, widgetCoordinates);
        break;
    }
    painter.restore();
}

void ShotWindow::drawRectangle(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const
{
    // 矩形工具支持三种风格:
    //   Stroke    描边/填充矩形(默认行为,沿用 filled+cornerRadius)
    //   Highlight 类荧光笔的矩形高亮:Multiply 混合 + 半透明色填充
    //   Invert    矩形覆盖区域内做像素反色
    auto mapRect = [this, widgetCoordinates](QRectF rect) {
        return widgetCoordinates ? imageRectToWidget(rect) : rect;
    };

    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal penWidth = std::max<qreal>(1.5, annotation.width * scale);
    const QRectF rect = mapRect(annotation.rect);
    const qreal radius = annotation.cornerRadius * scale;

    // 注:外层 drawAnnotation 已经 setPen(annotation.color) 并 save/restore,这里
    // 仅在需要时改写 painter 状态,统一用一个 inner save 块隔离副作用。
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    QPainterPath shapePath;
    if (radius > 0.0) {
        shapePath.addRoundedRect(rect, radius, radius);
    } else {
        shapePath.addRect(rect);
    }

    switch (annotation.rectangleStyle) {
    case RectangleStyle::Stroke: {
        // 1. 与原始 case 行为一致:可填充、可圆角
        painter.setPen(QPen(annotation.color, penWidth, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        painter.setBrush(annotation.filled ? QBrush(annotation.color) : QBrush(Qt::NoBrush));
        if (radius > 0.0) {
            painter.drawRoundedRect(rect, radius, radius);
        } else {
            painter.drawRect(rect);
        }
        break;
    }
    case RectangleStyle::Highlight: {
        // 1. 用 Multiply 混合 + 半透明色,叠加在底图上模拟荧光笔覆盖
        QColor highlightColor = annotation.color;
        highlightColor.setAlpha(qRound(annotation.color.alphaF() * 200.0));
        painter.setCompositionMode(QPainter::CompositionMode_Multiply);
        painter.setPen(Qt::NoPen);
        painter.setBrush(highlightColor);
        painter.drawPath(shapePath);
        break;
    }
    case RectangleStyle::Invert: {
        // 1. 取出矩形覆盖区域内的源图像
        // 2. 对图像 RGB 反相后绘制回原位置;对透明像素保持 alpha
        // 3. 形状外轮廓另以当前颜色描边作为视觉提示
        if (m_frozenFrame.isNull()) {
            break;
        }
        const QRectF imageRect = annotation.rect.normalized();
        if (imageRect.isEmpty()) {
            break;
        }
        const QRect intRect = imageRect.toAlignedRect()
                                  .intersected(QRect(0, 0,
                                                     m_frozenFrame.width(),
                                                     m_frozenFrame.height()));
        if (intRect.isEmpty()) {
            break;
        }
        QImage region = m_frozenFrame.copy(intRect)
                            .convertToFormat(QImage::Format_ARGB32);
        region.invertPixels(QImage::InvertRgb);
        const QRectF destRect = widgetCoordinates ? imageRectToWidget(intRect) : QRectF(intRect);
        painter.save();
        painter.setClipPath(shapePath);
        painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
        painter.drawImage(destRect, region);
        painter.restore();
        // 4. 描边可选保留;width 极小时省略以避免视觉干扰
        if (penWidth >= 1.5) {
            painter.setPen(QPen(annotation.color, penWidth, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
            painter.setBrush(Qt::NoBrush);
            if (radius > 0.0) {
                painter.drawRoundedRect(rect, radius, radius);
            } else {
                painter.drawRect(rect);
            }
        }
        break;
    }
    }

    painter.restore();
}

void ShotWindow::drawWheelPreview(QPainter &painter)
{
    if (!m_showWheelPreview || !m_wheelPreviewTimer.isValid() || m_wheelPreviewTimer.elapsed() > 900) {
        m_showWheelPreview = false;
        updateCursor();
        return;
    }

    if (wheelZoomsImage()) {
        const QString zoomText = QStringLiteral("%1%").arg(qRound(m_imageZoom * 100.0));
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setFont(markshot::theme::uiFont(12, QFont::DemiBold));
        const QFontMetrics metrics(painter.font());
        const QRectF textBounds = metrics.boundingRect(zoomText);
        QRectF bubble(m_wheelPreviewPosition.x() + 14.0,
                      m_wheelPreviewPosition.y() + 14.0,
                      textBounds.width() + 24.0,
                      textBounds.height() + 14.0);
        bubble.moveLeft(std::min<qreal>(bubble.left(), width() - bubble.width() - 8.0));
        bubble.moveTop(std::min<qreal>(bubble.top(), height() - bubble.height() - 8.0));
        bubble.moveLeft(std::max<qreal>(8.0, bubble.left()));
        bubble.moveTop(std::max<qreal>(8.0, bubble.top()));

        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(8, 13, 19, 230));
        painter.drawRoundedRect(bubble, 10.0, 10.0);
        painter.setPen(QColor(204, 251, 241, 245));
        painter.drawText(bubble, Qt::AlignCenter, zoomText);
        painter.restore();
        return;
    }

    const qreal size = std::clamp(currentToolPreviewSize(), 2.0, 96.0);
    QRectF preview(m_wheelPreviewPosition.x() - size / 2.0,
                   m_wheelPreviewPosition.y() - size / 2.0,
                   size,
                   size);

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing,
                          m_tool == Tool::Number || m_tool == Tool::Magnifier);
    painter.setPen(Qt::NoPen);
    painter.setBrush(m_currentColor);
    if (m_tool == Tool::Number || m_tool == Tool::Magnifier) {
        painter.drawEllipse(preview);
    } else {
        painter.drawRect(preview);
    }
    painter.restore();
}

void ShotWindow::drawLaserStroke(QPainter &painter, const LaserStroke &stroke, bool widgetCoordinates, qreal opacity) const
{
    if (stroke.points.size() < 2 || opacity <= 0.0) {
        return;
    }

    auto mapPoint = [this, widgetCoordinates](QPointF point) {
        return widgetCoordinates ? imageToWidget(point) : point;
    };
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal width = std::max<qreal>(3.0, stroke.width * scale);

    QVector<QPointF> mappedPoints;
    mappedPoints.reserve(stroke.points.size());
    for (const QPointF &point : stroke.points) {
        mappedPoints.append(mapPoint(point));
    }
    const QPainterPath path = smoothedStrokePath(mappedPoints);

    const qreal configuredOpacity = stroke.color.alphaF();
    QColor glow = stroke.color;
    glow.setAlpha(qRound(80 * opacity * configuredOpacity));
    QColor core = stroke.color;
    core.setAlpha(qRound(230 * opacity * configuredOpacity));
    QColor hot(255, 255, 255, qRound(170 * opacity * configuredOpacity));

    painter.save();
    painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
    painter.setPen(QPen(glow, width * 2.6, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawPath(path);
    painter.setPen(QPen(core, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawPath(path);
    painter.setPen(QPen(hot, std::max<qreal>(1.4, width * 0.22), Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawPath(path);
    painter.restore();
}

void ShotWindow::beginLaserStroke(QPointF imagePoint)
{
    m_dragging = true;
    m_dragStart = imagePoint;
    LaserStroke stroke;
    stroke.points.append(clampImagePoint(imagePoint));
    stroke.color = m_currentColor;
    stroke.width = m_strokeWidth;
    stroke.expiresAt = m_laserClock.elapsed() + kLaserLifetimeMs;
    m_laserDraft = stroke;
    update();
}

void ShotWindow::updateLaserStroke(QPointF imagePoint)
{
    if (!m_laserDraft.has_value()) {
        return;
    }
    const QPointF nextPoint = clampImagePoint(imagePoint);
    if (m_laserDraft->points.isEmpty()) {
        m_laserDraft->points.append(nextPoint);
        update();
        return;
    }

    const QPointF lastPoint = m_laserDraft->points.last();
    const qreal distance = QLineF(lastPoint, nextPoint).length();
    const qreal step = std::max<qreal>(2.0, m_strokeWidth * 0.38);
    if (distance < step * 0.35) {
        return;
    }

    const int inserts = std::clamp(static_cast<int>(std::floor(distance / step)), 0, 24);
    for (int i = 1; i <= inserts; ++i) {
        const qreal t = static_cast<qreal>(i) / static_cast<qreal>(inserts + 1);
        m_laserDraft->points.append(lastPoint + (nextPoint - lastPoint) * t);
    }
    m_laserDraft->points.append(nextPoint);
    update();
}

void ShotWindow::commitLaserStroke()
{
    if (!m_laserDraft.has_value()) {
        return;
    }
    if (m_laserDraft->points.size() >= 2) {
        m_laserDraft->expiresAt = m_laserClock.elapsed() + kLaserLifetimeMs;
        m_laserStrokes.append(*m_laserDraft);
        if (m_laserTimer && !m_laserTimer->isActive()) {
            m_laserTimer->start();
        }
    }
    m_laserDraft.reset();
    update();
}

void ShotWindow::cleanupLaserStrokes()
{
    const qint64 now = m_laserClock.elapsed();
    for (int i = m_laserStrokes.size() - 1; i >= 0; --i) {
        if (m_laserStrokes.at(i).expiresAt <= now) {
            m_laserStrokes.removeAt(i);
        }
    }
    if (m_laserStrokes.isEmpty() && m_laserTimer) {
        m_laserTimer->stop();
    }
    update();
}

void ShotWindow::drawNumber(QPainter &painter,
                            QPointF tipPoint,
                            QPointF bubblePoint,
                            const QString &label,
                            QColor color,
                            qreal width,
                            bool widgetCoordinates) const
{
    const QPointF tip = widgetCoordinates ? imageToWidget(tipPoint) : tipPoint;
    const QPointF center = widgetCoordinates ? imageToWidget(bubblePoint) : bubblePoint;
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal radius = std::max<qreal>(13.0, (13.0 + width * 1.35) * scale);
    const QRectF bubble(center.x() - radius, center.y() - radius, radius * 2.0, radius * 2.0);

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    const QLineF leader(center, tip);
    if (leader.length() > radius * 0.45) {
        const QPointF direction(leader.dx() / leader.length(), leader.dy() / leader.length());
        const QPointF normal(-direction.y(), direction.x());
        const qreal tailHalfWidth = std::clamp(radius * 0.46, 8.0, 38.0);
        const QPointF baseCenter = center + direction * (radius * 0.82);
        QPainterPath tail;
        tail.moveTo(tip);
        tail.lineTo(baseCenter + normal * tailHalfWidth);
        tail.lineTo(baseCenter - normal * tailHalfWidth);
        tail.closeSubpath();
        painter.setPen(Qt::NoPen);
        painter.setBrush(color);
        painter.drawPath(tail);
    }

    painter.setPen(QPen(QColor(255, 255, 255), std::clamp(width * 0.22 * scale, 2.0, 9.0)));
    painter.setBrush(color);
    painter.drawEllipse(bubble);

    int fontSize = qRound(std::clamp(radius * 0.92, 12.0, 54.0));
    QFont font = markshot::theme::uiFont(fontSize, QFont::Black);
    while (fontSize > 8 && QFontMetrics(font).horizontalAdvance(label) > radius * 1.58) {
        font.setPointSize(--fontSize);
    }
    painter.setFont(font);
    painter.setPen(Qt::white);
    painter.drawText(bubble, Qt::AlignCenter, label);
    painter.restore();
}

void ShotWindow::drawMosaic(QPainter &painter, QRectF imageRect, qreal blockSize, bool widgetCoordinates) const
{
    QRect sourceRect = imageRect.normalized().toAlignedRect().intersected(QRect(QPoint(0, 0), m_frozenFrame.size()));
    if (sourceRect.isEmpty()) {
        return;
    }

    const QImage mosaic = mosaicImage(sourceRect, qRound(blockSize));
    if (mosaic.isNull()) {
        return;
    }

    painter.save();
    painter.setRenderHint(QPainter::SmoothPixmapTransform, false);
    painter.drawImage(widgetCoordinates ? imageRectToWidget(sourceRect) : QRectF(sourceRect), mosaic);
    painter.restore();
}

void ShotWindow::drawMagnifier(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const
{
    const QRectF lensImageRect = annotation.rect.normalized();
    const QRectF sourceImageRect = magnifierSourceRect(annotation);
    if (lensImageRect.width() < 4.0 || lensImageRect.height() < 4.0
        || sourceImageRect.isEmpty() || m_frozenFrame.isNull()) {
        return;
    }

    const QRectF lensRect = widgetCoordinates ? imageRectToWidget(lensImageRect) : lensImageRect;
    const QRectF sourceRect = widgetCoordinates ? imageRectToWidget(sourceImageRect) : sourceImageRect;
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal borderWidth = std::clamp(annotation.width * scale, 1.5, 18.0);
    const QColor borderColor = annotation.color;
    const bool rectangular = annotation.magnifierShape == MagnifierShape::Rectangle;

    QPainterPath lensPath;
    if (rectangular) {
        lensPath.addRect(lensRect);
    } else {
        lensPath.addEllipse(lensRect);
    }

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    const QPointF sourceCenter = sourceRect.center();
    const QPointF lensCenter = lensRect.center();
    const QLineF centerLine(sourceCenter, lensCenter);
    const qreal sourceRadius = sourceRect.width() / 2.0;
    const qreal lensRadius = lensRect.width() / 2.0;
    if (centerLine.length() > lensRadius + sourceRadius * 0.65) {
        const QPointF towardLens(centerLine.dx() / centerLine.length(),
                                 centerLine.dy() / centerLine.length());
        const QPointF normal(-towardLens.y(), towardLens.x());
        constexpr qreal connectorAngle = 34.0 * M_PI / 180.0;
        const qreal along = std::cos(connectorAngle);
        const qreal across = std::sin(connectorAngle);
        const QPointF sourceUpper = sourceCenter
            + towardLens * (sourceRadius * along)
            + normal * (sourceRadius * across);
        const QPointF sourceLower = sourceCenter
            + towardLens * (sourceRadius * along)
            - normal * (sourceRadius * across);
        const QPointF towardSource = -towardLens;
        const QPointF lensUpper = lensCenter
            + towardSource * (lensRadius * along)
            + normal * (lensRadius * across);
        const QPointF lensLower = lensCenter
            + towardSource * (lensRadius * along)
            - normal * (lensRadius * across);

        painter.setPen(QPen(borderColor,
                            borderWidth,
                            Qt::SolidLine,
                            Qt::RoundCap,
                            Qt::RoundJoin));
        painter.drawLine(sourceUpper, lensUpper);
        painter.drawLine(sourceLower, lensLower);
    }

    painter.setClipPath(lensPath);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
    painter.drawImage(lensRect, m_frozenFrame, sourceImageRect);
    painter.setClipping(false);

    painter.setPen(QPen(borderColor, borderWidth, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.setBrush(Qt::NoBrush);
    if (rectangular) {
        painter.drawRect(lensRect.adjusted(borderWidth / 2.0,
                                           borderWidth / 2.0,
                                           -borderWidth / 2.0,
                                           -borderWidth / 2.0));
        painter.drawRect(sourceRect.adjusted(borderWidth / 2.0,
                                             borderWidth / 2.0,
                                             -borderWidth / 2.0,
                                             -borderWidth / 2.0));
    } else {
        painter.drawEllipse(lensRect.adjusted(borderWidth / 2.0,
                                              borderWidth / 2.0,
                                              -borderWidth / 2.0,
                                              -borderWidth / 2.0));
        painter.drawEllipse(sourceRect.adjusted(borderWidth / 2.0,
                                                borderWidth / 2.0,
                                                -borderWidth / 2.0,
                                                -borderWidth / 2.0));
    }
    painter.restore();
}

void ShotWindow::beginTextAnnotation(QPointF imagePoint)
{
    m_editingTextAnnotationId.reset();
    m_textEditorImagePoint = imagePoint;
    m_draft.reset();
    m_textEditor->clear();
    m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(m_currentColor, m_textBackgroundColor, qRound(20.0 + m_textSize)));
    m_textEditor->setFont(markshot::theme::textFont(qRound(20.0 + m_textSize),
                                                    QFont::DemiBold,
                                                    m_textFontFamily));
    m_textEditor->show();
    m_textEditor->raise();
    updateTextEditorGeometry();
    m_textEditor->setFocus(Qt::MouseFocusReason);
    updateLayerShellForIme();
    update();
}

void ShotWindow::beginEditingSelectedTextAnnotation()
{
    if (!m_selectedAnnotationId.has_value()) {
        return;
    }
    Annotation *annotation = annotationById(*m_selectedAnnotationId);
    if (!annotation || annotation->tool != Tool::Text) {
        return;
    }

    m_editingTextAnnotationId = annotation->id;
    m_textEditorImagePoint = annotation->rect.normalized().topLeft();
    m_draft.reset();
    m_textEditor->setPlainText(annotation->text);
    m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(annotation->color, annotation->backgroundColor, qRound(20.0 + annotation->width)));
    m_textEditor->setFont(markshot::theme::textFont(qRound(20.0 + annotation->width),
                                                    QFont::DemiBold,
                                                    annotation->fontFamily));
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    m_textEditor->show();
    m_textEditor->raise();
    const QRectF widgetRect = textContentRect(*annotation, true);
    m_textEditor->setGeometry(widgetRect.toAlignedRect().adjusted(0, 0, 1, 1));
    m_textEditor->setFocus(Qt::MouseFocusReason);
    updateLayerShellForIme();
    update();
}

void ShotWindow::commitTextEditor()
{
    if (m_committingText || !m_textEditor || !m_textEditor->isVisible()) {
        return;
    }

    m_committingText = true;
    const QString text = m_textEditor->toPlainText().trimmed();
    const QRect editorGeometry = m_textEditor->geometry();
    m_textEditor->hide();
    m_textEditor->clear();
    setFocus(Qt::OtherFocusReason);
    updateLayerShellForIme();

    if (m_editingTextAnnotationId.has_value()) {
        if (Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
            pushHistorySnapshot();
            annotation->text = text;
            annotation->rect = QRectF(widgetToImage(editorGeometry.topLeft()),
                                      widgetToImage(editorGeometry.bottomRight())).normalized();
            annotation->fontFamily = m_textEditor->font().family();
            annotation->rect = textContentRect(*annotation, false);
            if (!annotation->points.isEmpty()) {
                annotation->points[0] = annotation->rect.topLeft();
            }
        }
        m_editingTextAnnotationId.reset();
        m_committingText = false;
        updateAnnotationPropertyPanel();
        update();
        return;
    }

    if (!text.isEmpty()) {
        pushHistorySnapshot();
        Annotation annotation;
        annotation.id = m_nextAnnotationId++;
        annotation.tool = Tool::Text;
        annotation.points.append(m_textEditorImagePoint);
        annotation.rect = QRectF(widgetToImage(editorGeometry.topLeft()),
                                 widgetToImage(editorGeometry.bottomRight())).normalized();
        annotation.text = text;
        annotation.color = m_currentColor;
        annotation.backgroundColor = m_textBackgroundColor;
        annotation.width = m_textSize;
        annotation.fontFamily = m_textEditor->font().family();
        annotation.rect = textContentRect(annotation, false);
        m_textFontFamily = annotation.fontFamily;
        m_annotations.append(annotation);
    }

    m_committingText = false;
    update();
    persistAnnotationState();
}

QString ShotWindow::saveSelectionToTempFile() const
{
    if (!hasUsableSelection()) {
        return {};
    }

    const QImage output = renderedSelection();
    if (output.isNull()) {
        return {};
    }

    const QString tempDir = QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
        ? QDir::tempPath()
        : QStandardPaths::writableLocation(QStandardPaths::TempLocation);
    const QString filename = QStringLiteral("mark-shot-open-%1.png")
                                 .arg(QDateTime::currentDateTimeUtc().toString(QStringLiteral("yyyyMMdd-hhmmss-zzz")));
    const QString path = QDir(tempDir).filePath(filename);
    return output.save(path, "PNG") ? path : QString();
}

void ShotWindow::openSelectionWithDesktop(const DesktopApp &app)
{
    commitTextEditor();
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }

    const QString imagePath = saveSelectionToTempFile();
    if (imagePath.isEmpty()) {
        return;
    }

    QStringList command = expandDesktopExec(app, imagePath);
    if (command.isEmpty()) {
        return;
    }

    const QString program = command.takeFirst();
    if (QProcess::startDetached(program, command)) {
        close();
    }
}
