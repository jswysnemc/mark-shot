#include "providers/translate/translate_segments.h"

#include "ocr_result.h"

#include <QHash>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

#include <algorithm>

namespace markshot::providers {
namespace {

/**
 * 合并两个矩形为包围盒。
 * @param left 已累计矩形，可为空矩形。
 * @param right 新矩形。
 * @return 合并后的矩形。
 */
QRectF unionRect(const QRectF &left, const QRectF &right)
{
    return left.isNull() ? right : left.united(right);
}

}  // namespace

QVector<TranslateSourceSegment> translateSegmentsFromInputJson(const QByteArray &inputJson,
                                                               QString *targetLanguage)
{
    if (targetLanguage) {
        targetLanguage->clear();
    }
    const QJsonDocument document = QJsonDocument::fromJson(inputJson);
    if (!document.isObject()) {
        return {};
    }
    const QJsonObject root = document.object();
    if (targetLanguage) {
        *targetLanguage = root.value(QStringLiteral("targetLanguage")).toString().trimmed();
    }

    // 1. 复用 OCR 解析器读取 tokens，天然兼容多种 box 形态
    const markshot::ocr::ParsedOutput parsed =
        markshot::ocr::parseOutput(QJsonDocument(root).toJson(QJsonDocument::Compact));

    // 2. 按行聚合 token，行内按现有空格启发式拼接，box 取包围盒
    QHash<int, QVector<const markshot::ocr::Token *>> lines;
    QVector<int> lineOrder;
    for (const markshot::ocr::Token &token : parsed.tokens) {
        if (!lines.contains(token.line)) {
            lineOrder.append(token.line);
        }
        lines[token.line].append(&token);
    }
    std::sort(lineOrder.begin(), lineOrder.end());

    QVector<TranslateSourceSegment> segments;
    for (const int line : lineOrder) {
        QVector<const markshot::ocr::Token *> tokens = lines.value(line);
        std::stable_sort(tokens.begin(), tokens.end(),
                         [](const markshot::ocr::Token *left, const markshot::ocr::Token *right) {
                             if (left->index != right->index) {
                                 return left->index < right->index;
                             }
                             return left->imageRect.left() < right->imageRect.left();
                         });

        QString text;
        QRectF box;
        const markshot::ocr::Token *previous = nullptr;
        for (const markshot::ocr::Token *token : tokens) {
            if (previous
                && markshot::ocr::shouldInsertSpace(previous->text,
                                                    token->text,
                                                    previous->imageRect,
                                                    token->imageRect)) {
                text += QLatin1Char(' ');
            }
            text += token->text;
            box = unionRect(box, token->imageRect);
            previous = token;
        }
        if (!text.trimmed().isEmpty() && !box.isNull()) {
            segments.append({line, text.trimmed(), box});
        }
    }
    return segments;
}

QByteArray translateTokensJson(const QVector<TranslateSourceSegment> &segments,
                               const QHash<int, QString> &translations,
                               const QString &backend)
{
    QJsonArray tokens;
    for (int index = 0; index < segments.size(); ++index) {
        const TranslateSourceSegment &segment = segments.at(index);
        QString translated = translations.value(segment.id).trimmed();
        if (translated.isEmpty()) {
            translated = segment.text;
        }
        QJsonObject token;
        token.insert(QStringLiteral("text"), translated);
        token.insert(QStringLiteral("box"),
                     QJsonArray{segment.box.x(), segment.box.y(), segment.box.width(), segment.box.height()});
        token.insert(QStringLiteral("line"), index);
        token.insert(QStringLiteral("index"), 0);
        token.insert(QStringLiteral("confidence"), 1.0);
        tokens.append(token);
    }

    QJsonObject root;
    root.insert(QStringLiteral("backend"), backend);
    root.insert(QStringLiteral("tokens"), tokens);
    root.insert(QStringLiteral("errors"), QJsonArray());
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

}  // namespace markshot::providers
