#include "settings/cloud_translate_settings.h"

namespace markshot::settings {
namespace {

/**
 * 读取厂商子节中的字符串字段。
 * @param translation 应用配置中的 translation 对象。
 * @param vendor 厂商子节名称。
 * @param key 字段名。
 * @param fallback 字段缺失或为空时的默认值。
 * @return 去除首尾空白后的取值。
 */
QString vendorString(const QJsonObject &translation,
                     const QString &vendor,
                     const QString &key,
                     const QString &fallback = QString())
{
    const QString value = translation.value(vendor).toObject().value(key).toString().trimmed();
    return value.isEmpty() ? fallback : value;
}

}  // namespace

CloudTranslateSettings readCloudTranslateSettings(const QJsonObject &translation)
{
    CloudTranslateSettings settings;
    settings.tencentSecretId = vendorString(translation, QStringLiteral("tencent"), QStringLiteral("secretId"));
    settings.tencentSecretKey = vendorString(translation, QStringLiteral("tencent"), QStringLiteral("secretKey"));
    settings.tencentRegion = vendorString(translation,
                                          QStringLiteral("tencent"),
                                          QStringLiteral("region"),
                                          settings.tencentRegion);
    settings.baiduAppId = vendorString(translation, QStringLiteral("baidu"), QStringLiteral("appId"));
    settings.baiduAppKey = vendorString(translation, QStringLiteral("baidu"), QStringLiteral("appKey"));
    settings.youdaoAppKey = vendorString(translation, QStringLiteral("youdao"), QStringLiteral("appKey"));
    settings.youdaoAppSecret = vendorString(translation, QStringLiteral("youdao"), QStringLiteral("appSecret"));
    settings.geminiApiKey = vendorString(translation, QStringLiteral("gemini"), QStringLiteral("apiKey"));
    settings.geminiModel = vendorString(translation, QStringLiteral("gemini"), QStringLiteral("model"));
    settings.geminiEndpoint = vendorString(translation, QStringLiteral("gemini"), QStringLiteral("endpoint"));
    settings.geminiSystemPrompt = vendorString(translation, QStringLiteral("gemini"), QStringLiteral("systemPrompt"));
    settings.anthropicApiKey = vendorString(translation, QStringLiteral("anthropic"), QStringLiteral("apiKey"));
    settings.anthropicModel = vendorString(translation, QStringLiteral("anthropic"), QStringLiteral("model"));
    settings.anthropicEndpoint = vendorString(translation, QStringLiteral("anthropic"), QStringLiteral("endpoint"));
    settings.anthropicSystemPrompt = vendorString(translation, QStringLiteral("anthropic"), QStringLiteral("systemPrompt"));
    return settings;
}

QVector<QPair<QStringList, QString>> cloudTranslateConfigEntries(const CloudTranslateSettings &settings)
{
    const QString translation = QStringLiteral("translation");
    const QString tencent = QStringLiteral("tencent");
    const QString baidu = QStringLiteral("baidu");
    const QString youdao = QStringLiteral("youdao");
    const QString gemini = QStringLiteral("gemini");
    const QString anthropic = QStringLiteral("anthropic");
    return {
        {{translation, tencent, QStringLiteral("secretId")}, settings.tencentSecretId.trimmed()},
        {{translation, tencent, QStringLiteral("secretKey")}, settings.tencentSecretKey.trimmed()},
        {{translation, tencent, QStringLiteral("region")}, settings.tencentRegion.trimmed()},
        {{translation, baidu, QStringLiteral("appId")}, settings.baiduAppId.trimmed()},
        {{translation, baidu, QStringLiteral("appKey")}, settings.baiduAppKey.trimmed()},
        {{translation, youdao, QStringLiteral("appKey")}, settings.youdaoAppKey.trimmed()},
        {{translation, youdao, QStringLiteral("appSecret")}, settings.youdaoAppSecret.trimmed()},
        {{translation, gemini, QStringLiteral("apiKey")}, settings.geminiApiKey.trimmed()},
        {{translation, gemini, QStringLiteral("model")}, settings.geminiModel.trimmed()},
        {{translation, gemini, QStringLiteral("endpoint")}, settings.geminiEndpoint.trimmed()},
        {{translation, gemini, QStringLiteral("systemPrompt")}, settings.geminiSystemPrompt.trimmed()},
        {{translation, anthropic, QStringLiteral("apiKey")}, settings.anthropicApiKey.trimmed()},
        {{translation, anthropic, QStringLiteral("model")}, settings.anthropicModel.trimmed()},
        {{translation, anthropic, QStringLiteral("endpoint")}, settings.anthropicEndpoint.trimmed()},
        {{translation, anthropic, QStringLiteral("systemPrompt")}, settings.anthropicSystemPrompt.trimmed()},
    };
}

}  // namespace markshot::settings
