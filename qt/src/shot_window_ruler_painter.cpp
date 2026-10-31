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

namespace {

// 角度尺选中框向外留出的距离，窗口像素，选中框的缩放控制点与落在外接矩形角上的定点分得开
constexpr qreal kAngleFramePadding = 14.0;
// 旋转把手连线的长度，窗口像素，从刻度数字或选中框外侧量起
constexpr qreal kRotationStemLength = 20.0;

}  // namespace

ShotWindowRulerPainter::ShotWindowRulerPainter(const ShotWindow &window)
    : m_window(window)
{
}

/**
 * 计算旋转把手连线起点到尺子范围上边的距离。
 * @param annotation 尺子标注。
 * @return 距离，窗口像素。
 */
qreal ShotWindowRulerPainter::handleClearance(const Annotation &annotation) const
{
    return annotation.rulerMode == RulerMode::Angle
        ? kAngleFramePadding
        : markshot::ruler::rulerTopReach(lengthGeometry(annotation, true));
}

qreal ShotWindowRulerPainter::rotationHandleGap(const Annotation &annotation) const
{
    return handleClearance(annotation) + kRotationStemLength;
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

/**
 * 计算尺子说明和刻度数字允许出现的范围：导出图片取选区，窗口中再与窗口求交，缩放平移后也留在屏幕上。
 * @param widgetCoordinates true 表示画到窗口。
 * @return 允许范围，绘制坐标。
 */
QRectF ShotWindowRulerPainter::labelBounds(bool widgetCoordinates) const
{
    const QRectF selection = mapRect(m_window.normalizedSelection(), widgetCoordinates);
    if (!widgetCoordinates) {
        return selection;
    }
    const QRectF visible = selection.intersected(QRectF(m_window.rect()));
    return visible.isEmpty() ? selection : visible;
}

/**
 * 汇总一把长度尺的绘制参数：测量范围、缩放倍数、旋转角度和刻度数字允许范围。
 * @param annotation 长度尺。
 * @param widgetCoordinates true 表示画到窗口。
 * @return 绘制参数。
 */
markshot::ruler::RulerGeometry ShotWindowRulerPainter::lengthGeometry(const Annotation &annotation,
                                                                      bool widgetCoordinates) const
{
    const QRectF imageRect = annotation.rect.normalized();
    return {imageRect,
            mapRect(imageRect, widgetCoordinates),
            unitFor(annotation, widgetCoordinates),
            annotation.color,
            annotation.rotationDegrees,
            labelBounds(widgetCoordinates)};
}

/**
 * 汇总一把角度尺的绘制参数：三个定点、缩放倍数、按粗细放大的倍数和颜色。
 * @param annotation 角度尺。
 * @param widgetCoordinates true 表示画到窗口。
 * @return 绘制参数。
 */
markshot::ruler::AngleGeometry ShotWindowRulerPainter::angleGeometry(const Annotation &annotation,
                                                                     bool widgetCoordinates) const
{
    markshot::ruler::AngleGeometry angle{annotation.points,
                                         {},
                                         unitFor(annotation, widgetCoordinates),
                                         markshot::ruler::rulerUnitForWidth(annotation.width),
                                         annotation.color};
    for (const QPointF &point : annotation.points) {
        angle.paintPoints.append(mapPoint(point, widgetCoordinates));
    }
    return angle;
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
    if (annotation.rulerMode == RulerMode::Angle) {
        markshot::ruler::drawAngleBody(painter, angleGeometry(annotation, widgetCoordinates));
        return;
    }
    // 1. 刚按下还没拉开的长度尺先不画
    if (!markshot::ruler::rulerMeasures(annotation)) {
        return;
    }
    markshot::ruler::drawRulerScale(painter, lengthGeometry(annotation, widgetCoordinates));
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
    const QRectF bounds = labelBounds(widgetCoordinates);

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
            const QRectF rect = markshot::ruler::angleInfoLabelRect(angleGeometry(*annotation, widgetCoordinates),
                                                                    bounds);
            const qreal degrees = markshot::ruler::angleDegrees(annotation->points.at(0),
                                                                annotation->points.at(1),
                                                                annotation->points.at(2));
            occupied.append(rect);
            labels.append({rect, markshot::ruler::angleInfoText(degrees)});
            labelUnits.append(unit);
        } else if (markshot::ruler::rulerMeasures(*annotation)) {
            lengthRulers.append(lengthGeometry(*annotation, widgetCoordinates));
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
    const ShotWindow &window = m_window;
    const QRectF bounds = markshot::ruler::rulerBounds(annotation);
    if (bounds.isEmpty()) {
        return;
    }
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    // 1. 虚线范围：长度尺沿旋转后的测量框，角度尺取三点的外接矩形并向外留出间距，不压住两条边
    const QPointF center = bounds.center();
    const qreal rotation = window.annotationSupportsRotation(annotation) ? annotation.rotationDegrees : 0.0;
    QPolygonF frame;
    if (annotation.rulerMode == RulerMode::Angle) {
        frame = QPolygonF(window.imageRectToWidget(angleFrame(annotation)));
    } else {
        for (const QPointF &corner : {bounds.topLeft(), bounds.topRight(), bounds.bottomRight(), bounds.bottomLeft()}) {
            frame.append(window.imageToWidget(window.rotatedPoint(corner, center, rotation)));
        }
    }
    painter.setPen(QPen(QColor(251, 146, 60), 2.0, Qt::DashLine));
    painter.setBrush(Qt::NoBrush);
    painter.drawPolygon(frame);

    // 2. 旋转把手在范围上边中点外侧，连线从刻度数字或选中框外侧开始，样式与其他标注一致
    const QPointF topCenter =
        window.imageToWidget(window.rotatedPoint(QPointF(center.x(), bounds.top()), center, rotation));
    const QPointF rotateHandle = window.annotationRotationHandlePoint(annotation, true);
    const QLineF stem(topCenter, rotateHandle);
    const QPointF stemStart = stem.length() > 0.0 ? stem.pointAt(handleClearance(annotation) / stem.length()) : topCenter;
    painter.setPen(QPen(QColor(251, 146, 60), 1.8, Qt::SolidLine, Qt::RoundCap));
    painter.drawLine(stemStart, rotateHandle);
    painter.setBrush(QColor(251, 146, 60));
    painter.setPen(QPen(QColor(255, 255, 255), 1.5));
    painter.drawEllipse(rotateHandle, 6.0, 6.0);

    // 3. 控制点与其他标注一致：橙色实心，白色描边便于在深色背景上分辨；角度尺选中框另有四角和四边中点的缩放点
    auto drawSquare = [&painter](QPointF point) {
        painter.drawRoundedRect(QRectF(point.x() - 5.0, point.y() - 5.0, 10.0, 10.0), 2.0, 2.0);
    };
    if (annotation.rulerMode == RulerMode::Angle) {
        for (const QPointF &point : window.selectionHandlePoints(window.imageRectToWidget(angleFrame(annotation)))) {
            drawSquare(point);
        }
    }
    const qreal tolerance = ShotWindowRulerTool::imageTolerance(window);
    for (const markshot::ruler::RulerHandle &handle : markshot::ruler::rulerHandles(annotation, tolerance)) {
        const QPointF point = window.imageToWidget(handle.point);
        if (annotation.rulerMode == RulerMode::Angle) {
            painter.drawEllipse(point, 6.0, 6.0);
        } else {
            drawSquare(point);
        }
    }
    painter.restore();
}

QRectF ShotWindowRulerPainter::angleFrame(const Annotation &annotation) const
{
    const QRectF bounds = markshot::ruler::rulerBounds(annotation);
    if (bounds.isEmpty()) {
        return {};
    }
    // 1. 留白按窗口像素给定，换算到图像像素，缩放画布时选中框与定点的距离不变
    const qreal padding = kAngleFramePadding / std::max<qreal>(0.001, m_window.annotationSizeScale(true));
    return bounds.adjusted(-padding, -padding, padding, padding);
}
