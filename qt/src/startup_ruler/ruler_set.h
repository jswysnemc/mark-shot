#pragma once

#include <QLineF>
#include <QVector>

namespace markshot::startup_ruler {

/**
 * 判断一把尺子是否量到了尺寸。宽和高都不足 1 像素时视为单击，不算测量。
 * @param ruler 尺子的起点和终点，图像坐标。
 * @return 量到尺寸时返回 true。
 */
bool rulerMeasures(const QLineF &ruler);

/**
 * 拖动结束后移除最后一把没有量到尺寸的尺子，单击不会留下空尺子。
 * @param rulers 当前全部尺子，最后一项是刚拖动的尺子。
 * @return 移除了尺子时返回 true。
 */
bool dropUnmeasuredLastRuler(QVector<QLineF> &rulers);

}  // namespace markshot::startup_ruler
