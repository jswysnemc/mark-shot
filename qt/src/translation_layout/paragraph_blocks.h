#pragma once

#include "ocr_result.h"

#include <QRectF>
#include <QString>
#include <QVector>

namespace markshot::translation_layout {

/// @brief 【译文排版】【段落】由相邻 OCR 行合并出的一段原文，按段送去翻译并在原位置排版
struct SourceBlock {
    // 段落文字，行与行之间按语言规则拼接
    QString text;
    // 段落包围盒，图片像素坐标
    QRectF box;
    // 段内各行高度的中位数，决定译文字号
    qreal lineHeight = 0.0;
    // 相邻两行顶边的平均距离，单行段落为 0
    qreal linePitch = 0.0;
    // 段内各行的包围盒，自上而下
    QVector<QRectF> lineBoxes;
};

/**
 * 把 OCR token 按行拼成文本行，再把字号相近、左边或中线对齐、间距正常且上一行写满的相邻行合并成段落。
 * 同一行内横向空隙过大的 token 拆成两行，避免把两栏文字当成一句话。
 * @param tokens OCR 识别结果，图片像素坐标。
 * @return 段落列表，顺序与段落首行出现的先后一致。
 */
QVector<SourceBlock> groupParagraphs(const QVector<markshot::ocr::Token> &tokens);

/**
 * 把下一行文字接到段落末尾：中日韩文字直接相连，西文补空格，行尾连字符断开的单词重新拼回。
 * @param paragraph 已有段落文字。
 * @param nextLine 下一行文字。
 * @return 拼接后的段落文字。
 */
QString joinParagraphLine(const QString &paragraph, const QString &nextLine);

}  // namespace markshot::translation_layout
