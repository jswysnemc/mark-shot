#include "openai_translate_plugin.h"

#include "openai_translate_config.h"
#include "openai_translation_parser.h"

#include <QEventLoop>
#include <QHash>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QTimer>

namespace markshot::translate_openai {
namespace {

/**
 * 拼接 chat/completions 端点。
 * @param apiBase API 根地址。
 * @return chat/completions 完整地址。
 */
QUrl chatCompletionsUrl(QString apiBase)
{
    while (apiBase.endsWith(QLatin1Char('/'))) {
        apiBase.chop(1);
    }
    return QUrl(apiBase + QStringLiteral("/chat/completions"));
}

/**
 * 构造 OpenAI-compatible 请求体。
 * @param config 翻译配置。
 * @param segments 源文分段。
 * @param targetLanguage 目标语言。
 * @return JSON 请求体。
 */
QByteArray requestPayload(const OpenAiTranslateConfig &config,
                          const QVector<markshot::plugin::TranslateSegment> &segments,
                          const QString &targetLanguage)
{
    QJsonArray segmentArray;
    for (const markshot::plugin::TranslateSegment &segment : segments) {
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

    const QJsonObject payload{
        {QStringLiteral("model"), config.model},
        {QStringLiteral("temperature"), config.temperature},
        {QStringLiteral("messages"),
         QJsonArray{QJsonObject{{QStringLiteral("role"), QStringLiteral("system")},
                                {QStringLiteral("content"), config.systemPrompt}},
                    QJsonObject{{QStringLiteral("role"), QStringLiteral("user")},
                                {QStringLiteral("content"),
                                 QString::fromUtf8(QJsonDocument(userPrompt).toJson(QJsonDocument::Compact))}}}}};
    return QJsonDocument(payload).toJson(QJsonDocument::Compact);
}

/**
 * 从 chat/completions 响应体提取 message.content。
 * @param body HTTP 响应体。
 * @param content 输出模型回复正文。
 * @param error 输出错误信息。
 * @return 提取成功时返回 true。
 */
bool extractMessageContent(const QByteArray &body, QString *content, QString *error)
{
    const QJsonDocument document = QJsonDocument::fromJson(body);
    const QJsonArray choices = document.object().value(QStringLiteral("choices")).toArray();
    if (choices.isEmpty()) {
        if (error) {
            *error = QStringLiteral("llm response missing choices");
        }
        return false;
    }
    const QString value = choices.at(0)
                              .toObject()
                              .value(QStringLiteral("message"))
                              .toObject()
                              .value(QStringLiteral("content"))
                              .toString();
    if (value.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("llm response missing message content");
        }
        return false;
    }
    if (content) {
        *content = value;
    }
    return true;
}

/**
 * 同步发送 HTTP 翻译请求。
 * @param config 翻译配置。
 * @param payload JSON 请求体。
 * @param body 输出响应体。
 * @param error 输出错误信息。
 * @return 请求成功时返回 true。
 */
bool postTranslationRequest(const OpenAiTranslateConfig &config,
                            const QByteArray &payload,
                            QByteArray *body,
                            QString *error)
{
    QNetworkAccessManager network;
    QNetworkRequest request(chatCompletionsUrl(config.apiBase));
    request.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));
    request.setRawHeader(QByteArrayLiteral("Authorization"), "Bearer " + config.apiKey.toUtf8());

    QEventLoop loop;
    QTimer timeoutTimer;
    timeoutTimer.setSingleShot(true);
    QNetworkReply *reply = network.post(request, payload);
    QObject::connect(reply, &QNetworkReply::finished, &loop, &QEventLoop::quit);
    QObject::connect(&timeoutTimer, &QTimer::timeout, &loop, [&loop, reply] {
        // 1. 超时后中断网络请求并退出局部事件循环
        reply->abort();
        loop.quit();
    });
    timeoutTimer.start(config.timeoutMs);
    loop.exec();

    const bool timedOut = !timeoutTimer.isActive();
    timeoutTimer.stop();
    const QByteArray responseBody = reply->readAll();
    const QNetworkReply::NetworkError replyError = reply->error();
    const int httpStatus = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
    reply->deleteLater();

    if (timedOut) {
        if (error) {
            *error = QStringLiteral("llm request timed out");
        }
        return false;
    }
    if (replyError != QNetworkReply::NoError) {
        if (error) {
            *error = QStringLiteral("llm http %1: %2")
                         .arg(httpStatus)
                         .arg(QString::fromUtf8(responseBody.left(500)));
        }
        return false;
    }
    if (body) {
        *body = responseBody;
    }
    return true;
}

}  // namespace

QString OpenAiTranslatePlugin::providerId() const
{
    return QStringLiteral("openai-compatible");
}

QString OpenAiTranslatePlugin::displayName() const
{
    return QStringLiteral("OpenAI Compatible");
}

bool OpenAiTranslatePlugin::isAvailable(QString *error) const
{
    return validateOpenAiTranslateConfig(readOpenAiTranslateConfig(), error);
}

bool OpenAiTranslatePlugin::translate(const QVector<markshot::plugin::TranslateSegment> &segments,
                                      const QString &targetLanguage,
                                      QVector<markshot::plugin::TranslateSegment> *translations,
                                      QString *error)
{
    if (!translations) {
        if (error) {
            *error = QStringLiteral("translation output target is missing");
        }
        return false;
    }
    translations->clear();
    if (segments.isEmpty()) {
        return true;
    }

    const OpenAiTranslateConfig config = readOpenAiTranslateConfig();
    if (!validateOpenAiTranslateConfig(config, error)) {
        return false;
    }

    QByteArray body;
    if (!postTranslationRequest(config, requestPayload(config, segments, targetLanguage), &body, error)) {
        return false;
    }

    QString content;
    if (!extractMessageContent(body, &content, error)) {
        return false;
    }
    QHash<int, QString> translatedById;
    if (!parseTranslationContent(content, &translatedById, error)) {
        return false;
    }

    for (const markshot::plugin::TranslateSegment &segment : segments) {
        const QString text = translatedById.value(segment.id).trimmed();
        translations->append({segment.id, text.isEmpty() ? segment.text : text});
    }
    return true;
}

}  // namespace markshot::translate_openai
