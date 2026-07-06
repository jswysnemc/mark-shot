#pragma once

#include <QString>

namespace markshot::providers {

enum class ProviderPluginCapability {
    Unknown,
    Ocr,
    Translation,
    CodeScan,
};

struct ProviderPluginInfo {
    QString path;
    QString metadataName;
    QString metadataVersion;
    QString metadataVendor;
    ProviderPluginCapability capability = ProviderPluginCapability::Unknown;
    QString providerId;
    QString displayName;
    bool loaded = false;
    bool matched = false;
    bool available = false;
    QString error;
};

/**
 * 读取插件能力的展示名称。
 * @param capability 插件能力。
 * @return 能力展示名称。
 */
QString providerPluginCapabilityName(ProviderPluginCapability capability);

}  // namespace markshot::providers
