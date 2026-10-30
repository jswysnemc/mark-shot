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
 * 计算尺子未旋转时的命中和选中范围，宽或高不足 1 像素时向两侧补足，平直的尺子也有可用范围。
 * @param annotation 尺子标注。
 * @return 范围，图像像素坐标；长度尺旋转时是旋转前的测量框。
 */
QRectF rulerBounds(const Annotation &annotation);

/**
 * 列出尺子的控制点：长度尺为四角，边长足够时再加四边中点，随测量框旋转；角度尺为顶点和两条边的端点。
 * @param annotation 尺子标注。
 * @param tolerance 命中半径，图像像素，用来判断边长是否够放中点。
 * @return 控制点列表，角点在前，位置为旋转后的图像像素坐标。
 */
QVector<RulerHandle> rulerHandles(const Annotation &annotation, qreal tolerance);

/**
 * 把角度尺的三个点绕轴心旋转，角度尺的旋转直接改写定点，夹角不变。
 * @param annotation 角度尺，原地修改。
 * @param center 旋转轴心，图像像素坐标。
 * @param degrees 旋转角度，屏幕上顺时针为正。
 * @return 无返回值。
 */
void rotateAnglePoints(Annotation &annotation, QPointF center, qreal degrees);

/**
 * 计算角度尺第一条边的方向，供 Shift 旋转时把这条边吸附到 45 度倍数。
 * @param annotation 角度尺。
 * @return 方向角度，向右为 0、屏幕上顺时针为正；点不足或边长为零时为 0。
 */
qreal angleFirstArmDegrees(const Annotation &annotation);

/**
 * 判断指针在尺子上命中的调整方式：先找控制点，再找边框或边；长度尺内部是否算作移动由参数决定。
 * 旋转的长度尺先把指针换算回测量框未旋转时的位置再判断。
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
 * 按控制点拖动结果计算新尺子：长度尺对角或对边在屏幕上保持不动，旋转后也一样；角度尺只移动被拖的点。
 * @param before 拖动前的尺子。
 * @param drag 调整方式。
 * @param point 指针位置，图像像素坐标，调用方已限制在截图范围内。
 * @return 调整后的尺子。
 */
Annotation draggedRuler(const Annotation &before, SelectionDrag drag, QPointF point);

}  // namespace markshot::ruler
