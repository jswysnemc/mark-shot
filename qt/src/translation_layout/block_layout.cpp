#include "translation_layout/block_layout.h"

#include "translation_layout/block_colors.h"

#include <QFontMetricsF>
#include <QTextLayout>
#include <QTextLine>
#include <QTextOption>

#include <algorithm>
#include <cmath>

namespace markshot::translation_layout {

namespace {

// 译文字号范围，像素
constexpr int kMinFontPixelSize = 9;
constexpr int kMaxFontPixelSize = 96;
// 起始字号占原文行高的比例，OCR 行框通常比字面多出上下留白
constexpr qreal kFontToLineHeight = 0.78;
// 每次缩小字号的比例
constexpr qreal kShrinkFactor = 0.92;
// 底板向四周多盖住的宽度占行高的比例，盖住原文笔画的抗锯齿边
constexpr qreal kPlatePaddingRatio = 0.12;
// 译文行距占字体行高的范围
constexpr qreal kMinLineSpacing = 1.12;
constexpr qreal kMaxLineSpacing = 1.8;

/// @brief 选定字号后的一次排版
struct FittedText {
    QFont font;
    QVector<QString> lines;
    // 相邻两行顶边的距离
    qreal advance = 0.0;
    qreal ascent = 0.0;
    qreal lineHeight = 0.0;
};

/**
 * 按宽度给译文换行，西文优先在词间断开，过长的词和中日韩文字按字断开。
 * @param text 译文。
 * @param font 字体。
 * @param width 可用宽度。
 * @return 各行文字，已去掉行首行尾空白。
 */
QVector<QString> wrapText(const QString &text, const QFont &font, qreal width)
{
    QString normalized = text;
    normalized.replace(QLatin1Char('\n'), QChar::LineSeparator);
    QTextLayout layout(normalized, font);
    QTextOption option;
    option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
    layout.setTextOption(option);
    QVector<QString> lines;
    layout.beginLayout();
    while (true) {
        QTextLine line = layout.createLine();
        if (!line.isValid()) {
            break;
        }
        line.setLineWidth(width);
        const QString lineText = normalized.mid(line.textStart(), line.textLength()).trimmed();
        if (!lineText.isEmpty()) {
            lines.append(lineText);
        }
    }
    layout.endLayout();
    return lines;
}

/**
 * 计算排好的译文总高度。
 * @param fitted 一次排版。
 * @return 首行顶边到末行底边的高度。
 */
qreal textHeight(const FittedText &fitted)
{
    const int count = std::max(1, static_cast<int>(fitted.lines.size()));
    return (count - 1) * fitted.advance + fitted.lineHeight;
}

/**
 * 从原文行高对应的字号起步排版，放不下时逐步缩小，到最小字号为止。
 * @param text 译文。
 * @param block 原文段落。
 * @param baseFont 译文字体。
 * @param availableHeight 可用高度。
 * @return 放得下的最大字号的排版；最小字号仍放不下时返回最小字号的排版。
 */
FittedText fitText(const QString &text, const SourceBlock &block, const QFont &baseFont, qreal availableHeight)
{
    const int startSize =
        std::clamp(qRound(block.lineHeight * kFontToLineHeight), kMinFontPixelSize, kMaxFontPixelSize);
    const qreal width = std::max<qreal>(1.0, block.box.width());
    qreal size = startSize;
    while (true) {
        const int pixelSize = std::max(kMinFontPixelSize, qRound(size));
        FittedText fitted;
        fitted.font = baseFont;
        fitted.font.setPixelSize(pixelSize);
        const QFontMetricsF metrics(fitted.font);
        fitted.lines = wrapText(text, fitted.font, width);
        fitted.ascent = metrics.ascent();
        fitted.lineHeight = metrics.height();
        // 1. 行距沿用原文行距并随字号等比缩小，限制在字体行高的合理倍数内
        fitted.advance = std::clamp(block.linePitch * pixelSize / startSize, metrics.height() * kMinLineSpacing,
                                    metrics.height() * kMaxLineSpacing);
        if (textHeight(fitted) <= availableHeight || pixelSize <= kMinFontPixelSize) {
            return fitted;
        }
        size = std::min<qreal>(size * kShrinkFactor, pixelSize - 1);
    }
}

/**
 * 计算段落底板向下延伸的极限：下方横向重叠的段落顶边与图片底边中较高的一个。
 * @param blocks 原文段落。
 * @param index 当前段落序号。
 * @param bounds 图片矩形。
 * @return 底板底边的最大纵坐标。
 */
qreal belowLimit(const QVector<SourceBlock> &blocks, int index, const QRectF &bounds)
{
    const QRectF box = blocks.at(index).box;
    qreal limit = bounds.bottom();
    for (int other = 0; other < blocks.size(); ++other) {
        const QRectF candidate = blocks.at(other).box;
        if (other == index || candidate.top() < box.bottom() || candidate.right() <= box.left()
            || candidate.left() >= box.right()) {
            continue;
        }
        limit = std::min(limit, candidate.top() - 1.0);
    }
    return limit;
}

}  // namespace

QVector<TranslatedBlock> layoutTranslatedBlocks(const QImage &image, const QVector<SourceBlock> &blocks,
                                                const QVector<QString> &texts, const QFont &baseFont)
{
    QVector<TranslatedBlock> result;
    const QRectF bounds(image.rect());
    for (int index = 0; index < blocks.size() && index < texts.size(); ++index) {
        const SourceBlock &source = blocks.at(index);
        const QString text = texts.at(index).trimmed();
        if (text.isEmpty() || source.box.isEmpty()) {
            continue;
        }

        // 1. 底板比原文框略大，盖住原文笔画的边缘；先在底板高度内找放得下的字号
        const qreal padding = std::clamp(source.lineHeight * kPlatePaddingRatio, 1.0, 6.0);
        QRectF plate = source.box.adjusted(-padding, -padding, padding, padding).intersected(bounds);
        FittedText fitted = fitText(text, source, baseFont, plate.height());
        qreal needed = textHeight(fitted);

        // 2. 最小字号仍放不下时底板向下延伸，碰到下一段或图片底边后截断多出的行
        if (needed > plate.height()) {
            const qreal limit = std::max(plate.bottom(), belowLimit(blocks, index, bounds));
            plate.setBottom(std::min(limit, plate.top() + needed));
            const int capacity =
                std::max(1, static_cast<int>(std::floor((plate.height() - fitted.lineHeight) / fitted.advance)) + 1);
            if (fitted.lines.size() > capacity) {
                fitted.lines.resize(capacity);
                const QFontMetricsF metrics(fitted.font);
                fitted.lines.last() = metrics.elidedText(fitted.lines.constLast() + QChar(0x2026), Qt::ElideRight,
                                                         source.box.width());
                needed = textHeight(fitted);
            }
        }

        // 3. 单行译文与原文垂直居中，多行译文从原文首行顶边开始
        qreal top = fitted.lines.size() <= 1 ? source.box.center().y() - needed / 2.0 : source.box.top();
        top = std::clamp(top, plate.top(), std::max(plate.top(), plate.bottom() - needed));

        // 4. 记录配色和每行位置，行宽取文字实际宽度供拖选命中
        TranslatedBlock block;
        block.sourceBox = source.box;
        block.plateRect = plate;
        const BlockColors colors = sampleBlockColors(image, source.box, source.lineHeight);
        block.background = colors.background;
        block.foreground = colors.foreground;
        block.font = fitted.font;
        block.text = text;
        const QFontMetricsF metrics(fitted.font);
        for (int line = 0; line < fitted.lines.size(); ++line) {
            const qreal lineTop = top + line * fitted.advance;
            const qreal width = std::clamp(metrics.horizontalAdvance(fitted.lines.at(line)), 1.0,
                                           std::max(1.0, plate.right() - source.box.left()));
            block.lines.append({fitted.lines.at(line), QRectF(source.box.left(), lineTop, width, fitted.lineHeight),
                                lineTop + fitted.ascent});
        }
        result.append(block);
    }
    return result;
}

}  // namespace markshot::translation_layout
