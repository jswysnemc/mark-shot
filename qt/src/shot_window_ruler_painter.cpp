#include "shot_window_ruler_painter.h"

#include "shot_window_module.h"
#include "shot_window_ruler_tool.h"

#include "ruler/angle_painter.h"
#include "ruler/ruler_geometry.h"
#include "ruler/ruler_painter.h"

using namespace markshot::shot;
using markshot::shot::types::Mode;
using markshot::shot::types::RulerMode;
using markshot::shot::types::Tool;

ShotWindowRulerPainter::ShotWindowRulerPainter(const ShotWindow &window)
    : m_window(window)
{
}

/**
 * 计算尺子在当前绘制目标上的缩放倍数：按粗细换算，再换算到截图像素和窗口缩放。
 * @param annotation 尺子标注。
 * @param widgetCoordinates true 表示画到窗口。
 * @return 缩放倍数。
 */
qreal ShotWindowRulerPainter::unitFor(const Annotation &annotation, bool widgetCoordinates) const
{
    // 1. 高分屏截图一个逻辑像素对应多个截图像素，导出图片中的刻度与屏幕上看到的大小一致
    const QRect source = m_window.m_sourceGeometry;
    const qreal captureScale = source.isValid() && source.width() > 0 && !m_window.m_frozenFrame.isNull()
        ? std::clamp<qreal>(static_cast<qreal>(m_window.m_frozenFrame.width()) / source.width(), 0.5, 4.0)
        : 1.0;
    return markshot::ruler::rulerUnitForWidth(annotation.width) * captureScale
        * m_window.annotationSizeScale(widgetCoordinates);
}

/** 把图像坐标 imagePoint 换算到绘制坐标，widgetCoordinates 为 false 时原样返回。 */
QPointF ShotWindowRulerPainter::mapPoint(QPointF imagePoint, bool widgetCoordinates) const
{
    return widgetCoordinates ? m_window.imageToWidget(imagePoint) : imagePoint;
}

/** 把图像坐标 imageRect 换算到绘制坐标，widgetCoordinates 为 false 时原样返回。 */
QRectF ShotWindowRulerPainter::mapRect(const QRectF &imageRect, bool widgetCoordinates) const
{
    return widgetCoordinates ? m_window.imageRectToWidget(imageRect) : imageRect;
}

