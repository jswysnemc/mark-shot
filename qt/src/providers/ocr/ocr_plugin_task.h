#pragma once

#include "providers/provider_task.h"

#include <QFutureWatcher>
#include <QTimer>

#include <atomic>
#include <memory>

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

    /** 【OCR】【任务销毁】取消尚未执行的识别，不等待已经开始的同步推理，无参数和返回值。 */
    ~OcrPluginTask() override;

    /** 【OCR】【任务启动】按 timeoutMs 超时限制排队识别；同一任务仅启动一次，无返回值。 */
    void start(int timeoutMs) override;
    /** 【OCR】【任务取消】跳过尚未解码的任务并丢弃进行中结果，无参数和返回值。 */
    void cancel() override;

private:
    markshot::plugin::OcrProviderPlugin *m_plugin = nullptr;
    QString m_imagePath;
    QFutureWatcher<TaskResult> m_watcher;
    QTimer m_timeoutTimer;
    std::shared_ptr<std::atomic_bool> m_cancelled = std::make_shared<std::atomic_bool>(false);
    bool m_started = false;
};

}  // namespace markshot::providers
