#include "translation_layout/block_painter.h"

#include <QPainter>

#include <algorithm>

namespace markshot::translation_layout {

void paintTranslatedBlocks(QPainter &painter, const QVector<TranslatedBlock> &blocks)
{
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setRenderHint(QPainter::TextAntialiasing, true);

    // 1. 底板用原文背景色，小圆角让边缘与周围像素自然衔接
    painter.setPen(Qt::NoPen);
    for (const TranslatedBlock &block : blocks) {
        const qreal radius = std::min<qreal>(3.0, block.plateRect.height() / 4.0);
        painter.setBrush(block.background);
        painter.drawRoundedRect(block.plateRect, radius, radius);
    }

    // 2. 译文沿用原文文字颜色，按排版结果逐行画在基线上
    for (const TranslatedBlock &block : blocks) {
        painter.setFont(block.font);
        painter.setPen(block.foreground);
        for (const TranslatedLine &line : block.lines) {
            painter.drawText(QPointF(line.rect.left(), line.baseline), line.text);
        }
    }
    painter.restore();
}

}  // namespace markshot::translation_layout
