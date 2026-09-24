#include "anthropic_translate_request.h"

#include "translate_http_client.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QRegularExpression>

namespace markshot::translate_anthropic {
namespace {

bool parseTranslationJson(const QString &rawText, QHash<int, QString> *translations, QString *error)
{
    QString trimmed = rawText.trimmed();
    QJsonDocument document = QJsonDocument::fromJson(trimmed.toUtf8());
    if (!document.isObject()) {
        const QRegularExpression pattern(QStringLiteral("\\{.*\\}"),
                                         QRegularExpression::DotMatchesEverythingOption);
        const QRegularExpressionMatch match = pattern.match(trimmed);
        if (match.hasMatch()) {
            document = QJsonDocument::fromJson(match.captured(0).toUtf8());
        }
    }
    if (!document.isObject()) {
        if (error) {
            *error = QStringLiteral("anthropic text response is not valid JSON: ") + trimmed.left(200);
        }
        return false;
    }

    const QJsonValue value = document.object().value(QStringLiteral("translations"));
    if (!value.isArray()) {
        if (error) {
            *error = QStringLiteral("anthropic response missing translations array");
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

}  // namespace

QUrl buildAnthropicUrl(QString endpoint)
{
    while (endpoint.endsWith(QLatin1Char('/'))) {
        endpoint.chop(1);
    }
    if (endpoint.endsWith(QStringLiteral("/messages"))) {
        return QUrl(endpoint);
    }
    // 设置页占位符是不带版本号的 https://api.anthropic.com，此时补齐 /v1
    if (!endpoint.endsWith(QStringLiteral("/v1")) && !endpoint.contains(QStringLiteral("/v1/"))) {
        endpoint += QStringLiteral("/v1");
    }
    return QUrl(endpoint + QStringLiteral("/messages"));
}

QByteArray buildAnthropicPayload(const AnthropicTranslateConfig &config,
                                const QVector<markshot::plugin::TranslateSegment> &segments,
                                const QString &targetLanguage)
{
    QJsonArray segmentArray;
    for (const auto &segment : segments) {
        segmentArray.append(QJsonObject{{QStringLiteral("id"), segment.id},
                                        {QStringLiteral("text"), segment.text}});
    }

    const QJsonObject userPrompt{
        {QStringLiteral("target_language"), targetLanguage.trimmed().isEmpty()
             ? QStringLiteral("Simplified Chinese")
             : targetLanguage.trimmed()},
        {QStringLiteral("instructions"),
         QJsonArray{QStringLiteral("Translate each segment into target_language."),
                    QStringLiteral("Return JSON exactly as {\"translations\":[{\"id\":0,\"text\":\"...\"}]} "
                                   "with no markdown."),
                    QStringLiteral("Do not add explanations.")}},
        {QStringLiteral("segments"), segmentArray}};

    QJsonObject payload;
    payload.insert(QStringLiteral("model"), config.model);
    payload.insert(QStringLiteral("max_tokens"), config.maxTokens);
    if (config.temperature.has_value()) {
        payload.insert(QStringLiteral("temperature"), *config.temperature);
    }
    if (!config.systemPrompt.trimmed().isEmpty()) {
        payload.insert(QStringLiteral("system"), config.systemPrompt);
    }

    payload.insert(QStringLiteral("messages"),
                   QJsonArray{QJsonObject{
                       {QStringLiteral("role"), QStringLiteral("user")},
                       {QStringLiteral("content"),
                        QString::fromUtf8(QJsonDocument(userPrompt).toJson(QJsonDocument::Compact))}}});

    return QJsonDocument(payload).toJson(QJsonDocument::Compact);
}

bool parseAnthropicResponse(const QByteArray &responseBody,
                           QHash<int, QString> *translations,
                           QString *error)
{
    if (!translations) {
        if (error) {
            *error = QStringLiteral("translations output target is null");
        }
        return false;
    }
    translations->clear();

    const QJsonDocument document = QJsonDocument::fromJson(responseBody);
    if (!document.isObject()) {
        if (error) {
            *error = QStringLiteral("invalid JSON response from Anthropic");
        }
        return false;
    }

    const QJsonObject root = document.object();
    if (root.contains(QStringLiteral("error"))) {
        const QJsonObject errorObj = root.value(QStringLiteral("error")).toObject();
        if (error) {
            *error = QStringLiteral("anthropic error (%1): %2")
                         .arg(errorObj.value(QStringLiteral("type")).toString(),
                              errorObj.value(QStringLiteral("message")).toString());
        }
        return false;
    }

    const QJsonArray contentArray = root.value(QStringLiteral("content")).toArray();
    if (contentArray.isEmpty()) {
        if (error) {
            *error = QStringLiteral("anthropic response missing content");
        }
        return false;
    }

    QString text;
    for (const QJsonValue &val : contentArray) {
        if (val.isObject() && val.toObject().value(QStringLiteral("type")).toString() == QStringLiteral("text")) {
            text += val.toObject().value(QStringLiteral("text")).toString();
        }
    }

    if (text.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("anthropic response text content is empty");
        }
        return false;
    }

    return parseTranslationJson(text, translations, error);
}

bool sendAnthropicRequest(const AnthropicTranslateConfig &config,
                         const QByteArray &payload,
                         QByteArray *responseBody,
                         QString *error)
{
    markshot::translate_common::TranslateHttpRequest request;
    request.url = buildAnthropicUrl(config.endpoint);
    request.contentType = QByteArrayLiteral("application/json");
    request.headers.append({QByteArrayLiteral("x-api-key"), config.apiKey.toUtf8()});
    request.headers.append({QByteArrayLiteral("anthropic-version"), QByteArrayLiteral("2023-06-01")});
    request.body = payload;
    request.timeoutMs = config.timeoutMs;

    markshot::translate_common::TranslateHttpResponse response;
    if (!markshot::translate_common::sendTranslateHttpRequest(request, &response, error)) {
        return false;
    }

    if (response.httpStatus < 200 || response.httpStatus >= 300) {
        const QJsonDocument doc = QJsonDocument::fromJson(response.body);
        if (doc.isObject() && doc.object().contains(QStringLiteral("error"))) {
            const QString msg = doc.object().value(QStringLiteral("error")).toObject().value(QStringLiteral("message")).toString();
            if (error) {
                *error = QStringLiteral("anthropic http %1: %2").arg(response.httpStatus).arg(msg);
            }
            return false;
        }

        if (error) {
            *error = QStringLiteral("anthropic http %1: %2")
                         .arg(response.httpStatus)
                         .arg(QString::fromUtf8(response.body.left(500)));
        }
        return false;
    }

    if (responseBody) {
        *responseBody = response.body;
    }
    return true;
}

}  // namespace markshot::translate_anthropic
