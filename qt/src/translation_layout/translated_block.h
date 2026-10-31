#pragma once

#include "ocr_result.h"

#include <QColor>
#include <QFont>
#include <QPointF>
#include <QRectF>
#include <QString>
#include <QVector>

namespace markshot::translation_layout {

/// @brief 【译文排版】【译文行】排好的一行译文，图片像素坐标
struct TranslatedLine {
    QString text;
    // 文字实际占用的矩形，供拖选命中与高亮
    QRectF rect;
    // 基线纵坐标
    qreal baseline = 0.0;
};

/// @brief 【译文排版】【译文段落】一段译文的底板、配色、字体和排好的行，图片像素坐标
struct TranslatedBlock {
    // 原文段落包围盒
    QRectF sourceBox;
    // 覆盖原文的底板，比原文框略大，译文放不下时向下延伸
    QRectF plateRect;
    QColor background;
    QColor foreground;
    // 按像素设定字号的字体，排版和绘制使用同一个字体
    QFont font;
    // 整段译文
    QString text;
    QVector<TranslatedLine> lines;
};

/**
 * 把各段译文按段落顺序拼成全文，段与段之间换行。
 * @param blocks 排好的译文段落。
 * @return 全文。
 */
QString translatedText(const QVector<TranslatedBlock> &blocks);

/**
 * 把排好的译文行转换成 OCR token：每行一个 token，行号连续编号，供贴图拖选复制译文。
 * @param blocks 排好的译文段落。
 * @return token 列表，图片像素坐标。
 */
QVector<markshot::ocr::Token> translatedLineTokens(const QVector<TranslatedBlock> &blocks);

/**
 * 把译文段落整体平移，并去掉平移后底板落在范围外的段落。
 * @param blocks 排好的译文段落。
 * @param offset 平移量。
 * @param bounds 保留范围，通常是新图片的矩形。
 * @return 平移并筛选后的译文段落。
 */
QVector<TranslatedBlock> offsetBlocks(const QVector<TranslatedBlock> &blocks, const QPointF &offset,
                                      const QRectF &bounds);

}  // namespace markshot::translation_layout
