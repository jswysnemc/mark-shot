#pragma once

#include <QColor>
#include <QFont>
#include <QRectF>
#include <QSizeF>
#include <QString>
#include <QVector>

class QFontMetricsF;
class QPainter;

namespace markshot::ruler {

/// 一把长度尺在图像和绘制坐标中的范围及样式
struct RulerGeometry {
    // 测量范围，图像像素坐标，刻度和尺寸说明按它计算
    QRectF imageRect;
    // 测量范围，绘制坐标，绘制位置按它计算
    QRectF paintRect;
    // 刻度、字号和间距的缩放倍数，1 对应默认粗细下的屏幕尺寸
    qreal unit = 1.0;
    // 测量框、刻度和刻度数字的颜色
    QColor color = QColor(45, 212, 191);
};

/**
 * 按标注粗细换算尺子的缩放倍数，默认粗细 3 对应 1，粗细越大刻度和文字越大。
 * @param width 标注粗细。
 * @return 缩放倍数。
 */
qreal rulerUnitForWidth(qreal width);

/**
 * 返回尺子说明和刻度数字使用的字体，字号按像素设置，屏幕和导出图片尺寸一致。
 * @param unit 缩放倍数。
 * @return 字体。
 */
QFont rulerFont(qreal unit);

/**
 * 把浮动标签移回边界内，四周至少留一段间距。
 * @param rect 标签原始位置。
 * @param bounds 允许放置的范围。
 * @param gap 与边界的间距。
 * @return 移回边界内的标签位置。
 */
QRectF clampRulerFloatingRect(QRectF rect, const QRectF &bounds, qreal gap);

/**
 * 计算一把尺子的测量框连同四边刻度和刻度数字占用的范围。
 * @param paintRect 测量范围，绘制坐标。
 * @param unit 缩放倍数。
 * @return 占用范围，绘制坐标。
 */
QRectF rulerScaleBounds(const QRectF &paintRect, qreal unit);

/**
 * 为尺寸说明选位置：依次尝试尺子下、上、右、左，避开自身刻度和已占用的范围，都被占用时放在下方。
 * @param ruler 尺子范围及缩放倍数。
 * @param labelSize 尺寸说明标签大小。
 * @param bounds 标签允许放置的范围。
 * @param occupied 已占用的范围，例如其他标签和其他尺子的刻度。
 * @return 尺寸说明标签位置。
 */
QRectF placeRulerInfoLabel(const RulerGeometry &ruler,
                           const QSizeF &labelSize,
                           const QRectF &bounds,
                           const QVector<QRectF> &occupied);

/**
 * 生成尺寸说明文字。
 * @param imageRect 测量范围，图像像素坐标。
 * @return 宽高、对角线和面积说明。
 */
QString rulerInfoText(const QRectF &imageRect);

/**
 * 按文字和缩放倍数计算说明标签大小。
 * @param metrics 说明使用的字体度量。
 * @param text 说明文字。
 * @param unit 缩放倍数。
 * @return 标签大小。
 */
QSizeF rulerLabelSize(const QFontMetricsF &metrics, const QString &text, qreal unit);

/**
 * 为全部尺子的尺寸说明排位置：说明避开每把尺子的刻度，后排的说明避开先排的说明。
 * @param rulers 全部长度尺，按绘制先后排列。
 * @param bounds 标签允许放置的范围。
 * @param occupied 尺子之外已占用的范围，例如角度说明。
 * @return 与 rulers 一一对应的说明位置。
 */
QVector<QRectF> layoutRulerInfoLabels(const QVector<RulerGeometry> &rulers,
                                      const QRectF &bounds,
                                      QVector<QRectF> occupied);

/**
 * 绘制深色圆角说明标签。
 * @param painter 当前绘制器，调用前后状态不变。
 * @param rect 标签位置。
 * @param text 说明文字。
 * @param unit 缩放倍数。
 * @return 无返回值。
 */
void drawRulerLabel(QPainter &painter, const QRectF &rect, const QString &text, qreal unit);

/**
 * 绘制一把尺子的测量框和四边刻度。
 * @param painter 当前绘制器，调用前后状态不变。
 * @param ruler 尺子范围及样式。
 * @return 无返回值。
 */
void drawRulerScale(QPainter &painter, const RulerGeometry &ruler);

}  // namespace markshot::ruler