void ShotWindowRulerPainter::drawBody(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const
{
    const qreal unit = unitFor(annotation, widgetCoordinates);
    if (annotation.rulerMode == RulerMode::Angle) {
        markshot::ruler::AngleGeometry angle{annotation.points, {}, unit, annotation.color};
        for (const QPointF &point : annotation.points) {
            angle.paintPoints.append(mapPoint(point, widgetCoordinates));
        }
        markshot::ruler::drawAngleBody(painter, angle);
        return;
    }
    // 1. 刚按下还没拉开的长度尺先不画
    if (!markshot::ruler::rulerMeasures(annotation)) {
        return;
    }
    const QRectF imageRect = annotation.rect.normalized();
    markshot::ruler::drawRulerScale(painter, {imageRect, mapRect(imageRect, widgetCoordinates), unit, annotation.color});
}

void ShotWindowRulerPainter::drawLabels(QPainter &painter, bool widgetCoordinates) const
{
    // 1. 收集按绘制先后排列的全部尺子，窗口中还包含正在画的草稿
    QVector<const Annotation *> rulers;
    for (const Annotation &annotation : m_window.m_annotations) {
        if (annotation.tool == Tool::Ruler) {
            rulers.append(&annotation);
        }
    }
    if (widgetCoordinates && m_window.m_draft.has_value() && m_window.m_draft->tool == Tool::Ruler) {
        rulers.append(&*m_window.m_draft);
    }
    if (rulers.isEmpty()) {
        return;
    }
    const QRectF bounds = mapRect(m_window.normalizedSelection(), widgetCoordinates);

    // 2. 角度说明固定在角平分线方向，先登记为占用，长度说明再避开它们
    QVector<QRectF> occupied;
    QVector<std::pair<QRectF, QString>> labels;
    QVector<qreal> labelUnits;
    QVector<markshot::ruler::RulerGeometry> lengthRulers;
    for (const Annotation *annotation : std::as_const(rulers)) {
        const qreal unit = unitFor(*annotation, widgetCoordinates);
        if (annotation->rulerMode == RulerMode::Angle) {
            if (annotation->points.size() < 3) {
                continue;
            }
            markshot::ruler::AngleGeometry angle{annotation->points, {}, unit, annotation->color};
            for (const QPointF &point : annotation->points) {
                angle.paintPoints.append(mapPoint(point, widgetCoordinates));
            }
            const QRectF rect = markshot::ruler::angleInfoLabelRect(angle, bounds);
            const qreal degrees = markshot::ruler::angleDegrees(annotation->points.at(0),
                                                                annotation->points.at(1),
                                                                annotation->points.at(2));
            occupied.append(rect);
            labels.append({rect, markshot::ruler::angleInfoText(degrees)});
            labelUnits.append(unit);
        } else if (markshot::ruler::rulerMeasures(*annotation)) {
            const QRectF imageRect = annotation->rect.normalized();
            lengthRulers.append({imageRect, mapRect(imageRect, widgetCoordinates), unit, annotation->color});
        }
    }
    const QVector<QRectF> lengthLabels = markshot::ruler::layoutRulerInfoLabels(lengthRulers, bounds, occupied);
    for (int i = 0; i < lengthRulers.size(); ++i) {
        labels.append({lengthLabels.at(i), markshot::ruler::rulerInfoText(lengthRulers.at(i).imageRect)});
        labelUnits.append(lengthRulers.at(i).unit);
    }

    // 3. 说明画在全部尺子之上，不会被后画的尺子压住
    for (int i = 0; i < labels.size(); ++i) {
        markshot::ruler::drawRulerLabel(painter, labels.at(i).first, labels.at(i).second, labelUnits.at(i));
    }
}

void ShotWindowRulerPainter::drawHover(QPainter &painter) const
{
    const ShotWindow &window = m_window;
    if (window.m_mode != Mode::Editing || window.m_tool != Tool::Ruler || !window.m_ruler.hoverValid) {
        return;
    }
    painter.save();
    painter.setFont(markshot::theme::uiFont(10, QFont::DemiBold));
    painter.setPen(QPen(QColor(45, 212, 191, 150), 1.0, Qt::DashLine));

    // 1. 十字线贯穿整张截图
    const QPointF imagePoint = window.m_ruler.hoverImagePoint;
    const QPointF hover = window.imageToWidget(imagePoint);
    const QRectF image = window.m_frozenImageRect;
    painter.drawLine(QPointF(image.left(), hover.y()), QPointF(image.right(), hover.y()));
    painter.drawLine(QPointF(hover.x(), image.top()), QPointF(hover.x(), image.bottom()));

    // 2. 坐标和取色说明放在指针右下方，靠近窗口边缘时翻到另一侧
    const QString text = QStringLiteral("x %1  y %2  %3")
                             .arg(qRound(imagePoint.x()))
                             .arg(qRound(imagePoint.y()))
                             .arg(colorHexRgb(window.sampledImageColor(imagePoint)));
    const QFontMetrics metrics(painter.font());
    QRectF label(hover.x() + 14.0, hover.y() + 14.0, metrics.horizontalAdvance(text) + 18.0, metrics.height() + 10.0);
    if (label.right() > window.width() - 8.0) {
        label.moveRight(hover.x() - 14.0);
    }
    if (label.bottom() > window.height() - 8.0) {
        label.moveBottom(hover.y() - 14.0);
    }
    label = markshot::ruler::clampRulerFloatingRect(label, QRectF(window.rect()), 8.0);
    drawRoundedLabel(painter, label, text);
    painter.restore();
}

void ShotWindowRulerPainter::drawSelectedFrame(QPainter &painter, const Annotation &annotation) const
{
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    // 1. 长度尺画出测量范围，角度尺只画三个控制点
    if (annotation.rulerMode != RulerMode::Angle) {
        painter.setPen(QPen(QColor(251, 146, 60), 2.0, Qt::DashLine));
        painter.setBrush(Qt::NoBrush);
        painter.drawRect(m_window.imageRectToWidget(markshot::ruler::rulerBounds(annotation)));
    }

    // 2. 控制点与其他标注一致：橙色实心，白色描边便于在深色背景上分辨
    painter.setPen(QPen(QColor(255, 255, 255), 1.5));
    painter.setBrush(QColor(251, 146, 60));
    const qreal tolerance = ShotWindowRulerTool::imageTolerance(m_window);
    for (const markshot::ruler::RulerHandle &handle : markshot::ruler::rulerHandles(annotation, tolerance)) {
        const QPointF point = m_window.imageToWidget(handle.point);
        if (annotation.rulerMode == RulerMode::Angle) {
            painter.drawEllipse(point, 6.0, 6.0);
        } else {
            painter.drawRoundedRect(QRectF(point.x() - 5.0, point.y() - 5.0, 10.0, 10.0), 2.0, 2.0);
        }
    }
    painter.restore();
}
