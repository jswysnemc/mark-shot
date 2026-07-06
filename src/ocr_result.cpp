#include "ocr_result.h"

#include <QJsonDocument>
#include <QJsonParseError>
#include <QPointF>
#include <QSizeF>

#include <algorithm>

namespace markshot::ocr {

QJsonArray rectToJson(QRectF rect)
{
    QJsonArray array;
    array.append(rect.x());
    array.append(rect.y());
    array.append(rect.width());
    array.append(rect.height());
    return array;
}

std::optional<QRectF> rectFromJsonValue(const QJsonValue &value)
{
    if (!value.isArray()) {
        return std::nullopt;
    }

    const QJsonArray array = value.toArray();
    if (array.size() == 4
        && array.at(0).isDouble()
        && array.at(1).isDouble()
        && array.at(2).isDouble()
        && array.at(3).isDouble()) {
        return QRectF(array.at(0).toDouble(),
                      array.at(1).toDouble(),
                      array.at(2).toDouble(),
                      array.at(3).toDouble());
    }

    if (array.size() < 2 || !array.at(0).isArray()) {
        return std::nullopt;
    }

    qreal minX = 0.0;
    qreal minY = 0.0;
    qreal maxX = 0.0;
    qreal maxY = 0.0;
    bool initialized = false;
    for (const QJsonValue &pointValue : array) {
        if (!pointValue.isArray()) {
            continue;
        }
        const QJsonArray point = pointValue.toArray();
        if (point.size() < 2 || !point.at(0).isDouble() || !point.at(1).isDouble()) {
            continue;
        }
        const QPointF p(point.at(0).toDouble(), point.at(1).toDouble());
        if (!initialized) {
            minX = maxX = p.x();
            minY = maxY = p.y();
        } else {
            minX = std::min(minX, p.x());
            minY = std::min(minY, p.y());
            maxX = std::max(maxX, p.x());
            maxY = std::max(maxY, p.y());
        }
        initialized = true;
    }

    if (!initialized) {
        return std::nullopt;
    }
    return QRectF(QPointF(minX, minY), QPointF(maxX, maxY)).normalized();
}

std::optional<QRectF> tokenRect(const QJsonObject &object)
{
    for (const QString &key : {QStringLiteral("box"), QStringLiteral("bbox"), QStringLiteral("points")}) {
        if (object.contains(key)) {
            return rectFromJsonValue(object.value(key));
        }
    }

    if (object.contains(QStringLiteral("x")) && object.contains(QStringLiteral("y"))) {
        return QRectF(object.value(QStringLiteral("x")).toDouble(),
                      object.value(QStringLiteral("y")).toDouble(),
                      object.value(QStringLiteral("width")).toDouble(),
                      object.value(QStringLiteral("height")).toDouble());
    }

    if (object.contains(QStringLiteral("left")) && object.contains(QStringLiteral("top"))) {
        return QRectF(object.value(QStringLiteral("left")).toDouble(),
                      object.value(QStringLiteral("top")).toDouble(),
                      object.value(QStringLiteral("width")).toDouble(),
                      object.value(QStringLiteral("height")).toDouble());
    }

    return std::nullopt;
}

ParsedOutput parseOutput(const QByteArray &output, QSize imageSize)
{
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
    if (parseError.error != QJsonParseError::NoError) {
        return {};
    }

    QJsonArray tokenArray;
    if (document.isArray()) {
        tokenArray = document.array();
    } else if (document.isObject()) {
        tokenArray = document.object().value(QStringLiteral("tokens")).toArray();
    }

    ParsedOutput parsed;
    parsed.validJson = true;
    parsed.tokens.reserve(tokenArray.size());

    const bool clipToImage = imageSize.isValid() && !imageSize.isEmpty();
    const QRectF imageBounds(QPointF(0.0, 0.0), QSizeF(imageSize));
    int fallbackIndex = 0;
    for (const QJsonValue &value : tokenArray) {
        if (!value.isObject()) {
            continue;
        }

        const QJsonObject object = value.toObject();
        Token token;
        token.text = object.value(QStringLiteral("text")).toString().trimmed();
        if (token.text.isEmpty()) {
            continue;
        }

        const std::optional<QRectF> rect = tokenRect(object);
        if (!rect) {
            continue;
        }

        token.imageRect = rect->normalized();
        if (clipToImage) {
            token.imageRect = token.imageRect.intersected(imageBounds);
            if (token.imageRect.isEmpty()) {
                continue;
            }
        }

        token.line = object.value(QStringLiteral("line")).toInt(0);
        token.index = object.value(QStringLiteral("index")).toInt(fallbackIndex++);
        token.confidence = object.value(QStringLiteral("confidence")).toDouble(0.0);
        parsed.tokens.append(token);
    }

    std::stable_sort(parsed.tokens.begin(), parsed.tokens.end(), [](const Token &left, const Token &right) {
        if (left.line != right.line) {
            return left.line < right.line;
        }
        if (left.index != right.index) {
            return left.index < right.index;
        }
        if (!qFuzzyCompare(left.imageRect.top(), right.imageRect.top())) {
            return left.imageRect.top() < right.imageRect.top();
        }
        return left.imageRect.left() < right.imageRect.left();
    });

    return parsed;
}

/// @brief Parses OCR JSON output to retrieve a vector of Token objects.
/// @param output The raw JSON output.
/// @param imageSize The size of the source image.
/// @return A vector of parsed Token objects.
QVector<Token> tokensFromJsonOutput(const QByteArray &output, QSize imageSize)
{
    return parseOutput(output, imageSize).tokens;
}

bool isNoLeadingSpacePunctuation(QChar ch)
{
    switch (ch.unicode()) {
    case '.':
    case ',':
    case ';':
    case ':':
    case '!':
    case '?':
    case ')':
    case ']':
    case '}':
    case 0x3001:
    case 0x3002:
    case 0x300B:
    case 0x3011:
    case 0xFF01:
    case 0xFF09:
    case 0xFF0C:
    case 0xFF1A:
    case 0xFF1B:
    case 0xFF1F:
        return true;
    default:
        return false;
    }
}

/**
 * 判断码点是否属于中日韩、假名或韩文范围。
 * @param codePoint Unicode 码点。
 * @return 属于 CJK 类文字时返回 true。
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
 * 判断文本是否包含 CJK 类文字。
 * @param text 输入文本。
 * @return 包含 CJK 类文字时返回 true。
 */
bool containsCjkText(const QString &text)
{
    const QVector<uint> codePoints = text.toUcs4();
    return std::any_of(codePoints.cbegin(), codePoints.cend(), isCjkCodePoint);
}

/**
 * 判断字符是否为不应在后方补空格的中文标点。
 * @param ch 输入字符。
 * @return 中文标点后方应直接连接后续文字时返回 true。
 */
bool isNoTrailingSpaceCjkPunctuation(QChar ch)
{
    switch (ch.unicode()) {
    case 0x3001:
    case 0x3002:
    case 0x300B:
    case 0x3011:
    case 0xFF01:
    case 0xFF09:
    case 0xFF0C:
    case 0xFF1A:
    case 0xFF1B:
    case 0xFF1F:
        return true;
    default:
        return false;
    }
}

/**
 * 判断两个 CJK 相关 token 是否应保持直接相邻。
 * @param previousText 前一个 token 文本。
 * @param currentText 当前 token 文本。
 * @return 不应补空格时返回 true。
 */
bool shouldKeepCjkTokensAdjacent(const QString &previousText, const QString &currentText)
{
    if (containsCjkText(previousText) && containsCjkText(currentText)) {
        return true;
    }
    if (!previousText.isEmpty()
        && isNoTrailingSpaceCjkPunctuation(previousText.back())
        && containsCjkText(currentText)) {
        return true;
    }
    return false;
}

bool shouldInsertSpace(const QString &previousText,
                       const QString &currentText,
                       QRectF previousRect,
                       QRectF currentRect)
{
    if (previousText.isEmpty() || currentText.isEmpty()) {
        return false;
    }
    if (isNoLeadingSpacePunctuation(currentText.front())) {
        return false;
    }
    if (shouldKeepCjkTokensAdjacent(previousText, currentText)) {
        return false;
    }

    const qreal gap = currentRect.left() - previousRect.right();
    const qreal threshold = std::max<qreal>(3.0, std::min(previousRect.height(), currentRect.height()) * 0.28);
    return gap > threshold;
}

/// @brief Joins a range of tokens into a formatted text string, respecting lines and spacing.
/// @param tokens The vector of Token objects.
/// @param first The starting index in the token vector.
/// @param last The ending index (inclusive) in the token vector.
/// @return The joined text string for the specified range.
QString tokenRangeText(const QVector<Token> &tokens, int first, int last)
{
    if (tokens.isEmpty() || first < 0 || last < first || last >= tokens.size()) {
        return {};
    }

    QString text;
    int currentLine = -1;
    QRectF previousRect;
    QString previousText;
    for (int i = first; i <= last; ++i) {
        const Token &token = tokens.at(i);
        if (currentLine != token.line) {
            if (!text.isEmpty()) {
                text += QLatin1Char('\n');
            }
            currentLine = token.line;
        } else if (shouldInsertSpace(previousText, token.text, previousRect, token.imageRect)) {
            text += QLatin1Char(' ');
        }
        text += token.text;
        previousText = token.text;
        previousRect = token.imageRect;
    }
    return text;
}

/// @brief Converts a vector of tokens into a single text string.
/// @param tokens The vector of Token objects.
/// @return The joined text string.
QString tokensText(const QVector<Token> &tokens)
{
    return tokenRangeText(tokens, 0, tokens.size() - 1);
}

}  // namespace markshot::ocr
