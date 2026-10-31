#include "translation_layout/paragraph_blocks.h"

#include <QHash>
#include <QRegularExpression>

#include <algorithm>
#include <cmath>
#include <limits>
#include <optional>

namespace markshot::translation_layout {

namespace {

// 两行高度之比超过该值时视为不同字号
constexpr qreal kMaxHeightRatio = 1.4;
// 行间空白超过行高的该倍数时视为分段
constexpr qreal kMaxLineGapRatio = 0.8;
// 行间重叠超过行高的该倍数时视为错位的两行
constexpr qreal kMaxLineOverlapRatio = 0.35;
// 左边或中线偏差不超过行高的该倍数时视为对齐
constexpr qreal kAlignToleranceRatio = 1.0;
// 段落首行最多缩进行高的该倍数
constexpr qreal kFirstLineIndentRatio = 3.0;
// 上一行宽度不足行高的该倍数时视为短行，后面另起一段
constexpr qreal kMinContinuedLineWidthRatio = 8.0;
// 上一行右边离段落右边超过该距离时视为提前换行：取行高倍数与段宽比例中较大的一个
constexpr qreal kMaxRaggedRightRatio = 4.0;
constexpr qreal kMaxRaggedRightWidthRatio = 0.3;
// 同一行内 token 空隙超过行高的该倍数时拆成两栏
constexpr qreal kColumnGapRatio = 2.2;

/// @brief 按 OCR 行号拼好的一行文字
struct TextLine {
    QString text;
    QRectF box;
};

/**
 * 判断字符是否属于中日韩文字或全角标点，这类字符换行拼接时不补空格。
 * @param ch 输入字符。
 * @return 属于中日韩文字、全角标点或代理对时返回 true。
 */
bool isWideChar(QChar ch)
{
    const ushort code = ch.unicode();
    return ch.isSurrogate() || markshot::ocr::isCjkCodePoint(code) || (code >= 0x3000 && code <= 0x303F)
        || (code >= 0xFF00 && code <= 0xFFEF);
}

/**
 * 判断一行是否以句末或冒号结束，结束后的下一行另起一段。
 * @param text 行文字。
 * @return 以句号、问号、叹号或冒号结束时返回 true。
 */
bool endsSentence(const QString &text)
{
    static const QString terminators = QStringLiteral(".!?:;。！？：；");
    return !text.isEmpty() && terminators.contains(text.back());
}

/**
 * 判断一行是否以列表符号或编号开头，列表项总是另起一段。
 * @param text 行文字。
 * @return 以项目符号或 1. 1) (1) 一类编号开头时返回 true。
 */
bool startsListItem(const QString &text)
{
    static const QRegularExpression pattern(
        QStringLiteral("^(?:[•·●○■□◆▪‣⁃*\\-–—]\\s"
                       "|\\d{1,3}[.)、](?!\\d)|[(（]\\d{1,3}[)）])"));
    return pattern.match(text).hasMatch();
}

/**
 * 按 OCR 行号把 token 拼成文本行，行内横向空隙过大时拆成两行。
 * @param tokens OCR token，图片像素坐标。
 * @return 文本行，按顶边自上而下排列。
 */
QVector<TextLine> textLines(const QVector<markshot::ocr::Token> &tokens)
{
    // 1. 同一行号的 token 放在一起，保持行号首次出现的顺序
    QHash<int, QVector<const markshot::ocr::Token *>> rows;
    QVector<int> rowOrder;
    for (const markshot::ocr::Token &token : tokens) {
        if (token.text.trimmed().isEmpty() || token.imageRect.isEmpty()) {
            continue;
        }
        if (!rows.contains(token.line)) {
            rowOrder.append(token.line);
        }
        rows[token.line].append(&token);
    }

    QVector<TextLine> lines;
    for (const int row : rowOrder) {
        QVector<const markshot::ocr::Token *> rowTokens = rows.value(row);
        std::stable_sort(rowTokens.begin(), rowTokens.end(),
                         [](const markshot::ocr::Token *left, const markshot::ocr::Token *right) {
                             if (left->index != right->index) {
                                 return left->index < right->index;
                             }
                             return left->imageRect.left() < right->imageRect.left();
                         });
        // 2. 行内按阅读顺序拼接，空隙明显大于词距的地方断成两行
        TextLine current;
        const markshot::ocr::Token *previous = nullptr;
        for (const markshot::ocr::Token *token : rowTokens) {
            if (previous) {
                const qreal height =
                    std::max<qreal>(1.0, std::min(previous->imageRect.height(), token->imageRect.height()));
                if (token->imageRect.left() - previous->imageRect.right() > height * kColumnGapRatio) {
                    lines.append(current);
                    current = {};
                    previous = nullptr;
                }
            }
            if (previous
                && markshot::ocr::shouldInsertSpace(previous->text, token->text, previous->imageRect,
                                                    token->imageRect)) {
                current.text += QLatin1Char(' ');
            }
            current.text += token->text.trimmed();
            current.box = current.box.isEmpty() ? token->imageRect : current.box.united(token->imageRect);
            previous = token;
        }
        if (!current.text.isEmpty()) {
            lines.append(current);
        }
    }

    // 3. 按顶边排序，两栏交错时也能自上而下逐行比较
    std::stable_sort(lines.begin(), lines.end(),
                     [](const TextLine &left, const TextLine &right) { return left.box.top() < right.box.top(); });
    return lines;
}

/**
 * 判断文本行能否接在段落末尾。
 * @param block 已有段落。
 * @param lastText 段落末行的文字。
 * @param line 候选文本行，顶边不高于段落末行。
 * @return 可以合并时返回与末行之间的空白，否则返回空。
 */
std::optional<qreal> continuationGap(const SourceBlock &block, const QString &lastText, const TextLine &line)
{
    const QRectF last = block.lineBoxes.constLast();
    const qreal lastHeight = std::max<qreal>(1.0, last.height());
    const qreal height = std::max<qreal>(1.0, line.box.height());
    const qreal unit = std::min(lastHeight, height);

    // 1. 字号相近，并且紧挨在末行下方
    if (std::max(lastHeight, height) / unit > kMaxHeightRatio) {
        return std::nullopt;
    }
    const qreal gap = line.box.top() - last.bottom();
    if (gap > unit * kMaxLineGapRatio || gap < -unit * kMaxLineOverlapRatio) {
        return std::nullopt;
    }

    // 2. 横向有重叠，左边对齐、首行缩进或中线对齐
    if (line.box.left() > block.box.right() || line.box.right() < block.box.left()) {
        return std::nullopt;
    }
    const qreal tolerance = unit * kAlignToleranceRatio;
    const bool leftAligned = std::abs(line.box.left() - block.box.left()) <= tolerance;
    const bool indentedFirstLine = block.lineBoxes.size() == 1 && line.box.left() < last.left()
        && last.left() - line.box.left() <= unit * kFirstLineIndentRatio;
    const bool centerAligned = std::abs(line.box.center().x() - last.center().x()) <= tolerance;
    if (!leftAligned && !indentedFirstLine && !centerAligned) {
        return std::nullopt;
    }

    // 3. 末行写满且没有在句末结束，新行也不是列表项，才算同一段落
    const qreal right = std::max(block.box.right(), line.box.right());
    const qreal width = right - std::min(block.box.left(), line.box.left());
    const bool lastLineFull = last.width() >= unit * kMinContinuedLineWidthRatio
        && right - last.right() <= std::max(unit * kMaxRaggedRightRatio, width * kMaxRaggedRightWidthRatio);
    if (!lastLineFull || endsSentence(lastText) || startsListItem(line.text)) {
        return std::nullopt;
    }
    return gap;
}

}  // namespace

QString joinParagraphLine(const QString &paragraph, const QString &nextLine)
{
    const QString head = paragraph.trimmed();
    const QString tail = nextLine.trimmed();
    if (head.isEmpty() || tail.isEmpty()) {
        return head.isEmpty() ? tail : head;
    }
    // 1. 行尾连字符断开的英文单词去掉连字符后拼回
    if (head.size() >= 2 && head.back() == QLatin1Char('-') && head.at(head.size() - 2).isLetter()
        && tail.front().isLower()) {
        return head.left(head.size() - 1) + tail;
    }
    // 2. 中日韩文字和全角标点直接相连，其余补一个空格
    if (isWideChar(head.back()) || isWideChar(tail.front())) {
        return head + tail;
    }
    return head + QLatin1Char(' ') + tail;
}

QVector<SourceBlock> groupParagraphs(const QVector<markshot::ocr::Token> &tokens)
{
    QVector<SourceBlock> blocks;
    QVector<QString> lastLineTexts;
    for (const TextLine &line : textLines(tokens)) {
        // 1. 在已有段落里找能接上的一段，两栏交错时取离得最近的那段
        int target = -1;
        qreal bestGap = std::numeric_limits<qreal>::max();
        for (int index = 0; index < blocks.size(); ++index) {
            const std::optional<qreal> gap = continuationGap(blocks.at(index), lastLineTexts.at(index), line);
            if (gap && std::abs(*gap) < bestGap) {
                bestGap = std::abs(*gap);
                target = index;
            }
        }

        // 2. 接不上时另起一段，接上时并入文字和包围盒
        if (target < 0) {
            SourceBlock block;
            block.text = line.text;
            block.box = line.box;
            block.lineBoxes = {line.box};
            blocks.append(block);
            lastLineTexts.append(line.text);
            continue;
        }
        SourceBlock &block = blocks[target];
        block.text = joinParagraphLine(block.text, line.text);
        block.box = block.box.united(line.box);
        block.lineBoxes.append(line.box);
        lastLineTexts[target] = line.text;
    }

    // 3. 行高取中位数，避免带下伸部的行把字号拉大；行距取首末行顶边的平均间隔
    for (SourceBlock &block : blocks) {
        QVector<qreal> heights;
        heights.reserve(block.lineBoxes.size());
        for (const QRectF &lineBox : block.lineBoxes) {
            heights.append(lineBox.height());
        }
        std::sort(heights.begin(), heights.end());
        block.lineHeight = heights.at(heights.size() / 2);
        const int lineCount = block.lineBoxes.size();
        block.linePitch = lineCount > 1
            ? (block.lineBoxes.constLast().top() - block.lineBoxes.constFirst().top()) / (lineCount - 1)
            : 0.0;
    }
    return blocks;
}

}  // namespace markshot::translation_layout
