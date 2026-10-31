#pragma once

#include "shot_window_types.h"

#include <QPointF>
#include <QRectF>

class QPainter;
class ShotWindow;

namespace markshot::ruler {
struct AngleGeometry;
struct RulerGeometry;
}

/// 绘制截图窗口中的尺子：测量框和角度、说明标签、悬停十字线和选中控制点
class ShotWindowRulerPainter {
public:
    using Annotation = markshot::shot::types::Annotation;

    explicit ShotWindowRulerPainter(const ShotWindow &window);

    /**
     * 绘制一把尺子的测量框或角度，说明标签由 drawLabels 统一排版。
     * @param painter 当前绘制器，调用前后状态不变。
     * @param annotation 尺子标注。
     * @param widgetCoordinates true 表示画到窗口，false 表示画到导出图片。
     * @return 无返回值。
     */
    void drawBody(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const;

    /**
     * 统一排版并绘制全部尺子的说明标签，标签互不重叠且留在选区内，画到窗口时还留在窗口内。
     * @param painter 当前绘制器，调用前后状态不变。
     * @param widgetCoordinates true 表示画到窗口并包含草稿，false 表示画到导出图片。
     * @return 无返回值。
     */
    void drawLabels(QPainter &painter, bool widgetCoordinates) const;

    /**
     * 尺子工具下绘制悬停十字线和坐标取色说明，只画在窗口中，不进入导出图片。
     * @param painter 当前绘制器，调用前后状态不变。
     * @return 无返回值。
     */
    void drawHover(QPainter &painter) const;

    /**
     * 绘制选中尺子的虚线范围、控制点和旋转把手，长度尺的范围随测量框旋转；
     * 角度尺的范围是 angleFrame，另画四角和四边中点的缩放控制点。
     * @param painter 当前绘制器，调用前后状态不变。
     * @param annotation 选中的尺子。
     * @return 无返回值。
     */
    void drawSelectedFrame(QPainter &painter, const Annotation &annotation) const;

    /**
     * 计算角度尺选中框：三点外接矩形向外留出固定的窗口像素，选中框的绘制、拖动命中和删除按钮共用。
     * @param annotation 角度尺。
     * @return 选中框，图像像素坐标；没有定点时为空。
     */
    QRectF angleFrame(const Annotation &annotation) const;

    /**
     * 计算旋转把手到尺子范围上边的距离：长度尺越过上边的刻度数字，角度尺越过外扩的选中框。
     * @param annotation 尺子标注。
     * @return 距离，窗口像素。
     */
    qreal rotationHandleGap(const Annotation &annotation) const;

private:
    qreal handleClearance(const Annotation &annotation) const;
    qreal unitFor(const Annotation &annotation, bool widgetCoordinates) const;
    QRectF labelBounds(bool widgetCoordinates) const;
    markshot::ruler::RulerGeometry lengthGeometry(const Annotation &annotation, bool widgetCoordinates) const;
    markshot::ruler::AngleGeometry angleGeometry(const Annotation &annotation, bool widgetCoordinates) const;
    QPointF mapPoint(QPointF imagePoint, bool widgetCoordinates) const;
    QRectF mapRect(const QRectF &imageRect, bool widgetCoordinates) const;

    const ShotWindow &m_window;
};
