#include "settings/settings_page_plugins_model.h"

#include "providers/provider_plugin_paths.h"
#include "providers/provider_plugin_registry.h"
#include "ui/i18n.h"

#include <QFileInfo>
#include <QSet>

#include <algorithm>

namespace markshot::settings {
namespace {

/**
 * 构造固定 provider 选择项。
 * @return 固定选择项。
 */
QVector<ProviderOption> baseProviderOptions()
{
    return {
        {MS_TR("Auto"), QStringLiteral("auto")},
        {MS_TR("Plugin (any available)"), QStringLiteral("plugin")},
        {MS_TR("Builtin"), QStringLiteral("builtin")},
        {MS_TR("Legacy helper"), QStringLiteral("helper")},
    };
}

/**
 * 读取 provider 的可展示名称。
 * @param info 插件诊断信息。
 * @return 展示名称。
 */
QString providerLabel(const markshot::providers::ProviderPluginInfo &info)
{
    if (!info.displayName.isEmpty() && !info.providerId.isEmpty()) {
        return QStringLiteral("%1 (%2)").arg(info.displayName, info.providerId);
    }
    if (!info.displayName.isEmpty()) {
        return info.displayName;
    }
    return info.providerId;
}

/**
 * 读取插件状态文本。
 * @param info 插件诊断信息。
 * @return 状态文本。
 */
QString pluginStatus(const markshot::providers::ProviderPluginInfo &info)
{
    if (!info.loaded) {
        return MS_TR("Load failed");
    }
    if (!info.matched) {
        return MS_TR("Ignored");
    }
    return info.available ? MS_TR("Available") : MS_TR("Unavailable");
}

/**
 * 读取插件详情文本。
 * @param info 插件诊断信息。
 * @return 详情文本。
 */
QString pluginDetails(const markshot::providers::ProviderPluginInfo &info)
{
    QStringList details;
    if (!info.metadataName.isEmpty()) {
        QString metadata = info.metadataName;
        if (!info.metadataVersion.isEmpty()) {
            metadata += QStringLiteral(" ") + info.metadataVersion;
        }
        if (!info.metadataVendor.isEmpty()) {
            metadata += QStringLiteral(" / ") + info.metadataVendor;
        }
        details.append(metadata);
    }
    if (!info.error.isEmpty()) {
        details.append(info.error);
    }
    if (details.isEmpty()) {
        return QString();
    }
    return details.join(QStringLiteral("; "));
}

}  // namespace

QVector<ProviderOption> providerOptionsForCapability(markshot::providers::ProviderPluginCapability capability)
{
    QVector<ProviderOption> options = baseProviderOptions();
    QSet<QString> seenValues;
    for (const ProviderOption &option : options) {
        seenValues.insert(option.value);
    }

    QVector<markshot::providers::ProviderPluginInfo> infos =
        markshot::providers::ProviderPluginRegistry::instance().pluginInfos();
    std::sort(infos.begin(), infos.end(), [](const auto &left, const auto &right) {
        return providerLabel(left).localeAwareCompare(providerLabel(right)) < 0;
    });

    for (const markshot::providers::ProviderPluginInfo &info : infos) {
        if (!info.matched || info.capability != capability || info.providerId.isEmpty()) {
            continue;
        }
        const QString value = QStringLiteral("plugin:%1").arg(info.providerId);
        if (seenValues.contains(value)) {
            continue;
        }
        options.append({QStringLiteral("%1: %2").arg(MS_TR("Plugin"), providerLabel(info)), value});
        seenValues.insert(value);
    }
    return options;
}

QVector<PluginDiagnosticRow> pluginDiagnosticRows()
{
    QVector<PluginDiagnosticRow> rows;
    const QVector<markshot::providers::ProviderPluginInfo> infos =
        markshot::providers::ProviderPluginRegistry::instance().pluginInfos();
    for (const markshot::providers::ProviderPluginInfo &info : infos) {
        rows.append({markshot::i18n::translate(markshot::providers::providerPluginCapabilityName(info.capability)),
                     providerLabel(info),
                     pluginStatus(info),
                     QFileInfo(info.path).absoluteFilePath(),
                     pluginDetails(info)});
    }
    return rows;
}

QStringList pluginSearchDirectoryRows()
{
    return markshot::providers::ProviderPluginRegistry::pluginSearchDirs();
}

QString userPluginDirectory()
{
    return markshot::providers::userPluginDirectory();
}

}  // namespace markshot::settings
