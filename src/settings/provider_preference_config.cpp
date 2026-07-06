#include "settings/provider_preference_config.h"

#include "config_value.h"

namespace markshot::settings {
namespace {

/**
 * 读取 provider 字段并做基础归一化。
 * @param object 配置对象。
 * @param fallback 默认值。
 * @return provider 配置值。
 */
QString providerValue(const QJsonObject &object, const QString &fallback)
{
    const QString value = object.value(QStringLiteral("provider")).toString().trimmed().toLower();
    return value.isEmpty() ? fallback : value;
}

/**
 * 向嵌套对象写入 provider 字段。
 * @param root 应用配置根对象。
 * @param key 顶层配置键。
 * @param provider provider 配置值。
 * @return 无返回值。
 */
void writeTopLevelProvider(QJsonObject *root, const QString &key, const QString &provider)
{
    if (!root) {
        return;
    }
    QJsonObject object = root->value(key).isObject() ? root->value(key).toObject() : QJsonObject();
    object.insert(QStringLiteral("provider"), provider.trimmed().toLower());
    root->insert(key, object);
}

}  // namespace

ProviderPreferenceConfig providerPreferenceConfigFromRoot(const QJsonObject &root)
{
    ProviderPreferenceConfig config;
    const QJsonObject ocr = markshot::config::firstObjectValue(root, QStringLiteral("ocr"));
    config.ocrProvider = providerValue(ocr, config.ocrProvider);

    const QJsonObject translation = markshot::config::firstObjectValue(root, QStringLiteral("translation"));
    config.translationProvider = providerValue(translation, config.translationProvider);

    const QJsonObject codeScan =
        markshot::config::firstObjectValue(root,
                                           {QStringLiteral("codeScan"),
                                            QStringLiteral("codeScanner"),
                                            QStringLiteral("barcodeScanner"),
                                            QStringLiteral("barcode")});
    config.codeScanProvider = providerValue(codeScan, config.codeScanProvider);
    return config;
}

void writeProviderPreferenceConfig(QJsonObject *root, const ProviderPreferenceConfig &config)
{
    writeTopLevelProvider(root, QStringLiteral("ocr"), config.ocrProvider);
    writeTopLevelProvider(root, QStringLiteral("translation"), config.translationProvider);
    writeTopLevelProvider(root, QStringLiteral("codeScan"), config.codeScanProvider);
}

}  // namespace markshot::settings
