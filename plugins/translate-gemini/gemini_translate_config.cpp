#include "gemini_translate_config.h"

#include "translate_config_source.h"

#include <QJsonValue>
#include <QStringList>

namespace markshot::translate_gemini {
namespace {

using markshot::translate_common::TranslateConfigSource;

/**
 * 读取 API Key 的环境变量候选名。
 * @return 按优先级排列的环境变量名列表。
 */
QStringList apiKeyEnvNames()
{
    return {QStringLiteral("GEMINI_API_KEY"), QStringLiteral("MARK_SHOT_GEMINI_API_KEY")};
}

/**
 * 读取 Model 的环境变量候选名。
 * @return 环境变量名列表。
 */
QStringList modelEnvNames()
{
    return {QStringLiteral("GEMINI_MODEL"), QStringLiteral("MARK_SHOT_GEMINI_MODEL")};
}

/**
 * 读取 Endpoint 的环境变量候选名。
 * @return 环境变量名列表。
 */
QStringList endpointEnvNames()
{
    return {QStringLiteral("GEMINI_API_BASE"), QStringLiteral("MARK_SHOT_GEMINI_API_BASE")};
}

/**
 * 读取厂商子节中显式配置的浮点数。
 * @param source 配置来源。
 * @param key 配置键名。
 * @return 已配置且为数值时返回取值，否则返回空。
 */
std::optional<double> vendorDouble(const TranslateConfigSource &source, const QString &key)
{
    const QJsonValue value = source.vendor.value(key);
    if (!value.isDouble()) {
        return std::nullopt;
    }
    return value.toDouble();
}

}  // namespace

QString defaultSystemPrompt()
{
    return QStringLiteral(
        "You are a professional native translator. Fluently translate OCR text segments into the target language. "
        "Preserve original meaning, keep segment count and ids unchanged. "
        "Keep proper nouns, brand names, code snippets, formulas, and numbers intact. "
        "Return only valid JSON without extra text or explanations.");
}

GeminiTranslateConfig readGeminiTranslateConfig()
{
    TranslateConfigSource source =
        markshot::translate_common::readTranslateConfigSource(QStringLiteral("gemini"));
    // 1. 只读 translation.gemini 子节：公共节的 apiKey/model/systemPrompt 属于 OpenAI 兼容服务
    source.translation = QJsonObject();

    GeminiTranslateConfig result;

    result.apiKey = markshot::translate_common::configString(source, QStringLiteral("apiKey"), apiKeyEnvNames());
    result.model = markshot::translate_common::configString(source, QStringLiteral("model"), modelEnvNames(), result.model);
    result.endpoint = markshot::translate_common::configString(source, QStringLiteral("endpoint"), endpointEnvNames(), result.endpoint);
    result.systemPrompt = markshot::translate_common::configString(source,
                                                                   QStringLiteral("systemPrompt"),
                                                                   {},
                                                                   defaultSystemPrompt());
    result.temperature = vendorDouble(source, QStringLiteral("temperature"));
    result.thinkingLevel = markshot::translate_common::configString(source, QStringLiteral("thinkingLevel"), {});
    result.timeoutMs = markshot::translate_common::configInt(source,
                                                             QStringLiteral("timeoutMs"),
                                                             result.timeoutMs,
                                                             1000,
                                                             300000);
    return result;
}

bool validateGeminiTranslateConfig(const GeminiTranslateConfig &config, QString *error)
{
    if (config.endpoint.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("missing gemini endpoint");
        }
        return false;
    }
    if (config.model.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("missing gemini model");
        }
        return false;
    }
    if (config.apiKey.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("missing gemini apiKey: set translation.gemini.apiKey or GEMINI_API_KEY");
        }
        return false;
    }
    return true;
}

}  // namespace markshot::translate_gemini
