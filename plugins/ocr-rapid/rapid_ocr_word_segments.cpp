#include "rapid_ocr_word_segments.h"

#include <QChar>

#include <algorithm>

namespace markshot::ocr_rapid {
namespace {

enum class SegmentClass {
    Separator,
    Grouped,
    Standalone,
};

/**
 * 判断码点是否属于中日韩或韩文文字范围。
 * @param codePoint Unicode 码点。
 * @return 属于 CJK、假名或韩文范围时返回 true。
 */
bool isCjkCodePoint(uint codePoint)
{
    return (codePoint >= 0x3400 && codePoint <= 0x4DBF)
        || (codePoint >= 0x4E00 && codePoint <= 0x9FFF)
        || (codePoint >= 0xF900 && codePoint <= 0xFAFF)
        || (codePoint >= 0x20000 && codePoint <= 0x2A6DF)
        || (codePoint >= 0x2A700 && codePoint <= 0x2B73F)
        || (codePoint >= 0x2B740 && codePoint <= 0x2B81F)
        || (codePoint >= 0x2B820 && codePoint <= 0x2CEAF)
        || (codePoint >= 0x3040 && codePoint <= 0x30FF)
        || (codePoint >= 0xAC00 && codePoint <= 0xD7AF);
}

/**
 * 判断文本是否全部由空白字符组成。
 * @param text 输入文本。
 * @return 空文本或全空白时返回 true。
 */
bool isSeparatorText(const QString &text)
{
    if (text.isEmpty()) {
        return true;
    }
    for (const QChar &character : text) {
        if (!character.isSpace()) {
            return false;
        }
    }
    return true;
}

/**
 * 判断文本是否包含 CJK 类字符。
 * @param text 输入文本。
 * @return 包含 CJK 类字符时返回 true。
 */
bool containsCjkText(const QString &text)
{
    const QVector<uint> codePoints = text.toUcs4();
    return std::any_of(codePoints.cbegin(), codePoints.cend(), isCjkCodePoint);
}

/**
 * 判断字符类别是否属于标点或符号。
 * @param category Qt Unicode 字符类别。
 * @return 标点或符号类别返回 true。
 */
bool isPunctuationOrSymbol(QChar::Category category)
{
    switch (category) {
    case QChar::Punctuation_Connector:
    case QChar::Punctuation_Dash:
    case QChar::Punctuation_Open:
    case QChar::Punctuation_Close:
    case QChar::Punctuation_InitialQuote:
    case QChar::Punctuation_FinalQuote:
    case QChar::Punctuation_Other:
    case QChar::Symbol_Math:
    case QChar::Symbol_Currency:
    case QChar::Symbol_Modifier:
    case QChar::Symbol_Other:
        return true;
    default:
        return false;
    }
}

/**
 * 判断字符类别是否属于零宽组合字符。
 * @param category Unicode 字符类别。
 * @return 属于组合标记时返回 true。
 */
bool isZeroWidthCategory(QChar::Category category)
{
    switch (category) {
    case QChar::Mark_NonSpacing:
    case QChar::Mark_SpacingCombining:
    case QChar::Mark_Enclosing:
    case QChar::Other_Control:
    case QChar::Other_Format:
    case QChar::Other_Surrogate:
        return true;
    default:
        return false;
    }
}

/**
 * 判断字符是否属于 East Asian 宽字符或全角字符范围。
 * @param codePoint Unicode BMP 码点。
 * @return 字符通常占用两个显示列时返回 true。
 */
bool isWideOrFullWidth(uint codePoint)
{
    return (codePoint >= 0x1100 && codePoint <= 0x115F)
        || (codePoint >= 0x2329 && codePoint <= 0x232A)
        || (codePoint >= 0x2E80 && codePoint <= 0xA4CF && codePoint != 0x303F)
        || (codePoint >= 0xAC00 && codePoint <= 0xD7A3)
        || (codePoint >= 0xF900 && codePoint <= 0xFAFF)
        || (codePoint >= 0xFE10 && codePoint <= 0xFE19)
        || (codePoint >= 0xFE30 && codePoint <= 0xFE6F)
        || (codePoint >= 0xFF00 && codePoint <= 0xFF60)
        || (codePoint >= 0xFFE0 && codePoint <= 0xFFE6);
}

/**
 * 计算字符在单行文本中的显示列宽权重。
 * @param text CTC 解码字符文本。
 * @return 半角字符返回 1，全角字符返回 2，零宽字符返回 0。
 */
qreal characterDisplayWeight(const QString &text)
{
    qreal weight = 0.0;
    for (const QChar &character : text) {
        if (isZeroWidthCategory(character.category())) {
            continue;
        }
        weight += isWideOrFullWidth(character.unicode()) ? 2.0 : 1.0;
    }
    return weight;
}

/**
 * 判断识别行是否包含宽字符。
 * @param characters CTC 字符列表。
 * @return 包含宽字符时返回 true。
 */
bool containsWideDisplayCharacter(const QVector<RapidRecCharacter> &characters)
{
    return std::any_of(characters.cbegin(), characters.cend(), [](const RapidRecCharacter &character) {
        for (const QChar &item : character.text) {
            if (isWideOrFullWidth(item.unicode())) {
                return true;
            }
        }
        return false;
    });
}

/**
 * 判断文本是否包含标点或符号。
 * @param text 输入文本。
 * @return 包含标点或符号时返回 true。
 */
bool containsPunctuationOrSymbol(const QString &text)
{
    return std::any_of(text.cbegin(), text.cend(), [](const QChar &character) {
        return isPunctuationOrSymbol(character.category());
    });
}

/**
 * 读取单字符所属的切分类别。
 * @param character CTC 解码字符。
 * @return 切分类别。
 */
SegmentClass classifyCharacter(const RapidRecCharacter &character)
{
    if (isSeparatorText(character.text)) {
        return SegmentClass::Separator;
    }
    if (containsCjkText(character.text) || containsPunctuationOrSymbol(character.text)) {
        return SegmentClass::Standalone;
    }
    return SegmentClass::Grouped;
}

/**
 * 拼接字符范围内的文本。
 * @param characters CTC 字符列表。
 * @param first 起始下标，包含该字符。
 * @param last 结束下标，包含该字符。
 * @return 去除首尾空白后的片段文本。
 */
QString segmentText(const QVector<RapidRecCharacter> &characters, int first, int last)
{
    QString text;
    for (int index = first; index <= last; ++index) {
        text += characters.at(index).text;
    }
    return text.trimmed();
}

/**
 * 计算字符范围内的平均置信度。
 * @param characters CTC 字符列表。
 * @param first 起始下标，包含该字符。
 * @param last 结束下标，包含该字符。
 * @return 平均置信度。
 */
qreal segmentConfidence(const QVector<RapidRecCharacter> &characters, int first, int last)
{
    double confidenceSum = 0.0;
    int count = 0;
    for (int index = first; index <= last; ++index) {
        confidenceSum += characters.at(index).confidence;
        ++count;
    }
    return count > 0 ? confidenceSum / count : 0.0;
}

/**
 * 按显示列宽把字符范围映射为原图像中的横向矩形。
 * @param characters CTC 字符列表。
 * @param first 起始下标，包含该字符。
 * @param last 结束下标，包含该字符。
 * @param lineBox 单行检测框。
 * @return 片段对应的像素矩形。
 */
QRectF displayWeightSegmentBox(const QVector<RapidRecCharacter> &characters,
                               int first,
                               int last,
                               const QRectF &lineBox)
{
    qreal totalWeight = 0.0;
    qreal prefixWeight = 0.0;
    qreal segmentWeight = 0.0;
    for (int index = 0; index < characters.size(); ++index) {
        const qreal weight = characterDisplayWeight(characters.at(index).text);
        totalWeight += weight;
        if (index < first) {
            prefixWeight += weight;
        } else if (index <= last) {
            segmentWeight += weight;
        }
    }
    if (totalWeight <= 0.0 || segmentWeight <= 0.0) {
        return {};
    }

    const qreal left = lineBox.left() + lineBox.width() * prefixWeight / totalWeight;
    const qreal width = lineBox.width() * segmentWeight / totalWeight;
    return QRectF(left, lineBox.top(), std::max<qreal>(0.5, width), lineBox.height());
}

/**
 * 把 CTC 时间步范围映射为原图像中的横向矩形。
 * @param characters CTC 字符列表。
 * @param first 起始下标，包含该字符。
 * @param last 结束下标，包含该字符。
 * @param stepCount 识别输出总时间步数。
 * @param lineBox 单行检测框。
 * @return 片段对应的像素矩形。
 */
QRectF segmentBox(const QVector<RapidRecCharacter> &characters,
                  int first,
                  int last,
                  int stepCount,
                  const QRectF &lineBox)
{
    if (containsWideDisplayCharacter(characters)) {
        const QRectF weightBox = displayWeightSegmentBox(characters, first, last, lineBox);
        if (weightBox.isValid()) {
            return weightBox;
        }
    }

    int startStep = stepCount;
    int endStep = -1;
    for (int index = first; index <= last; ++index) {
        const RapidRecCharacter &character = characters.at(index);
        const int characterStart = std::clamp(std::min(character.startStep, character.endStep),
                                             0,
                                             stepCount - 1);
        const int characterEnd = std::clamp(std::max(character.startStep, character.endStep),
                                           0,
                                           stepCount - 1);
        startStep = std::min(startStep, characterStart);
        endStep = std::max(endStep, characterEnd);
    }
    if (endStep < startStep) {
        return {};
    }

    const qreal leftRatio = static_cast<qreal>(startStep) / stepCount;
    const qreal rightRatio = static_cast<qreal>(endStep + 1) / stepCount;
    const qreal left = lineBox.left() + lineBox.width() * leftRatio;
    const qreal right = lineBox.left() + lineBox.width() * rightRatio;
    return QRectF(left, lineBox.top(), std::max<qreal>(0.5, right - left), lineBox.height());
}

/**
 * 追加一个词级别片段。
 * @param result 单行识别结果。
 * @param lineBox 单行检测框。
 * @param first 起始下标，包含该字符。
 * @param last 结束下标，包含该字符。
 * @param segments 输出片段列表。
 * @return 无返回值。
 */
void appendSegment(const RapidRecResult &result,
                   const QRectF &lineBox,
                   int first,
                   int last,
                   QVector<RapidOcrWordSegment> *segments)
{
    if (first < 0 || last < first || last >= result.characters.size()) {
        return;
    }

    const QString text = segmentText(result.characters, first, last);
    const QRectF box = segmentBox(result.characters, first, last, result.stepCount, lineBox);
    if (text.isEmpty() || !box.isValid()) {
        return;
    }

    RapidOcrWordSegment segment;
    segment.text = text;
    segment.box = box;
    segment.confidence = segmentConfidence(result.characters, first, last);
    segments->append(segment);
}

}  // namespace

QVector<RapidOcrWordSegment> buildRapidOcrWordSegments(const RapidRecResult &result,
                                                       const QRectF &lineBox)
{
    QVector<RapidOcrWordSegment> segments;
    if (!lineBox.isValid() || result.stepCount <= 0 || result.characters.isEmpty()) {
        return segments;
    }

    int groupStart = -1;
    for (int index = 0; index < result.characters.size(); ++index) {
        const SegmentClass segmentClass = classifyCharacter(result.characters.at(index));
        if (segmentClass == SegmentClass::Separator) {
            // 1. 空白结束当前拉丁词组
            appendSegment(result, lineBox, groupStart, index - 1, &segments);
            groupStart = -1;
            continue;
        }

        if (segmentClass == SegmentClass::Standalone) {
            // 2. 中文、标点和符号独立成词，避免无空格文本被合并成整行
            appendSegment(result, lineBox, groupStart, index - 1, &segments);
            groupStart = -1;
            appendSegment(result, lineBox, index, index, &segments);
            continue;
        }

        // 3. 拉丁字母和数字连续归并为一个词
        if (groupStart < 0) {
            groupStart = index;
        }
    }

    appendSegment(result, lineBox, groupStart, result.characters.size() - 1, &segments);
    return segments;
}

}  // namespace markshot::ocr_rapid
