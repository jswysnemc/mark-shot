#include "gemini_translate_request.h"

#include "translate_http_client.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QRegularExpression>

namespace markshot::translate_gemini {
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
            *error = QStringLiteral("gemini text response is not valid JSON: ") + trimmed.left(200);
        }
        return false;
    }

    const QJsonValue value = document.object().value(QStringLiteral("translations"));
    if (!value.isArray()) {
        if (error) {
            *error = QStringLiteral("gemini response missing translations array");
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

QUrl buildGeminiUrl(QString endpoint, const QString &model)
{
    while (endpoint.endsWith(QLatin1Char('/'))) {
        endpoint.chop(1);
    }
    if (!endpoint.contains(QStringLiteral("/v1"))) {
        endpoint += QStringLiteral("/v1beta");
    }
    QString modelName = model.trimmed();
    if (modelName.startsWith(QStringLiteral("models/"))) {
        modelName = modelName.mid(7);
    }
    return QUrl(QStringLiteral("%1/models/%2:generateContent").arg(endpoint, modelName));
}

QByteArray buildGeminiPayload(const GeminiTranslateConfig &config,
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
    if (!config.systemPrompt.trimmed().isEmpty()) {
        payload.insert(QStringLiteral("systemInstruction"),
                       QJsonObject{{QStringLiteral("parts"),
                                    QJsonArray{QJsonObject{{QStringLiteral("text"), config.systemPrompt}}}}});
    }

    payload.insert(QStringLiteral("contents"),
                   QJsonArray{QJsonObject{
                       {QStringLiteral("role"), QStringLiteral("user")},
                       {QStringLiteral("parts"),
                        QJsonArray{QJsonObject{{QStringLiteral("text"),
                                                QString::fromUtf8(QJsonDocument(userPrompt).toJson(QJsonDocument::Compact))}}}}}});

    QJsonObject generationConfig;
    if (config.temperature.has_value()) {
        generationConfig.insert(QStringLiteral("temperature"), *config.temperature);
    }
    generationConfig.insert(QStringLiteral("responseMimeType"), QStringLiteral("application/json"));

    // 仅在显式配置时下发思考档位：minimal 等取值并非所有模型都支持，错配会返回 400
    if (!config.thinkingLevel.trimmed().isEmpty()) {
        generationConfig.insert(QStringLiteral("thinkingConfig"),
                                QJsonObject{{QStringLiteral("thinkingLevel"),
                                             config.thinkingLevel.trimmed().toUpper()}});
    }

    payload.insert(QStringLiteral("generationConfig"), generationConfig);

    return QJsonDocument(payload).toJson(QJsonDocument::Compact);
}

bool parseGeminiResponse(const QByteArray &responseBody,
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
            *error = QStringLiteral("invalid JSON response from Gemini");
        }
        return false;
    }

    const QJsonObject root = document.object();
    if (root.contains(QStringLiteral("error"))) {
        const QJsonObject errorObj = root.value(QStringLiteral("error")).toObject();
        if (error) {
            *error = QStringLiteral("gemini error: %1").arg(errorObj.value(QStringLiteral("message")).toString());
        }
        return false;
    }

    const QJsonArray candidates = root.value(QStringLiteral("candidates")).toArray();
    if (candidates.isEmpty()) {
        if (error) {
            *error = QStringLiteral("gemini response missing candidates");
        }
        return false;
    }

    const QJsonObject candidate = candidates.at(0).toObject();
    const QJsonObject content = candidate.value(QStringLiteral("content")).toObject();
    const QJsonArray parts = content.value(QStringLiteral("parts")).toArray();
    if (parts.isEmpty()) {
        if (error) {
            *error = QStringLiteral("gemini response candidate missing content parts");
        }
        return false;
    }

    const QString text = parts.at(0).toObject().value(QStringLiteral("text")).toString();
    if (text.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("gemini candidate part text is empty");
        }
        return false;
    }

    return parseTranslationJson(text, translations, error);
}

bool sendGeminiRequest(const GeminiTranslateConfig &config,
                       const QByteArray &payload,
                       QByteArray *responseBody,
                       QString *error)
{
    markshot::translate_common::TranslateHttpRequest request;
    request.url = buildGeminiUrl(config.endpoint, config.model);
    request.contentType = QByteArrayLiteral("application/json");
    request.headers.append({QByteArrayLiteral("x-goog-api-key"), config.apiKey.toUtf8()});
    request.body = payload;
    request.timeoutMs = config.timeoutMs;

    markshot::translate_common::TranslateHttpResponse response;
    if (!markshot::translate_common::sendTranslateHttpRequest(request, &response, error)) {
        return false;
    }

    if (response.httpStatus < 200 || response.httpStatus >= 300) {
        // Try parsing error message from response body
        const QJsonDocument doc = QJsonDocument::fromJson(response.body);
        if (doc.isObject() && doc.object().contains(QStringLiteral("error"))) {
            const QString msg = doc.object().value(QStringLiteral("error")).toObject().value(QStringLiteral("message")).toString();
            if (error) {
                *error = QStringLiteral("gemini http %1: %2").arg(response.httpStatus).arg(msg);
            }
            return false;
        }

        if (error) {
            *error = QStringLiteral("gemini http %1: %2")
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

}  // namespace markshot::translate_gemini
