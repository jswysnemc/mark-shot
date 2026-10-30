#pragma once

#include <QRectF>
#include <QSizeF>
#include <QString>
#include <QVector>

class QFontMetricsF;
class QPainter;

namespace markshot::startup_ruler {

/// 一把尺子在图像和窗口中的范围
struct RulerGeometry {
    // 测量范围，图像像素坐标，刻度和尺寸说明按它计算
    QRectF imageRect;
    // 测量范围，窗口坐标，绘制位置按它计算
    QRectF widgetRect;
};

/**
 * 把浮动标签移回窗口内，四周至少留 8 像素。
 * @param rect 标签原始位置，窗口坐标。
 * @param windowSize 窗口尺寸。
 * @return 移回窗口内的标签位置。
 */
QRectF clampRulerFloatingRect(QRectF rect, const QSizeF &windowSize);

/**
 * 计算一把尺子的测量框连同四边刻度和刻度数字占用的范围。
 * @param widgetRect 尺子测量范围，窗口坐标。
 * @return 占用范围，窗口坐标。
 */
QRectF rulerScaleBounds(const QRectF &widgetRect);

/**
 * 为尺寸说明选位置：依次尝试尺子下、上、右、左，避开自身刻度和已占用的范围，都被占用时放在下方。
 * @param widgetRect 尺子测量范围，窗口坐标。
 * @param labelSize 尺寸说明标签大小。
 * @param windowSize 窗口尺寸。
 * @param occupied 已占用的范围，例如其他标签和其他尺子的刻度，窗口坐标。
 * @return 尺寸说明标签位置，窗口坐标。
 */
QRectF placeRulerInfoLabel(const QRectF &widgetRect,
                           const QSizeF &labelSize,
                           const QSizeF &windowSize,
                           const QVector<QRectF> &occupied);

/**
 * 生成尺寸说明文字。
 * @param imageRect 测量范围，图像像素坐标。
 * @return 宽高、对角线和面积说明。
 */
QString rulerInfoText(const QRectF &imageRect);

/**
 * 为全部尺子的尺寸说明排位置：说明避开每把尺子的刻度，后排的说明避开先排的说明。
 * @param rulers 全部尺子，按拖出先后排列。
 * @param metrics 尺寸说明使用的字体度量。
 * @param windowSize 窗口尺寸。
 * @param occupied 尺子之外已占用的范围，例如悬停坐标标签，窗口坐标。
 * @return 与 rulers 一一对应的说明位置，窗口坐标。
 */
QVector<QRectF> layoutRulerInfoLabels(const QVector<RulerGeometry> &rulers,
                                      const QFontMetricsF &metrics,
                                      const QSizeF &windowSize,
                                      QVector<QRectF> occupied);

/**
 * 绘制一把尺子的测量框和四边刻度。
 * @param painter 当前绘制器，刻度数字使用其字体，调用前后状态不变。
 * @param ruler 尺子范围。
 * @return 无返回值。
 */
void drawRulerScale(QPainter &painter, const RulerGeometry &ruler);

}  // namespace markshot::startup_ruler
