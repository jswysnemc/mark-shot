#pragma once

#include "shot_window_types.h"

#include <QSize>

namespace markshot::shot {

/**
 * 按拖动目标调整截图选区，并保留最小尺寸和图像边界
 * @param before 调整前的选区
 * @param handle 拖动的边、角或整个选区
 * @param dragStart 开始拖动时的图像坐标
 * @param pointer 当前指针的图像坐标
 * @param imageSize 图像尺寸
 * @return 调整后的选区，无效输入返回原选区
 */
QRectF adjustedSelectionRect(QRectF before, types::SelectionDrag handle,
                             QPointF dragStart, QPointF pointer, QSize imageSize);

/**
 * 把指针吸附到当前调整的选区边或角，避免方向键首次调整发生跳变
 * @param selection 当前选区
 * @param handle 调整目标
 * @param pointer 未调整方向上保留的指针坐标
 * @return 调整目标上的坐标；移动整个选区时返回原指针坐标
 */
QPointF selectionAdjustmentPoint(QRectF selection, types::SelectionDrag handle, QPointF pointer);

}
