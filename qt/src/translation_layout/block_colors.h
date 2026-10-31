#pragma once

#include <QColor>
#include <QImage>
#include <QRectF>

namespace markshot::translation_layout {

/// @brief 【译文排版】【取色】原文段落的背景色和文字颜色
struct BlockColors {
    QColor background;
    QColor foreground;
};

/**
 * 从原图取出段落的背景色和文字颜色：文字框外侧一圈出现最多的颜色作为背景，
 * 框内与背景差别最大的一批像素的平均色作为文字颜色，两者对比不足时文字改用黑色或白色。
 * @param image 原图，像素坐标与段落框一致。
 * @param box 段落包围盒。
 * @param lineHeight 段落行高，决定外侧取样宽度。
 * @return 背景色和文字颜色，图片或段落为空时返回白底深色字。
 */
BlockColors sampleBlockColors(const QImage &image, const QRectF &box, qreal lineHeight);

}  // namespace markshot::translation_layout
