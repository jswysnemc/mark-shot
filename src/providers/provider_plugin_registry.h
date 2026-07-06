#pragma once

#include "providers/provider_plugin_info.h"

#include <QStringList>
#include <QVector>

namespace markshot::plugin {
class OcrProviderPlugin;
class TranslateProviderPlugin;
class CodeScanProviderPlugin;
}

namespace markshot::providers {

/**
 * provider 插件注册器。
 *
 * 首次访问时扫描插件搜索目录，按 IID 匹配加载 OCR/翻译/扫码 provider 插件。
 * 加载结果进程内缓存，仅主线程访问。
 */
class ProviderPluginRegistry final {
public:
    /**
     * 获取进程级注册器单例。
     * @return 注册器引用。
     */
    static ProviderPluginRegistry &instance();

    /**
     * 枚举已加载的 OCR provider 插件。
     * @return 插件指针列表，生命周期由注册器持有。
     */
    QVector<markshot::plugin::OcrProviderPlugin *> ocrProviders();

    /**
     * 枚举已加载的翻译 provider 插件。
     * @return 插件指针列表。
     */
    QVector<markshot::plugin::TranslateProviderPlugin *> translateProviders();

    /**
     * 枚举已加载的扫码 provider 插件。
     * @return 插件指针列表。
     */
    QVector<markshot::plugin::CodeScanProviderPlugin *> codeScanProviders();

    /**
     * 枚举插件加载诊断信息。
     * @return 插件诊断条目。
     */
    QVector<ProviderPluginInfo> pluginInfos();

    /**
     * 读取插件搜索目录列表。
     * @return 目录绝对路径列表，供设置页展示。
     */
    static QStringList pluginSearchDirs();

private:
    ProviderPluginRegistry() = default;

    /**
     * 扫描插件目录并加载全部 provider 插件，只执行一次。
     * @return 无返回值。
     */
    void loadOnce();

    bool m_loaded = false;
    QVector<ProviderPluginInfo> m_pluginInfos;
    QVector<markshot::plugin::OcrProviderPlugin *> m_ocrProviders;
    QVector<markshot::plugin::TranslateProviderPlugin *> m_translateProviders;
    QVector<markshot::plugin::CodeScanProviderPlugin *> m_codeScanProviders;
};

}  // namespace markshot::providers
