#include "translation_layout/translated_block.h"

#include <QStringList>

namespace markshot::translation_layout {

QString translatedText(const QVector<TranslatedBlock> &blocks)
{
    QStringList paragraphs;
    for (const TranslatedBlock &block : blocks) {
        if (!block.text.trimmed().isEmpty()) {
            paragraphs.append(block.text.trimmed());
        }
    }
    return paragraphs.join(QLatin1Char('\n'));
}

QVector<markshot::ocr::Token> translatedLineTokens(const QVector<TranslatedBlock> &blocks)
{
    QVector<markshot::ocr::Token> tokens;
    int line = 0;
    for (const TranslatedBlock &block : blocks) {
        for (const TranslatedLine &translatedLine : block.lines) {
            if (translatedLine.text.trimmed().isEmpty() || translatedLine.rect.isEmpty()) {
                continue;
            }
            tokens.append({translatedLine.text, translatedLine.rect, line++, 0, 1.0});
        }
    }
    return tokens;
}

QVector<TranslatedBlock> offsetBlocks(const QVector<TranslatedBlock> &blocks, const QPointF &offset,
                                      const QRectF &bounds)
{
    QVector<TranslatedBlock> shifted;
    for (TranslatedBlock block : blocks) {
        block.sourceBox.translate(offset);
        block.plateRect.translate(offset);
        if (!block.plateRect.intersects(bounds)) {
            continue;
        }
        for (TranslatedLine &line : block.lines) {
            line.rect.translate(offset);
            line.baseline += offset.y();
        }
        shifted.append(block);
    }
    return shifted;
}

}  // namespace markshot::translation_layout
