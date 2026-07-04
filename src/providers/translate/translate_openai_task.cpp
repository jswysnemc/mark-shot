#include "providers/translate/translate_openai_task.h"

#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QRegularExpression>

#include <utility>

namespace markshot::providers {
namespace {

/**
 * 读取应用配置中的 translation 节。
 * @param configPath 配置文件路径。
 * @return translation JSON 对象，读取失败时为空对象。
 */
QJsonObject translationConfig(const QString &configPath)
{
    QFile file(configPath);
    if (configPath.isEmpty() || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return {};
    }
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll());
    if (!document.isObject()) {
        return {};
    }
    return document.object().value(QStringLiteral("translation")).toObject();
}

/**
 * 按配置与环境变量解析首个非空取值。
 * @param configValue 配置取值。
 * @param envNames 依次尝试的环境变量名。
 * @param fallback 兜底值。
 * @return 解析结果。
 */
QString firstNonEmpty(const QString &configValue, const QStringList &envNames, const QString &fallback)
{
    if (!configValue.trimmed().isEmpty()) {
        return configValue.trimmed();
    }
    for (const QString &name : envNames) {
        const QString value = qEnvironmentVariable(name.toUtf8().constData()).trimmed();
        if (!value.isEmpty()) {
            return value;
        }
    }
    return fallback;
}

/**
 * 从模型回复文本解析 {translations:[{id,text}]}。
 * @param content 回复文本。
 * @param translations 输出 id 到译文映射。
 * @param error 输出错误信息。
 * @return 解析成功时返回 true。
 */
bool parseTranslationContent(const QString &content, QHash<int, QString> *translations, QString *error)
{
    QString trimmed = content.trimmed();
    QJsonDocument document = QJsonDocument::fromJson(trimmed.toUtf8());
    if (!document.isObject()) {
        // 1. 兼容模型输出被 markdown 包裹的场景，截取首个大括号 JSON
        const QRegularExpression pattern(QStringLiteral("\\{.*\\}"),
                                         QRegularExpression::DotMatchesEverythingOption);
        const QRegularExpressionMatch match = pattern.match(trimmed);
        if (match.hasMatch()) {
            document = QJsonDocument::fromJson(match.captured(0).toUtf8());
        }
    }
    if (!document.isObject()) {
        *error = QStringLiteral("llm response is not valid JSON");
        return false;
    }
    const QJsonValue value = document.object().value(QStringLiteral("translations"));
    if (!value.isArray()) {
        *error = QStringLiteral("llm response missing translations array");
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

TranslateOpenAiTask::TranslateOpenAiTask(QByteArray inputJson,
                                         QString targetLanguage,
                                         QString configPath,
                                         QObject *parent)
    : ProviderTask(QStringLiteral("openai-compatible"), parent)
    , m_inputJson(std::move(inputJson))
    , m_targetLanguage(std::move(targetLanguage))
    , m_configPath(std::move(configPath))
{
    m_network = new QNetworkAccessManager(this);
    m_timeoutTimer.setSingleShot(true);
    connect(&m_timeoutTimer, &QTimer::timeout, this, [this] {
        if (m_reply) {
            m_reply->abort();
        }
        emitFinished({false, TaskError::Timeout, {}, {}, {}});
    });
}

void TranslateOpenAiTask::start(int timeoutMs)
{
    // 1. 解析输入分段，与旧 helper 相同的行合并逻辑
    QString inputLanguage;
    m_segments = translateSegmentsFromInputJson(m_inputJson, &inputLanguage);
    if (m_targetLanguage.trimmed().isEmpty()) {
        m_targetLanguage = inputLanguage.isEmpty() ? QStringLiteral("Simplified Chinese") : inputLanguage;
    }
    if (m_segments.isEmpty()) {
        failWith(QStringLiteral("no source text"));
        return;
    }

    // 2. 解析 API 配置，配置项与环境变量兜底顺序与旧 helper 一致
    const QJsonObject config = translationConfig(m_configPath);
    const QString apiBase = firstNonEmpty(
        config.value(QStringLiteral("apiBase")).toString(config.value(QStringLiteral("baseUrl")).toString()),
        {QStringLiteral("MARK_SHOT_LLM_API_BASE"),
         QStringLiteral("OPENAI_BASE_URL"),
         QStringLiteral("OPENAI_API_BASE")},
        QStringLiteral("https://api.openai.com/v1"));
    const QString model = firstNonEmpty(config.value(QStringLiteral("model")).toString(),
                                        {QStringLiteral("MARK_SHOT_LLM_MODEL"), QStringLiteral("OPENAI_MODEL")},
                                        QStringLiteral("gpt-4o-mini"));
    QString apiKey = config.value(QStringLiteral("apiKey")).toString().trimmed();
    const QString apiKeyEnv = firstNonEmpty(config.value(QStringLiteral("apiKeyEnv")).toString(),
                                            {},
                                            QStringLiteral("OPENAI_API_KEY"));
    if (apiKey.isEmpty()) {
        apiKey = firstNonEmpty({}, {apiKeyEnv, QStringLiteral("MARK_SHOT_LLM_API_KEY")}, {});
    }
    if (apiKey.isEmpty()) {
        failWith(QStringLiteral("missing api key: set %1 or translation.apiKey").arg(apiKeyEnv));
        return;
    }

    // 3. 组装 chat/completions 请求
    const QString systemPrompt = firstNonEmpty(
        config.value(QStringLiteral("systemPrompt")).toString(),
        {},
        QStringLiteral("You translate OCR text segments. Preserve meaning, keep segment count and ids "
                       "unchanged, and return only valid JSON."));
    QJsonArray segmentArray;
    for (const TranslateSourceSegment &segment : m_segments) {
        segmentArray.append(QJsonObject{{QStringLiteral("id"), segment.id},
                                        {QStringLiteral("text"), segment.text}});
    }
    const QJsonObject userPrompt{
        {QStringLiteral("target_language"), m_targetLanguage},
        {QStringLiteral("instructions"),
         QJsonArray{QStringLiteral("Translate each segment into target_language."),
                    QStringLiteral("Return JSON exactly as {\"translations\":[{\"id\":0,\"text\":\"...\"}]} "
                                   "with no markdown."),
                    QStringLiteral("Do not add explanations.")}},
        {QStringLiteral("segments"), segmentArray}};
    const QJsonObject payload{
        {QStringLiteral("model"), model},
        {QStringLiteral("temperature"),
         config.contains(QStringLiteral("temperature"))
             ? config.value(QStringLiteral("temperature")).toDouble(0.2)
             : 0.2},
        {QStringLiteral("messages"),
         QJsonArray{QJsonObject{{QStringLiteral("role"), QStringLiteral("system")},
                                {QStringLiteral("content"), systemPrompt}},
                    QJsonObject{{QStringLiteral("role"), QStringLiteral("user")},
                                {QStringLiteral("content"),
                                 QString::fromUtf8(QJsonDocument(userPrompt).toJson(QJsonDocument::Compact))}}}}};

    QNetworkRequest request(QUrl(apiBase + QStringLiteral("/chat/completions")));
    request.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));
    request.setRawHeader(QByteArrayLiteral("Authorization"), "Bearer " + apiKey.toUtf8());

    if (timeoutMs > 0) {
        m_timeoutTimer.start(timeoutMs);
    }
    m_reply = m_network->post(request, QJsonDocument(payload).toJson(QJsonDocument::Compact));
    connect(m_reply, &QNetworkReply::finished, this, &TranslateOpenAiTask::handleReply);
}

void TranslateOpenAiTask::cancel()
{
    m_timeoutTimer.stop();
    if (m_reply) {
        m_reply->disconnect(this);
        m_reply->abort();
        m_reply->deleteLater();
        m_reply = nullptr;
    }
}

void TranslateOpenAiTask::handleReply()
{
    m_timeoutTimer.stop();
    QNetworkReply *reply = m_reply;
    m_reply = nullptr;
    if (!reply) {
        return;
    }
    reply->deleteLater();

    const QByteArray body = reply->readAll();
    if (reply->error() != QNetworkReply::NoError) {
        const int status = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
        failWith(QStringLiteral("llm http %1: %2")
                     .arg(status)
                     .arg(QString::fromUtf8(body.left(500))));
        return;
    }

    // 1. 提取回复 content 并解析译文映射
    const QJsonDocument document = QJsonDocument::fromJson(body);
    const QString content = document.object()
                                .value(QStringLiteral("choices"))
                                .toArray()
                                .at(0)
                                .toObject()
                                .value(QStringLiteral("message"))
                                .toObject()
                                .value(QStringLiteral("content"))
                                .toString();
    QHash<int, QString> translations;
    QString parseError;
    if (!parseTranslationContent(content, &translations, &parseError)) {
        failWith(parseError);
        return;
    }

    emitFinished({true,
                  TaskError::None,
                  translateTokensJson(m_segments, translations, QStringLiteral("openai-compatible")),
                  {},
                  {}});
}

void TranslateOpenAiTask::failWith(const QString &message)
{
    QJsonObject root;
    root.insert(QStringLiteral("backend"), QStringLiteral("openai-compatible"));
    root.insert(QStringLiteral("tokens"), QJsonArray());
    root.insert(QStringLiteral("errors"), QJsonArray{message});
    emitFinished({false,
                  TaskError::Failed,
                  QJsonDocument(root).toJson(QJsonDocument::Compact),
                  message.toUtf8(),
                  {}});
}

}  // namespace markshot::providers
