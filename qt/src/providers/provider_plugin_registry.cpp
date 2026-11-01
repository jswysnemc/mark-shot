#include "providers/provider_plugin_registry.h"

#include "debug_log.h"
#include "marketplace/plugin_updates.h"
#include "markshot/code_scan_provider_plugin.h"
#include "markshot/ocr_provider_plugin.h"
#include "providers/provider_plugin_paths.h"
#include "providers/provider_runtime_dependencies.h"
#include "markshot/translate_provider_plugin.h"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>
#include <QLibrary>
#include <QPluginLoader>

#include <algorithm>

namespace markshot::providers {
namespace {

/**
 * 读取插件元数据中的自定义字段。
 * @param loader 插件加载器。
 * @return metadata.json 内容。
 */
QJsonObject pluginMetadata(const QPluginLoader &loader)
{
    return loader.metaData().value(QStringLiteral("MetaData")).toObject();
}

/**
 * 从插件加载器读取基础诊断信息。
 * @param path 插件库绝对路径。
 * @param loader 插件加载器。
 * @return 基础诊断信息。
 */
ProviderPluginInfo basePluginInfo(const QString &path, const QPluginLoader &loader)
{
    ProviderPluginInfo info;
    info.path = path;
    const QJsonObject metadata = pluginMetadata(loader);
    info.metadataName = metadata.value(QStringLiteral("name")).toString();
    info.metadataVersion = metadata.value(QStringLiteral("version")).toString();
    info.metadataVendor = metadata.value(QStringLiteral("vendor")).toString();
    return info;
}

/**
 * 读取插件元数据声明的能力，不加载动态库。
 * @param loader 插件加载器。
 * @return 已知能力列表，未声明或全部未知时为空。
 */
QVector<ProviderPluginCapability> declaredCapabilities(const QPluginLoader &loader)
{
    QVector<ProviderPluginCapability> capabilities;
    const QJsonArray entries = pluginMetadata(loader).value(QStringLiteral("capabilities")).toArray();
    for (const QJsonValue &entry : entries) {
        const QString type = entry.toObject().value(QStringLiteral("type")).toString();
        ProviderPluginCapability capability = ProviderPluginCapability::Unknown;
        if (type == QStringLiteral("ocr")) {
            capability = ProviderPluginCapability::Ocr;
        } else if (type == QStringLiteral("translation")) {
            capability = ProviderPluginCapability::Translation;
        } else if (type == QStringLiteral("code-scan")) {
            capability = ProviderPluginCapability::CodeScan;
        }
        if (capability != ProviderPluginCapability::Unknown && !capabilities.contains(capability)) {
            capabilities.append(capability);
        }
    }
    return capabilities;
}

/**
 * 读取元数据里声明的 provider 标识，不加载动态库。
 * @param loader 插件加载器。
 * @return 非空标识，未声明时为空。
 */
QStringList declaredProviderIds(const QPluginLoader &loader)
{
    QStringList ids;
    const QJsonArray entries = pluginMetadata(loader).value(QStringLiteral("capabilities")).toArray();
    for (const QJsonValue &entry : entries) {
        const QString id = entry.toObject().value(QStringLiteral("providerId")).toString();
        if (!id.isEmpty() && !ids.contains(id)) {
            ids.append(id);
        }
    }
    return ids;
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

/**
 * 按候选发现顺序收集实现指定接口的插件实例。
 * @param candidates 候选插件列表。
 * @return 插件接口指针列表。
 */
template <typename Plugin>
QVector<Plugin *> collectProviders(const QVector<ProviderPluginCandidate> &candidates)
{
    QVector<Plugin *> providers;
    for (const ProviderPluginCandidate &candidate : candidates) {
        if (auto *plugin = qobject_cast<Plugin *>(candidate.instance)) {
            providers.append(plugin);
        }
    }
    return providers;
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
    loadCapability(ProviderPluginCapability::Ocr);
    return collectProviders<markshot::plugin::OcrProviderPlugin>(m_candidates);
}

QVector<markshot::plugin::TranslateProviderPlugin *> ProviderPluginRegistry::translateProviders()
{
    loadCapability(ProviderPluginCapability::Translation);
    return collectProviders<markshot::plugin::TranslateProviderPlugin>(m_candidates);
}

QVector<markshot::plugin::CodeScanProviderPlugin *> ProviderPluginRegistry::codeScanProviders()
{
    loadCapability(ProviderPluginCapability::CodeScan);
    return collectProviders<markshot::plugin::CodeScanProviderPlugin>(m_candidates);
}

QVector<ProviderPluginInfo> ProviderPluginRegistry::pluginInfos()
{
    loadAll();
    QVector<ProviderPluginInfo> infos;
    for (ProviderPluginCandidate &candidate : m_candidates) {
        for (ProviderPluginInfo &info : candidate.infos) {
            // 1. 【插件】【可用状态】模型下载或配置变更后重新检查，不复用首次加载时的错误
            if (candidate.instance && info.matched) {
                info.error.clear();
                if (info.capability == ProviderPluginCapability::Ocr) {
                    info.available = qobject_cast<markshot::plugin::OcrProviderPlugin *>(candidate.instance)
                                         ->isAvailable(&info.error);
                } else if (info.capability == ProviderPluginCapability::Translation) {
                    info.available = qobject_cast<markshot::plugin::TranslateProviderPlugin *>(candidate.instance)
                                         ->isAvailable(&info.error);
                } else if (info.capability == ProviderPluginCapability::CodeScan) {
                    info.available = qobject_cast<markshot::plugin::CodeScanProviderPlugin *>(candidate.instance)
                                         ->isAvailable(&info.error);
                }
            }
            infos.append(info);
        }
    }
    return infos;
}

void ProviderPluginRegistry::discoverOnce()
{
    if (m_discovered) {
        return;
    }
    m_discovered = true;

    for (const QString &error : markshot::marketplace::applyPendingPluginUpdates(userPluginDirectory())) {
        markshot::debugLog("providers", "【插件】【启动更新】%s", error.toUtf8().constData());
    }

    for (const QString &dir : pluginSearchDirs()) {
        const QDir pluginDir(dir);
        if (!pluginDir.exists()) {
            continue;
        }
        for (const QFileInfo &entry : pluginDir.entryInfoList(QDir::Files)) {
            if (!QLibrary::isLibrary(entry.absoluteFilePath())) {
                continue;
            }
            // 1. 【插件】【元数据扫描】只读取插件元数据，不映射动态库及其依赖
            ProviderPluginCandidate candidate;
            candidate.path = entry.absoluteFilePath();
            // 2. 【插件】【元数据扫描】先清除默认驻留标记再关联路径，避免污染同目录非插件运行库的共享句柄
            candidate.loader = new QPluginLoader(QCoreApplication::instance());
            candidate.loader->setLoadHints({});
            candidate.loader->setFileName(candidate.path);
            candidate.declaredCapabilities = declaredCapabilities(*candidate.loader);
            m_candidates.append(std::move(candidate));
        }
    }
}

void ProviderPluginRegistry::loadCapability(ProviderPluginCapability capability)
{
    discoverOnce();
    for (ProviderPluginCandidate &candidate : m_candidates) {
        // 1. 【插件】【按需加载】未声明能力的旧插件无法预判接口，保持加载以兼容第三方插件
        if (candidate.declaredCapabilities.isEmpty() || candidate.declaredCapabilities.contains(capability)) {
            loadCandidate(&candidate);
        }
    }
}

void ProviderPluginRegistry::loadAll()
{
    discoverOnce();
    for (ProviderPluginCandidate &candidate : m_candidates) {
        loadCandidate(&candidate);
    }
}

void ProviderPluginRegistry::loadCandidate(ProviderPluginCandidate *candidate)
{
    if (candidate->attempted) {
        return;
    }
    candidate->attempted = true;

    // 1. 同一 provider 只保留搜索顺序中的第一份。用户目录在前，后面的构建目录副本不再映射。
    QPluginLoader *loader = candidate->loader;
    const QStringList declaredIds = declaredProviderIds(*loader);
    const bool duplicate = !declaredIds.isEmpty()
        && std::all_of(declaredIds.cbegin(), declaredIds.cend(), [this](const QString &id) {
               return m_loadedProviderIds.contains(id);
           });
    if (duplicate) {
        markshot::debugLog("providers",
                           "【插件】【跳过重复】id=%s path=%s",
                           declaredIds.join(QLatin1Char(',')).toUtf8().constData(),
                           candidate->path.toUtf8().constData());
        loader->unload();
        loader->deleteLater();
        candidate->loader = nullptr;
        return;
    }

    // 2. 加载候选动态库并检查是否实现任一 provider 接口
    const ProviderPluginInfo baseInfo = basePluginInfo(candidate->path, *loader);
    // 3. 【插件】【实例寿命】仅真正的 Qt 插件保持驻留，保证借用接口和静态字符串的原有寿命
    if (!loader->metaData().isEmpty()) {
        loader->setLoadHints(loader->loadHints() | QLibrary::PreventUnloadHint);
    }
    QObject *instance = loadProviderPluginWithRuntimeFallback(loader);
    if (!instance) {
        ProviderPluginInfo info = baseInfo;
        info.error = loader->errorString();
        candidate->infos.append(info);
        markshot::debugLog("providers",
                           "【插件】【加载失败】path=%s error=%s",
                           candidate->path.toUtf8().constData(),
                           loader->errorString().toUtf8().constData());
        loader->deleteLater();
        candidate->loader = nullptr;
        return;
    }

    bool matched = false;
    if (auto *ocr = qobject_cast<markshot::plugin::OcrProviderPlugin *>(instance)) {
        matched = true;
        QString error;
        const bool available = ocr->isAvailable(&error);
        candidate->infos.append(matchedPluginInfo(baseInfo,
                                                  ProviderPluginCapability::Ocr,
                                                  ocr->providerId(),
                                                  ocr->displayName(),
                                                  available,
                                                  error));
        markshot::debugLog("providers",
                           "【插件】【OCR】loaded id=%s path=%s",
                           ocr->providerId().toUtf8().constData(),
                           candidate->path.toUtf8().constData());
    }
    if (auto *translate = qobject_cast<markshot::plugin::TranslateProviderPlugin *>(instance)) {
        matched = true;
        QString error;
        const bool available = translate->isAvailable(&error);
        candidate->infos.append(matchedPluginInfo(baseInfo,
                                                  ProviderPluginCapability::Translation,
                                                  translate->providerId(),
                                                  translate->displayName(),
                                                  available,
                                                  error));
        markshot::debugLog("providers",
                           "【插件】【翻译】loaded id=%s path=%s",
                           translate->providerId().toUtf8().constData(),
                           candidate->path.toUtf8().constData());
    }
    if (auto *codeScan = qobject_cast<markshot::plugin::CodeScanProviderPlugin *>(instance)) {
        matched = true;
        QString error;
        const bool available = codeScan->isAvailable(&error);
        candidate->infos.append(matchedPluginInfo(baseInfo,
                                                  ProviderPluginCapability::CodeScan,
                                                  codeScan->providerId(),
                                                  codeScan->displayName(),
                                                  available,
                                                  error));
        markshot::debugLog("providers",
                           "【插件】【扫码】loaded id=%s path=%s",
                           codeScan->providerId().toUtf8().constData(),
                           candidate->path.toUtf8().constData());
    }

    // 3. 元数据没写标识的旧插件，加载后再按运行时标识去重并卸载后到的副本
    if (matched) {
        QStringList runtimeIds;
        for (const ProviderPluginInfo &info : candidate->infos) {
            if (!info.providerId.isEmpty() && !runtimeIds.contains(info.providerId)) {
                runtimeIds.append(info.providerId);
            }
        }
        const bool runtimeDuplicate = !runtimeIds.isEmpty()
            && std::all_of(runtimeIds.cbegin(), runtimeIds.cend(), [this](const QString &id) {
                   return m_loadedProviderIds.contains(id);
               });
        if (runtimeDuplicate) {
            candidate->infos.clear();
            markshot::debugLog("providers",
                               "【插件】【跳过重复】id=%s path=%s",
                               runtimeIds.join(QLatin1Char(',')).toUtf8().constData(),
                               candidate->path.toUtf8().constData());
            loader->unload();
            loader->deleteLater();
            candidate->loader = nullptr;
            return;
        }
        candidate->instance = instance;
        for (const QString &id : declaredIds + runtimeIds) {
            m_loadedProviderIds.insert(id);
        }
        return;
    }
    ProviderPluginInfo info = baseInfo;
    info.loaded = true;
    info.error = QStringLiteral("No supported provider interface");
    candidate->infos.append(info);
    loader->unload();
    loader->deleteLater();
    candidate->loader = nullptr;
}

}  // namespace markshot::providers
