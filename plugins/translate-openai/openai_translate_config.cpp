#include "openai_translate_config.h"

#include "translate_config_source.h"

namespace markshot::translate_openai {
namespace {

using markshot::translate_common::TranslateConfigSource;

/**
 * 读取配置来源中的浮点数取值。
 * @param source 配置来源。
 * @param key 配置键名。
 * @param fallback 缺失时的默认值。
 * @return 厂商子节优先、公共节兜底的浮点取值。
 */
double configDouble(const TranslateConfigSource &source, const QString &key, double fallback)
{
    double value = fallback;
    if (source.translation.contains(key)) {
        value = source.translation.value(key).toDouble(value);
    }
    if (source.vendor.contains(key)) {
        value = source.vendor.value(key).toDouble(value);
    }
    return value;
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

OpenAiTranslateConfig readOpenAiTranslateConfig()
{
    const TranslateConfigSource source =
        markshot::translate_common::readTranslateConfigSource(QStringLiteral("openai"));

    OpenAiTranslateConfig result;
    result.extraBody = source.translation.value(QStringLiteral("extraBody"));

    // 1. apiBase 兼容旧配置使用的 baseUrl 键名
    QString apiBase = markshot::translate_common::configString(source, QStringLiteral("apiBase"), {});
    if (apiBase.isEmpty()) {
        apiBase = markshot::translate_common::configString(source,
                                                           QStringLiteral("baseUrl"),
                                                           {QStringLiteral("MARK_SHOT_LLM_API_BASE"),
                                                            QStringLiteral("OPENAI_BASE_URL"),
                                                            QStringLiteral("OPENAI_API_BASE")},
                                                           result.apiBase);
    }
    result.apiBase = apiBase;

    result.model = markshot::translate_common::configString(
        source,
        QStringLiteral("model"),
        {QStringLiteral("MARK_SHOT_LLM_MODEL"), QStringLiteral("OPENAI_MODEL")},
        result.model);

    // 2. apiKeyEnv 决定密钥从哪个环境变量读取，需先于 apiKey 解析
    result.apiKeyEnv =
        markshot::translate_common::configString(source, QStringLiteral("apiKeyEnv"), {}, result.apiKeyEnv);
    result.apiKey = markshot::translate_common::configString(
        source,
        QStringLiteral("apiKey"),
        {result.apiKeyEnv, QStringLiteral("MARK_SHOT_LLM_API_KEY")},
        {});

    result.systemPrompt = markshot::translate_common::configString(source,
                                                                   QStringLiteral("systemPrompt"),
                                                                   {},
                                                                   defaultSystemPrompt());
    result.temperature = configDouble(source, QStringLiteral("temperature"), result.temperature);
    result.timeoutMs = markshot::translate_common::configInt(source,
                                                             QStringLiteral("timeoutMs"),
                                                             result.timeoutMs,
                                                             1000,
                                                             300000);
    return result;
}

bool validateOpenAiTranslateConfig(const OpenAiTranslateConfig &config, QString *error)
{
    if (!config.extraBody.isUndefined() && !config.extraBody.isObject()) {
        if (error) {
            *error = QStringLiteral("translation.extraBody must be a JSON object");
        }
        return false;
    }
    if (config.apiBase.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("missing translation apiBase");
        }
        return false;
    }
    if (config.model.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("missing translation model");
        }
        return false;
    }
    if (config.apiKey.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("missing api key: set %1 or translation.apiKey").arg(config.apiKeyEnv);
        }
        return false;
    }
    return true;
}

}  // namespace markshot::translate_openai
