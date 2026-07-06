#include "providers/provider_plugin_info.h"

namespace markshot::providers {

QString providerPluginCapabilityName(ProviderPluginCapability capability)
{
    switch (capability) {
    case ProviderPluginCapability::Ocr:
        return QStringLiteral("OCR");
    case ProviderPluginCapability::Translation:
        return QStringLiteral("Translation");
    case ProviderPluginCapability::CodeScan:
        return QStringLiteral("Code Scanner");
    case ProviderPluginCapability::Unknown:
        break;
    }
    return QStringLiteral("Plugin");
}

}  // namespace markshot::providers
