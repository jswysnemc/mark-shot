#pragma once

#include "providers/provider_task.h"

#include <QFutureWatcher>
#include <QTimer>

namespace markshot::plugin {
class OcrProviderPlugin;
}

namespace markshot::providers {

/**
 * 以外部库插件形态执行的 OCR 任务。
 *
 * 在线程池中调用插件同步 recognize 接口，结果转换为标准 tokens JSON。
 */
class OcrPluginTask final : public ProviderTask {
    Q_OBJECT

public:
    /**
     * 创建插件 OCR 任务。
     * @param plugin OCR 插件实例，生命周期由注册器持有。
     * @param imagePath 输入图像路径。
     * @param parent 父对象。
     */
    OcrPluginTask(markshot::plugin::OcrProviderPlugin *plugin,
                  QString imagePath,
                  QObject *parent = nullptr);

    void start(int timeoutMs) override;
    void cancel() override;

private:
    markshot::plugin::OcrProviderPlugin *m_plugin = nullptr;
    QString m_imagePath;
    QFutureWatcher<TaskResult> m_watcher;
    QTimer m_timeoutTimer;
};

}  // namespace markshot::providers
