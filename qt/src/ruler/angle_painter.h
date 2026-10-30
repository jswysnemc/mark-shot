#pragma once

#include <QColor>
#include <QPointF>
#include <QRectF>
#include <QString>
#include <QVector>

class QPainter;

namespace markshot::ruler {

/// 一把角度尺在图像和绘制坐标中的位置及样式
struct AngleGeometry {
    // 顶点、第一条边端点、第二条边端点，图像像素坐标，度数按它计算；不足三点时只画已有的边
    QVector<QPointF> imagePoints;
    // 与 imagePoints 一一对应的绘制坐标
    QVector<QPointF> paintPoints;
    // 线宽、圆点和字号的缩放倍数
    qreal unit = 1.0;
    // 只由粗细决定的放大倍数，不含截图和窗口缩放；圆弧占边长的比例随它放大
    qreal sizeScale = 1.0;
    // 边、圆弧和端点的颜色
    QColor color = QColor(45, 212, 191);
};

/**
 * 计算两条边之间较小的夹角。
 * @param vertex 顶点。
 * @param first 第一条边端点。
 * @param second 第二条边端点。
 * @return 0 到 180 之间的度数，任一条边长度为零时返回 0。
 */
qreal angleDegrees(QPointF vertex, QPointF first, QPointF second);

/**
 * 生成角度说明文字，保留一位小数。
 * @param degrees 度数。
 * @return 说明文字。
 */
QString angleInfoText(qreal degrees);

/**
 * 计算角度说明标签的位置：放在角平分线方向的圆弧外侧，并移回边界内。
 * @param angle 三点齐全的角度尺。
 * @param bounds 标签允许放置的范围。
 * @return 标签位置；不足三点时返回空矩形。
 */
QRectF angleInfoLabelRect(const AngleGeometry &angle, const QRectF &bounds);

/**
 * 绘制角度尺的两条边、端点和顶点处的圆弧。
 * @param painter 当前绘制器，调用前后状态不变。
 * @param angle 角度尺位置及样式。
 * @return 无返回值。
 */
void drawAngleBody(QPainter &painter, const AngleGeometry &angle);

}  // namespace markshot::ruler
