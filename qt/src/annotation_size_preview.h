#pragma once

#include "shot_window_types.h"
#include "ui/toolbar_palette.h"

#include <QColor>
#include <QPointF>
#include <QRectF>
#include <QVector>

class QPainter;

namespace markshot::shot {

/// @brief 标注粗细预览的参数，尺寸沿用标注模型，缩放用于换算窗口内轮廓
struct AnnotationSizePreview {
    types::Tool tool;
    qreal width;
    qreal imageScale;
    QColor color;
};

/// @brief 绘制固定十字、尺寸轮廓和避开控件的紧凑数值提示
/// @param painter 目标绘制器，返回时恢复原有绘制状态
/// @param preview 实际工具、尺寸、缩放比例和标注颜色
/// @param position 光标热点的窗口坐标
/// @param viewport 可用于显示提示的窗口区域
/// @param obstacles 提示需要避开的可见控件区域
/// @param palette 截图会话的工具栏配色，数值提示跟随明暗主题
/// @return 无返回值
void drawAnnotationSizePreview(QPainter &painter, const AnnotationSizePreview &preview,
                               QPointF position, QRectF viewport, const QVector<QRect> &obstacles,
                               const ui::ToolbarPalette &palette = {});

}  // namespace markshot::shot
