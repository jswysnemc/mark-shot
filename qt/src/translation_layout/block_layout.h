#pragma once

#include "translation_layout/paragraph_blocks.h"
#include "translation_layout/translated_block.h"

#include <QFont>
#include <QImage>
#include <QString>
#include <QVector>

namespace markshot::translation_layout {

/**
 * 把各段译文排进原文段落的位置：从原图取背景色和文字颜色，字号从原文行高起步，
 * 按段落宽度换行，放不下时逐步缩小字号，最小字号仍放不下时底板向下延伸到下一段或图片底边，
 * 再放不下的行截断并加省略号。
 * @param image 原图，像素坐标与段落框一致。
 * @param blocks 原文段落。
 * @param texts 与段落一一对应的译文，空字符串的段落不排版。
 * @param baseFont 译文字体，只取字族和字重，字号按段落重新设定。
 * @return 排好的译文段落，顺序与原文段落一致。
 */
QVector<TranslatedBlock> layoutTranslatedBlocks(const QImage &image, const QVector<SourceBlock> &blocks,
                                                const QVector<QString> &texts, const QFont &baseFont);

}  // namespace markshot::translation_layout
