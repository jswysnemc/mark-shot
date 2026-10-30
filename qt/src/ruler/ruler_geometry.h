#pragma once

#include "shot_window_types.h"

#include <QPointF>
#include <QRectF>
#include <QVector>

namespace markshot::ruler {

using markshot::shot::types::Annotation;
using markshot::shot::types::SelectionDrag;

/// 尺子上可拖动的一个控制点
struct RulerHandle {
    // 控制点位置，图像像素坐标
    QPointF point;
    // 拖动该点时的调整方式
    SelectionDrag drag = SelectionDrag::None;
};

/**
 * 判断尺子是否已经量到内容：长度尺宽或高至少 1 像素，角度尺三点齐全且两条边都不短于 1 像素。
 * @param annotation 尺子标注。
 * @return 量到内容时返回 true。
 */
bool rulerMeasures(const Annotation &annotation);

/**
 * 计算尺子的命中和选中范围，宽或高不足 1 像素时向两侧补足，平直的尺子也有可用范围。
 * @param annotation 尺子标注。
 * @return 范围，图像像素坐标。
 */
QRectF rulerBounds(const Annotation &annotation);

/**
 * 列出尺子的控制点：长度尺为四角，边长足够时再加四边中点；角度尺为顶点和两条边的端点。
 * @param annotation 尺子标注。
 * @param tolerance 命中半径，图像像素，用来判断边长是否够放中点。
 * @return 控制点列表，角点在前。
 */
QVector<RulerHandle> rulerHandles(const Annotation &annotation, qreal tolerance);

/**
 * 判断指针在尺子上命中的调整方式：先找控制点，再找边框或边；长度尺内部是否算作移动由参数决定。
 * @param annotation 尺子标注。
 * @param point 指针位置，图像像素坐标。
 * @param tolerance 命中半径，图像像素。
 * @param interiorMoves 为 true 时长度尺内部也算移动，尺子工具中为 false，内部留给新的测量。
 * @return 调整方式，未命中时返回 None。
 */
SelectionDrag rulerDragAt(const Annotation &annotation, QPointF point, qreal tolerance, bool interiorMoves);

/**
 * 判断调整方式是否只改变尺子形状，移动整把尺子不在其中。
 * @param drag 调整方式。
 * @return 是控制点调整时返回 true。
 */
bool isRulerHandleDrag(SelectionDrag drag);

/**
 * 按控制点拖动结果计算新尺子：长度尺对角或对边保持不动，角度尺只移动被拖的点。
 * @param before 拖动前的尺子。
 * @param drag 调整方式。
 * @param point 指针位置，图像像素坐标，调用方已限制在截图范围内。
 * @return 调整后的尺子。
 */
Annotation draggedRuler(const Annotation &before, SelectionDrag drag, QPointF point);

}  // namespace markshot::ruler
