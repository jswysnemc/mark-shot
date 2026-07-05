#include "openai_translation_parser.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QRegularExpression>

namespace markshot::translate_openai {

bool parseTranslationContent(const QString &content, QHash<int, QString> *translations, QString *error)
{
    if (!translations) {
        if (error) {
            *error = QStringLiteral("translation output target is missing");
        }
        return false;
    }
    translations->clear();

    QString trimmed = content.trimmed();
    QJsonDocument document = QJsonDocument::fromJson(trimmed.toUtf8());
    if (!document.isObject()) {
        // 1. 兼容模型输出被 Markdown 包裹的场景
        const QRegularExpression pattern(QStringLiteral("\\{.*\\}"),
                                         QRegularExpression::DotMatchesEverythingOption);
        const QRegularExpressionMatch match = pattern.match(trimmed);
        if (match.hasMatch()) {
            document = QJsonDocument::fromJson(match.captured(0).toUtf8());
        }
    }
    if (!document.isObject()) {
        if (error) {
            *error = QStringLiteral("llm response is not valid JSON");
        }
        return false;
    }

    const QJsonValue value = document.object().value(QStringLiteral("translations"));
    if (!value.isArray()) {
        if (error) {
            *error = QStringLiteral("llm response missing translations array");
        }
        return false;
    }

    for (const QJsonValue &item : value.toArray()) {
        if (!item.isObject()) {
            continue;
        }
        const QJsonObject object = item.toObject();
        if (!object.contains(QStringLiteral("id"))) {
            continue;
        }
        translations->insert(object.value(QStringLiteral("id")).toInt(),
                             object.value(QStringLiteral("text")).toString().trimmed());
    }
    return true;
}

}  // namespace markshot::translate_openai
