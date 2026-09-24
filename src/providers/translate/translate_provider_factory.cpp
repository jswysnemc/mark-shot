#include "providers/translate/translate_provider_factory.h"

#include "markshot/translate_provider_plugin.h"
#include "providers/provider_plugin_registry.h"
#include "providers/provider_process_task.h"
#include "providers/translate/translate_openai_task.h"
#include "providers/translate/translate_plugin_task.h"

#include <algorithm>

namespace markshot::providers {
namespace {

/**
 * 读取翻译插件在 auto 链中的优先级。
 *
 * 插件注册表按文件系统顺序加载，多个插件同时可用时 auto 的结果会不确定。
 * 这里给出固定次序，openai-compatible 排在最前以保持既有配置的行为不变。
 *
 * @param providerId 插件 provider 标识。
 * @return 优先级序号，数值越小越优先。
 */
int translateProviderRank(const QString &providerId)
{
    static const QStringList order = {QStringLiteral("openai-compatible"),
                                      QStringLiteral("gemini"),
                                      QStringLiteral("anthropic"),
                                      QStringLiteral("tencent-tmt"),
                                      QStringLiteral("baidu-fanyi"),
                                      QStringLiteral("youdao-nmt")};
    const int index = order.indexOf(providerId);
    return index < 0 ? order.size() : index;
}

/**
 * 选择第一个可用的翻译插件。
 * @param preferredId 指定插件 id，空串表示按 auto 优先级挑选。
 * @return 可用插件，找不到时返回空指针。
 */
markshot::plugin::TranslateProviderPlugin *pickTranslatePlugin(const QString &preferredId)
{
    auto plugins = ProviderPluginRegistry::instance().translateProviders();

    // 1. 未指定 id 时按固定优先级排序，避免 auto 结果随加载顺序漂移
    if (preferredId.isEmpty()) {
        std::stable_sort(plugins.begin(),
                         plugins.end(),
                         [](markshot::plugin::TranslateProviderPlugin *left,
                            markshot::plugin::TranslateProviderPlugin *right) {
                             return translateProviderRank(left->providerId())
                                 < translateProviderRank(right->providerId());
                         });
    }

    // 2. 取首个配置完整的插件，凭据缺失的插件自动跳过
    for (markshot::plugin::TranslateProviderPlugin *plugin : plugins) {
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
 * @param request 翻译请求。
 * @param parent 父对象。
 * @return helper 任务。
 */
ProviderTask *createHelperTask(const TranslateTaskRequest &request, QObject *parent)
{
    return ProviderProcessTask::fromProgram(QStringLiteral("helper"),
                                            request.helperProgram,
                                            {QStringLiteral("--input"),
                                             request.inputPath,
                                             QStringLiteral("--target-language"),
                                             request.targetLanguage,
                                             QStringLiteral("--config"),
                                             request.configPath},
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

ProviderTask *createTranslateTask(const TranslateTaskRequest &request, QObject *parent)
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
        return new TranslateOpenAiTask(request.inputJson, request.targetLanguage, request.configPath, parent);
    }
    if (kind == QStringLiteral("plugin")) {
        if (markshot::plugin::TranslateProviderPlugin *plugin = pickTranslatePlugin(pluginId)) {
            return new TranslatePluginTask(plugin, request.inputJson, request.targetLanguage, parent);
        }
        return createHelperTask(request, parent);
    }

    // 3. auto 链：插件 > 内置实现（与 helper 等价的 HTTP 调用）> helper 兜底
    if (markshot::plugin::TranslateProviderPlugin *plugin = pickTranslatePlugin(QString())) {
        return new TranslatePluginTask(plugin, request.inputJson, request.targetLanguage, parent);
    }
    return new TranslateOpenAiTask(request.inputJson, request.targetLanguage, request.configPath, parent);
}

QString resolvedTranslateProviderName(const TranslateTaskRequest &request)
{
    if (!request.commandLine.isEmpty()) {
        return QStringLiteral("custom command");
    }

    QString pluginId;
    const QString kind = normalizedProviderKind(request.provider, &pluginId);
    if (kind == QStringLiteral("helper")) {
        return QStringLiteral("helper (mark-shot-translate)");
    }
    if (kind == QStringLiteral("builtin")) {
        return QStringLiteral("builtin (openai-compatible)");
    }
    if (kind == QStringLiteral("plugin")) {
        if (markshot::plugin::TranslateProviderPlugin *plugin = pickTranslatePlugin(pluginId)) {
            return QStringLiteral("plugin (%1)").arg(plugin->displayName());
        }
        return QStringLiteral("helper (plugin unavailable)");
    }

    if (markshot::plugin::TranslateProviderPlugin *plugin = pickTranslatePlugin(QString())) {
        return QStringLiteral("plugin (%1)").arg(plugin->displayName());
    }
    return QStringLiteral("builtin (openai-compatible)");
}

}  // namespace markshot::providers
