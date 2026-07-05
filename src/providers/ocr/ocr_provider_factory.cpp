#include "providers/ocr/ocr_provider_factory.h"

#include "markshot/ocr_provider_plugin.h"
#include "providers/ocr/ocr_plugin_task.h"
#include "providers/ocr/ocr_tesseract_task.h"
#include "providers/provider_plugin_registry.h"
#include "providers/provider_process_task.h"

#include <QDir>
#include <QFileInfo>

namespace markshot::providers {
namespace {

/**
 * 选择第一个可用的 OCR 插件。
 * @param preferredId 指定插件 id，空串表示任意。
 * @return 可用插件，找不到时返回空指针。
 */
markshot::plugin::OcrProviderPlugin *pickOcrPlugin(const QString &preferredId)
{
    const auto plugins = ProviderPluginRegistry::instance().ocrProviders();
    for (markshot::plugin::OcrProviderPlugin *plugin : plugins) {
        if (!preferredId.isEmpty() && plugin->providerId() != preferredId) {
            continue;
        }
        QString error;
        if (plugin->isAvailable(&error)) {
            return plugin;
        }
    }
    return nullptr;
}

/**
 * 创建 helper 子进程任务。
 * @param request OCR 请求。
 * @param parent 父对象。
 * @return helper 任务。
 */
ProviderTask *createHelperTask(const OcrTaskRequest &request, QObject *parent)
{
    return ProviderProcessTask::fromProgram(QStringLiteral("helper"),
                                            request.helperProgram,
                                            {QStringLiteral("--format"),
                                             QStringLiteral("json"),
                                             QStringLiteral("--backend"),
                                             request.backend,
                                             request.imagePath},
                                            parent);
}

/**
 * 解析 provider 偏好，返回归一化类别与插件 id。
 * @param provider 配置值。
 * @param pluginId 输出 plugin:<id> 中的 id。
 * @return 类别：auto/plugin/builtin/helper。
 */
QString normalizedProviderKind(const QString &provider, QString *pluginId)
{
    const QString trimmed = provider.trimmed().toLower();
    if (trimmed.startsWith(QStringLiteral("plugin:"))) {
        *pluginId = trimmed.mid(7).trimmed();
        return QStringLiteral("plugin");
    }
    if (trimmed == QStringLiteral("plugin") || trimmed == QStringLiteral("builtin")
        || trimmed == QStringLiteral("helper")) {
        return trimmed;
    }
    return QStringLiteral("auto");
}

}  // namespace

bool legacyOcrHelperConfigured()
{
    // 1. 与 helper 脚本一致的 venv 位置判断，保证旧用户升级后行为不变
    const QString preferred = qEnvironmentVariable("MARK_SHOT_OCR_PYTHON").trimmed();
    if (!preferred.isEmpty()) {
        return QFileInfo::exists(preferred);
    }
    const QString dataHome = qEnvironmentVariable("XDG_DATA_HOME").trimmed();
    const QString base = dataHome.isEmpty()
        ? QDir::home().filePath(QStringLiteral(".local/share"))
        : dataHome;
    return QFileInfo::exists(QDir(base).filePath(QStringLiteral("mark-shot/ocr-venv/bin/python")));
}

ProviderTask *createOcrTask(const OcrTaskRequest &request, QObject *parent)
{
    // 1. 用户自定义命令优先级最高，行为与旧版本完全一致
    if (!request.commandLine.isEmpty()) {
        return ProviderProcessTask::fromShellCommand(QStringLiteral("command"),
                                                     request.commandLine,
                                                     parent);
    }

    QString pluginId;
    const QString kind = normalizedProviderKind(request.provider, &pluginId);

    // 2. 显式指定 provider 时不做回退，用户意图明确
    if (kind == QStringLiteral("helper")) {
        return createHelperTask(request, parent);
    }
    if (kind == QStringLiteral("builtin")) {
        return new OcrTesseractTask(request.imagePath, parent);
    }
    if (kind == QStringLiteral("plugin")) {
        if (markshot::plugin::OcrProviderPlugin *plugin = pickOcrPlugin(pluginId)) {
            return new OcrPluginTask(plugin, request.imagePath, parent);
        }
        return createHelperTask(request, parent);
    }

    // 3. auto 链：旧 venv 用户保持 helper，其余优先插件再内置，最后 helper 兜底
    if (legacyOcrHelperConfigured()) {
        return createHelperTask(request, parent);
    }
    if (markshot::plugin::OcrProviderPlugin *plugin = pickOcrPlugin(QString())) {
        return new OcrPluginTask(plugin, request.imagePath, parent);
    }
    if (OcrTesseractTask::available()) {
        return new OcrTesseractTask(request.imagePath, parent);
    }
    return createHelperTask(request, parent);
}

QString resolvedOcrProviderName(const OcrTaskRequest &request)
{
    if (!request.commandLine.isEmpty()) {
        return QStringLiteral("custom command");
    }

    QString pluginId;
    const QString kind = normalizedProviderKind(request.provider, &pluginId);
    if (kind == QStringLiteral("helper")) {
        return QStringLiteral("helper (mark-shot-ocr)");
    }
    if (kind == QStringLiteral("builtin")) {
        return QStringLiteral("builtin (tesseract)");
    }
    if (kind == QStringLiteral("plugin")) {
        if (markshot::plugin::OcrProviderPlugin *plugin = pickOcrPlugin(pluginId)) {
            return QStringLiteral("plugin (%1)").arg(plugin->displayName());
        }
        return QStringLiteral("helper (plugin unavailable)");
    }

    if (legacyOcrHelperConfigured()) {
        return QStringLiteral("helper (legacy venv)");
    }
    if (markshot::plugin::OcrProviderPlugin *plugin = pickOcrPlugin(QString())) {
        return QStringLiteral("plugin (%1)").arg(plugin->displayName());
    }
    if (OcrTesseractTask::available()) {
        return QStringLiteral("builtin (tesseract)");
    }
    return QStringLiteral("helper (mark-shot-ocr)");
}

}  // namespace markshot::providers
