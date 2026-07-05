#include "providers/code_scan/code_scan_provider_factory.h"

#include "markshot/code_scan_provider_plugin.h"
#include "providers/code_scan/code_scan_tasks.h"
#include "providers/provider_plugin_registry.h"
#include "providers/provider_process_task.h"

namespace markshot::providers {
namespace {

/**
 * 选择第一个可用的扫码插件。
 * @param preferredId 指定插件 id，空串表示任意。
 * @return 可用插件，找不到时返回空指针。
 */
markshot::plugin::CodeScanProviderPlugin *pickCodeScanPlugin(const QString &preferredId)
{
    const auto plugins = ProviderPluginRegistry::instance().codeScanProviders();
    for (markshot::plugin::CodeScanProviderPlugin *plugin : plugins) {
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
 * @param request 扫码请求。
 * @param parent 父对象。
 * @return helper 任务。
 */
ProviderTask *createHelperTask(const CodeScanTaskRequest &request, QObject *parent)
{
    return ProviderProcessTask::fromProgram(QStringLiteral("helper"),
                                            request.helperProgram,
                                            {QStringLiteral("--format"),
                                             QStringLiteral("json"),
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

ProviderTask *createCodeScanTask(const CodeScanTaskRequest &request, QObject *parent)
{
    // 1. 用户自定义命令优先级最高，行为与旧版本完全一致
    if (!request.commandLine.isEmpty()) {
        return ProviderProcessTask::fromShellCommand(QStringLiteral("command"),
                                                     request.commandLine,
                                                     parent);
    }

    QString pluginId;
    const QString kind = normalizedProviderKind(request.provider, &pluginId);

    // 2. 显式指定 provider 时不做回退
    if (kind == QStringLiteral("helper")) {
        return createHelperTask(request, parent);
    }
    if (kind == QStringLiteral("builtin")) {
        return new CodeScanZxingTask(request.imagePath, parent);
    }
    if (kind == QStringLiteral("plugin")) {
        if (markshot::plugin::CodeScanProviderPlugin *plugin = pickCodeScanPlugin(pluginId)) {
            return new CodeScanPluginTask(plugin, request.imagePath, parent);
        }
        return createHelperTask(request, parent);
    }

    // 3. auto 链：插件 > 内置 zxing-cpp > helper 兜底
    if (markshot::plugin::CodeScanProviderPlugin *plugin = pickCodeScanPlugin(QString())) {
        return new CodeScanPluginTask(plugin, request.imagePath, parent);
    }
    if (CodeScanZxingTask::available()) {
        return new CodeScanZxingTask(request.imagePath, parent);
    }
    return createHelperTask(request, parent);
}

QString resolvedCodeScanProviderName(const CodeScanTaskRequest &request)
{
    if (!request.commandLine.isEmpty()) {
        return QStringLiteral("custom command");
    }

    QString pluginId;
    const QString kind = normalizedProviderKind(request.provider, &pluginId);
    if (kind == QStringLiteral("helper")) {
        return QStringLiteral("helper (mark-shot-code-scan)");
    }
    if (kind == QStringLiteral("builtin")) {
        return QStringLiteral("builtin (zxing-cpp)");
    }
    if (kind == QStringLiteral("plugin")) {
        if (markshot::plugin::CodeScanProviderPlugin *plugin = pickCodeScanPlugin(pluginId)) {
            return QStringLiteral("plugin (%1)").arg(plugin->displayName());
        }
        return QStringLiteral("helper (plugin unavailable)");
    }

    if (markshot::plugin::CodeScanProviderPlugin *plugin = pickCodeScanPlugin(QString())) {
        return QStringLiteral("plugin (%1)").arg(plugin->displayName());
    }
    if (CodeScanZxingTask::available()) {
        return QStringLiteral("builtin (zxing-cpp)");
    }
    return QStringLiteral("helper (mark-shot-code-scan)");
}

}  // namespace markshot::providers
