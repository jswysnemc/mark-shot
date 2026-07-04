#include "providers/provider_plugin_registry.h"

#include "debug_log.h"
#include "markshot/code_scan_provider_plugin.h"
#include "markshot/ocr_provider_plugin.h"
#include "markshot/translate_provider_plugin.h"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>
#include <QLibrary>
#include <QPluginLoader>

namespace markshot::providers {
namespace {

/**
 * 追加去重后的插件搜索目录。
 * @param dirs 目录列表。
 * @param path 待追加路径。
 * @return 无返回值。
 */
void addSearchDir(QStringList *dirs, const QString &path)
{
    const QString absolute = QDir(path).absolutePath();
    if (!absolute.isEmpty() && !dirs->contains(absolute)) {
        dirs->append(absolute);
    }
}

}  // namespace

ProviderPluginRegistry &ProviderPluginRegistry::instance()
{
    static ProviderPluginRegistry registry;
    return registry;
}

QStringList ProviderPluginRegistry::pluginSearchDirs()
{
    QStringList dirs;
    const QString appDir = QCoreApplication::applicationDirPath();
    // 1. 与 layer-shell 插件一致的目录约定，外加 providers 子目录
    addSearchDir(&dirs, QDir(appDir).filePath(QStringLiteral("plugins")));
    addSearchDir(&dirs, QDir(appDir).filePath(QStringLiteral("../lib/mark-shot/plugins")));
    addSearchDir(&dirs, QDir(appDir).filePath(QStringLiteral("../lib64/mark-shot/plugins")));
    for (const QString &path : QCoreApplication::libraryPaths()) {
        addSearchDir(&dirs, QDir(path).filePath(QStringLiteral("mark-shot/plugins")));
    }
    // 2. 用户级插件目录，便于免打包尝试插件
    const QString dataHome = qEnvironmentVariable("XDG_DATA_HOME").trimmed();
    const QString userBase = dataHome.isEmpty()
        ? QDir::home().filePath(QStringLiteral(".local/share"))
        : dataHome;
    addSearchDir(&dirs, QDir(userBase).filePath(QStringLiteral("mark-shot/plugins")));
    return dirs;
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
            QObject *instance = loader->instance();
            if (!instance) {
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
                markshot::debugLog("providers",
                                   "【插件】【OCR】loaded id=%s path=%s",
                                   ocr->providerId().toUtf8().constData(),
                                   entry.absoluteFilePath().toUtf8().constData());
            }
            if (auto *translate = qobject_cast<markshot::plugin::TranslateProviderPlugin *>(instance)) {
                m_translateProviders.append(translate);
                matched = true;
                markshot::debugLog("providers",
                                   "【插件】【翻译】loaded id=%s path=%s",
                                   translate->providerId().toUtf8().constData(),
                                   entry.absoluteFilePath().toUtf8().constData());
            }
            if (auto *codeScan = qobject_cast<markshot::plugin::CodeScanProviderPlugin *>(instance)) {
                m_codeScanProviders.append(codeScan);
                matched = true;
                markshot::debugLog("providers",
                                   "【插件】【扫码】loaded id=%s path=%s",
                                   codeScan->providerId().toUtf8().constData(),
                                   entry.absoluteFilePath().toUtf8().constData());
            }

            // 2. 与 provider 无关的插件立即卸载，避免占用内存
            if (!matched) {
                loader->unload();
                loader->deleteLater();
            }
        }
    }
}

}  // namespace markshot::providers
