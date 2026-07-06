#pragma once

#include <QJsonObject>
#include <QString>

namespace markshot::settings {

struct ProviderPreferenceConfig {
    QString ocrProvider = QStringLiteral("auto");
    QString translationProvider = QStringLiteral("auto");
    QString codeScanProvider = QStringLiteral("auto");
};

/**
 * 从应用配置根对象读取 provider 偏好。
 * @param root 应用配置根对象。
 * @return provider 偏好配置。
 */
ProviderPreferenceConfig providerPreferenceConfigFromRoot(const QJsonObject &root);

/**
 * 将 provider 偏好写入应用配置根对象。
 * @param root 应用配置根对象。
 * @param config provider 偏好配置。
 * @return 无返回值。
 */
void writeProviderPreferenceConfig(QJsonObject *root, const ProviderPreferenceConfig &config);

}  // namespace markshot::settings
