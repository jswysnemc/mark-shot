#include "translation_layout/translation_exchange.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

namespace markshot::translation_layout {

namespace {

/**
 * 计算两个矩形相交部分的面积。
 * @param left 第一个矩形。
 * @param right 第二个矩形。
 * @return 相交面积，不相交时为 0。
 */
qreal intersectionArea(const QRectF &left, const QRectF &right)
{
    const QRectF overlap = left.intersected(right);
    return overlap.isEmpty() ? 0.0 : overlap.width() * overlap.height();
}

/**
 * 为一段译文找到对应的段落。
 * @param token 译文 token。
 * @param blocks 原文段落。
 * @return 段落序号，没有相交段落时返回 -1。
 */
int matchingBlock(const markshot::ocr::Token &token, const QVector<SourceBlock> &blocks)
{
    // 1. 行号指向的段落与译文框相交时直接采用
    if (token.line >= 0 && token.line < blocks.size()
        && intersectionArea(blocks.at(token.line).box, token.imageRect) > 0.0) {
        return token.line;
    }
    // 2. 翻译器改写了行号时，取相交面积最大的段落
    int best = -1;
    qreal bestArea = 0.0;
    for (int index = 0; index < blocks.size(); ++index) {
        const qreal area = intersectionArea(blocks.at(index).box, token.imageRect);
        if (area > bestArea) {
            bestArea = area;
            best = index;
        }
    }
    return best;
}

}  // namespace

QByteArray translationInputJson(const QVector<SourceBlock> &blocks, const QString &targetLanguage)
{
    QJsonArray tokens;
    for (int index = 0; index < blocks.size(); ++index) {
        const SourceBlock &block = blocks.at(index);
        QJsonObject object;
        object.insert(QStringLiteral("text"), block.text);
        object.insert(QStringLiteral("box"), markshot::ocr::rectToJson(block.box));
        object.insert(QStringLiteral("line"), index);
        object.insert(QStringLiteral("index"), 0);
        object.insert(QStringLiteral("confidence"), 1.0);
        tokens.append(object);
    }
    QJsonObject root;
    root.insert(QStringLiteral("targetLanguage"), targetLanguage);
    root.insert(QStringLiteral("tokens"), tokens);
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

QVector<QString> translatedBlockTexts(const QByteArray &output, const QVector<SourceBlock> &blocks,
                                      QSize imageSize)
{
    QVector<QString> texts(blocks.size());
    for (const markshot::ocr::Token &token : markshot::ocr::tokensFromJsonOutput(output, imageSize)) {
        const int block = matchingBlock(token, blocks);
        if (block < 0) {
            continue;
        }
        texts[block] = joinParagraphLine(texts.at(block), token.text);
    }
    return texts;
}

}  // namespace markshot::translation_layout
