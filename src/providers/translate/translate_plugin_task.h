#pragma once

#include "providers/provider_task.h"

#include <QFutureWatcher>
#include <QTimer>

namespace markshot::plugin {
class TranslateProviderPlugin;
}

namespace markshot::providers {

/**
 * 以外部库插件形态执行的翻译任务。
 */
class TranslatePluginTask final : public ProviderTask {
    Q_OBJECT

public:
    /**
     * 创建插件翻译任务。
     * @param plugin 翻译插件实例，生命周期由注册器持有。
     * @param inputJson {tokens, targetLanguage} 输入。
     * @param targetLanguage 目标语言，空串时取输入 JSON 内的值。
     * @param parent 父对象。
     */
    TranslatePluginTask(markshot::plugin::TranslateProviderPlugin *plugin,
                        QByteArray inputJson,
                        QString targetLanguage,
                        QObject *parent = nullptr);

    void start(int timeoutMs) override;
    void cancel() override;

private:
    markshot::plugin::TranslateProviderPlugin *m_plugin = nullptr;
    QByteArray m_inputJson;
    QString m_targetLanguage;
    QFutureWatcher<TaskResult> m_watcher;
    QTimer m_timeoutTimer;
};

}  // namespace markshot::providers
