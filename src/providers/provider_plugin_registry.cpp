#include "providers/provider_plugin_registry.h"

#include "debug_log.h"
#include "markshot/code_scan_provider_plugin.h"
#include "markshot/ocr_provider_plugin.h"
#include "providers/provider_plugin_paths.h"
#include "markshot/translate_provider_plugin.h"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>
#include <QJsonObject>
#include <QJsonValue>
#include <QLibrary>
#include <QPluginLoader>

namespace markshot::providers {
namespace {

/**
 * 从插件加载器读取基础诊断信息。
 * @param entry 插件库文件信息。
 * @param loader 插件加载器。
 * @return 基础诊断信息。
 */
ProviderPluginInfo basePluginInfo(const QFileInfo &entry, const QPluginLoader &loader)
{
    ProviderPluginInfo info;
    info.path = entry.absoluteFilePath();
    const QJsonObject metadata = loader.metaData().value(QStringLiteral("MetaData")).toObject();
    info.metadataName = metadata.value(QStringLiteral("name")).toString();
    info.metadataVersion = metadata.value(QStringLiteral("version")).toString();
    info.metadataVendor = metadata.value(QStringLiteral("vendor")).toString();
    return info;
}

/**
 * 构造已匹配 provider 接口的诊断信息。
 * @param base 插件基础诊断信息。
 * @param capability 插件能力。
 * @param providerId provider 标识。
 * @param displayName 展示名称。
 * @param available 当前是否可用。
 * @param error 不可用原因。
 * @return provider 诊断信息。
 */
ProviderPluginInfo matchedPluginInfo(const ProviderPluginInfo &base,
                                     ProviderPluginCapability capability,
                                     const QString &providerId,
                                     const QString &displayName,
                                     bool available,
                                     const QString &error)
{
    ProviderPluginInfo info = base;
    info.capability = capability;
    info.providerId = providerId;
    info.displayName = displayName;
    info.loaded = true;
    info.matched = true;
    info.available = available;
    info.error = error;
    return info;
}

}  // namespace

ProviderPluginRegistry &ProviderPluginRegistry::instance()
{
    static ProviderPluginRegistry registry;
    return registry;
}

QStringList ProviderPluginRegistry::pluginSearchDirs()
{
    return markshot::providers::pluginSearchDirs();
}

QVector<markshot::plugin::OcrProviderPlugin *> ProviderPluginRegistry::ocrProviders()
{
    loadOnce();
    return m_ocrProviders;
}

QVector<markshot::plugin::TranslateProviderPlugin *> ProviderPluginRegistry::translateProviders()
{
    loadOnce();
    return m_translateProviders;
}

QVector<markshot::plugin::CodeScanProviderPlugin *> ProviderPluginRegistry::codeScanProviders()
{
    loadOnce();
    return m_codeScanProviders;
}

QVector<ProviderPluginInfo> ProviderPluginRegistry::pluginInfos()
{
    loadOnce();
    return m_pluginInfos;
}

void ProviderPluginRegistry::loadOnce()
{
    if (m_loaded) {
        return;
    }
    m_loaded = true;

    for (const QString &dir : pluginSearchDirs()) {
        const QDir pluginDir(dir);
        if (!pluginDir.exists()) {
            continue;
        }
        for (const QFileInfo &entry : pluginDir.entryInfoList(QDir::Files)) {
            if (!QLibrary::isLibrary(entry.absoluteFilePath())) {
                continue;
            }
            // 1. 加载候选动态库并检查是否实现任一 provider 接口
            auto *loader = new QPluginLoader(entry.absoluteFilePath(), QCoreApplication::instance());
            const ProviderPluginInfo baseInfo = basePluginInfo(entry, *loader);
            QObject *instance = loader->instance();
            if (!instance) {
                ProviderPluginInfo info = baseInfo;
                info.error = loader->errorString();
                m_pluginInfos.append(info);
                markshot::debugLog("providers",
                                   "【插件】【加载失败】path=%s error=%s",
                                   entry.absoluteFilePath().toUtf8().constData(),
                                   loader->errorString().toUtf8().constData());
                loader->deleteLater();
                continue;
            }

            bool matched = false;
            if (auto *ocr = qobject_cast<markshot::plugin::OcrProviderPlugin *>(instance)) {
                m_ocrProviders.append(ocr);
                matched = true;
                QString error;
                const bool available = ocr->isAvailable(&error);
                m_pluginInfos.append(matchedPluginInfo(baseInfo,
                                                       ProviderPluginCapability::Ocr,
                                                       ocr->providerId(),
                                                       ocr->displayName(),
                                                       available,
                                                       error));
                markshot::debugLog("providers",
                                   "【插件】【OCR】loaded id=%s path=%s",
                                   ocr->providerId().toUtf8().constData(),
                                   entry.absoluteFilePath().toUtf8().constData());
            }
            if (auto *translate = qobject_cast<markshot::plugin::TranslateProviderPlugin *>(instance)) {
                m_translateProviders.append(translate);
                matched = true;
                QString error;
                const bool available = translate->isAvailable(&error);
                m_pluginInfos.append(matchedPluginInfo(baseInfo,
                                                       ProviderPluginCapability::Translation,
                                                       translate->providerId(),
                                                       translate->displayName(),
                                                       available,
                                                       error));
                markshot::debugLog("providers",
                                   "【插件】【翻译】loaded id=%s path=%s",
                                   translate->providerId().toUtf8().constData(),
                                   entry.absoluteFilePath().toUtf8().constData());
            }
            if (auto *codeScan = qobject_cast<markshot::plugin::CodeScanProviderPlugin *>(instance)) {
                m_codeScanProviders.append(codeScan);
                matched = true;
                QString error;
                const bool available = codeScan->isAvailable(&error);
                m_pluginInfos.append(matchedPluginInfo(baseInfo,
                                                       ProviderPluginCapability::CodeScan,
                                                       codeScan->providerId(),
                                                       codeScan->displayName(),
                                                       available,
                                                       error));
                markshot::debugLog("providers",
                                   "【插件】【扫码】loaded id=%s path=%s",
                                   codeScan->providerId().toUtf8().constData(),
                                   entry.absoluteFilePath().toUtf8().constData());
            }

            // 2. 与 provider 无关的插件立即卸载，避免占用内存
            if (!matched) {
                ProviderPluginInfo info = baseInfo;
                info.loaded = true;
                info.error = QStringLiteral("No supported provider interface");
                m_pluginInfos.append(info);
                loader->unload();
                loader->deleteLater();
            }
        }
    }
}

}  // namespace markshot::providers
