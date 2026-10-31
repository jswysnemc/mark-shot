#pragma once

#include "translation_layout/translated_block.h"

#include <QVector>

class QPainter;

namespace markshot::translation_layout {

/**
 * 在图片坐标系中绘制译文：先画全部底板盖住原文，再逐行画译文，相邻段落的底板不会压住已画的文字。
 * 调用方负责把画笔变换到图片像素坐标并设置裁剪。
 * @param painter 已变换到图片像素坐标的画笔。
 * @param blocks 排好的译文段落。
 * @return 无返回值。
 */
void paintTranslatedBlocks(QPainter &painter, const QVector<TranslatedBlock> &blocks);

}  // namespace markshot::translation_layout
