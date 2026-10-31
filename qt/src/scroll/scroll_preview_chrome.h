#pragma once

#include "ui/toolbar_palette.h"

#include <QFontMetrics>
#include <QPainter>
#include <QPainterPath>

#include <algorithm>

namespace markshot::scroll {

/// @brief 【滚动截图】【预览背景】使用工具栏主题绘制背景和单像素细边框
/// @param painter 当前绘图器
/// @param rect 预览区域的逻辑坐标
/// @param rounded 是否绘制圆角面板
/// @param palette 当前截图会话的配色
/// @return 无返回值
inline void drawScrollPreviewSurface(QPainter &painter, const QRect &rect, bool rounded = false,
                                     const ui::ToolbarPalette &palette = {})
{
    if (rect.isEmpty()) {
        return;
    }
    painter.save();
    const QRectF bounds = QRectF(rect).adjusted(0.5, 0.5, -0.5, -0.5);
    painter.setPen(QPen(palette.separator, 1));
    painter.setBrush(palette.listFill);
    if (rounded) {
        painter.drawRoundedRect(bounds, 10, 10);
    } else {
        painter.drawRect(bounds);
    }
    painter.restore();
}

/// @brief 【滚动截图】【预览状态】按主题绘制静态状态点和文字，避免动画引起闲置重绘
/// @param painter 当前绘图器
/// @param rect 紧凑状态行的逻辑坐标
/// @param text 当前捕获状态，空间不足时省略尾部
/// @param metric 右侧常驻的拼接长度，为空时不占位
/// @param palette 当前截图会话的配色
/// @return 无返回值
inline void drawScrollPreviewStatus(QPainter &painter, const QRect &rect, const QString &text,
                                    const QString &metric = {}, const ui::ToolbarPalette &palette = {})
{
    if (rect.isEmpty()) {
        return;
    }
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing);
    // 1. 【滚动截图】【预览状态】状态含义由文字表达，青点只承担轻量视觉提示
    painter.setPen(Qt::NoPen);
    painter.setBrush(palette.sliderEnd);
    painter.drawEllipse(QPointF(rect.left() + 4, rect.center().y() + 0.5), 3, 3);
    QFont font = painter.font();
    font.setPointSize(9);
    font.setBold(false);
    painter.setFont(font);
    const QFontMetrics metrics(font);
    // 2. 【滚动截图】【拼接长度】右侧固定显示已拼接像素，等待滚动时也不收回
    const int metricWidth = metric.isEmpty() ? 0 : metrics.horizontalAdvance(metric) + 10;
    painter.setPen(palette.strongText);
    const QRect textRect = rect.adjusted(15, 0, -metricWidth, 0);
    painter.drawText(textRect, Qt::AlignVCenter | Qt::AlignLeft,
                     metrics.elidedText(text, Qt::ElideRight, std::max(0, textRect.width())));
    if (!metric.isEmpty()) {
        painter.setPen(palette.mutedText);
        painter.drawText(rect.adjusted(0, 0, -2, 0), Qt::AlignVCenter | Qt::AlignRight, metric);
    }
    painter.restore();
}

}
