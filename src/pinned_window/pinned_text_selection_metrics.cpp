#include "pinned_window/pinned_text_selection_metrics.h"

#include <algorithm>

namespace markshot::shot {
namespace {

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

}  // namespace

qreal pinnedTextSelectionCharacterWeight(QChar character)
{
    if (isZeroWidthCategory(character.category())) {
        return 0.0;
    }
    if (isWideOrFullWidth(character.unicode())) {
        return 2.0;
    }
    return 1.0;
}

qreal pinnedTextSelectionTextWeight(const QString &text)
{
    qreal weight = 0.0;
    for (const QChar &character : text) {
        weight += pinnedTextSelectionCharacterWeight(character);
    }
    return weight;
}

QRectF pinnedTextSelectionHighlightRect(QRectF tokenRect, const QString &text)
{
    if (!tokenRect.isValid() || text.isEmpty()) {
        return tokenRect;
    }

    qreal displayWeight = 0.0;
    qreal visibleCharacters = 0.0;
    for (const QChar &character : text) {
        const qreal characterWeight = pinnedTextSelectionCharacterWeight(character);
        displayWeight += characterWeight;
        if (characterWeight > 0.0) {
            visibleCharacters += 1.0;
        }
    }
    if (displayWeight <= visibleCharacters || visibleCharacters <= 0.0) {
        return tokenRect;
    }

    // 1. 钉图选择框通常来自 OCR 矩形；当中文被识别成半角宽度时，用行高估算单列宽补齐
    const qreal estimatedColumnWidth = tokenRect.height() * 0.5;
    const qreal minimumWidth = estimatedColumnWidth * displayWeight;
    if (tokenRect.width() >= minimumWidth) {
        return tokenRect;
    }
    tokenRect.setWidth(std::max(tokenRect.width(), minimumWidth));
    return tokenRect;
}

}  // namespace markshot::shot
