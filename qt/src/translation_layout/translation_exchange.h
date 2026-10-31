#pragma once

#include "translation_layout/paragraph_blocks.h"

#include <QByteArray>
#include <QSize>
#include <QString>
#include <QVector>

namespace markshot::translation_layout {

/**
 * 组装翻译插件读取的 token JSON：每个段落一个 token，行号等于段落序号，翻译器按段翻译整句。
 * @param blocks 原文段落。
 * @param targetLanguage 目标语言。
 * @return 紧凑 JSON。
 */
QByteArray translationInputJson(const QVector<SourceBlock> &blocks, const QString &targetLanguage);

/**
 * 把翻译输出对应回段落：行号指向的段落与译文框相交时按行号对应，否则取相交面积最大的段落；
 * 同一段落收到多段译文时按输出顺序拼接。
 * @param output 翻译插件输出的 token JSON。
 * @param blocks 送去翻译的原文段落。
 * @param imageSize 图片尺寸，用于裁剪译文框。
 * @return 与段落一一对应的译文，没有对应译文的段落为空字符串。
 */
QVector<QString> translatedBlockTexts(const QByteArray &output, const QVector<SourceBlock> &blocks,
                                      QSize imageSize);

}  // namespace markshot::translation_layout
