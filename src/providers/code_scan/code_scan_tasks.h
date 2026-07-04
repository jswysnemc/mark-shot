#pragma once

#include "providers/provider_task.h"

#include <QFutureWatcher>
#include <QTimer>

namespace markshot::plugin {
class CodeScanProviderPlugin;
}

namespace markshot::providers {

/**
 * 内置 zxing-cpp 扫码任务。
 *
 * 直接链接 zxing-cpp 库识别二维码与条形码，不依赖 Python 环境。
 * 未启用 HAVE_ZXING_CPP 编译时 available 返回 false。
 */
class CodeScanZxingTask final : public ProviderTask {
    Q_OBJECT

public:
    /**
     * 创建内置扫码任务。
     * @param imagePath 输入图像路径。
     * @param parent 父对象。
     */
    explicit CodeScanZxingTask(QString imagePath, QObject *parent = nullptr);

    void start(int timeoutMs) override;
    void cancel() override;

    /**
     * 判断内置 zxing-cpp 是否编译可用。
     * @return 构建启用 zxing-cpp 时返回 true。
     */
    static bool available();

private:
    QString m_imagePath;
    QFutureWatcher<TaskResult> m_watcher;
    QTimer m_timeoutTimer;
};

/**
 * 以外部库插件形态执行的扫码任务。
 */
class CodeScanPluginTask final : public ProviderTask {
    Q_OBJECT

public:
    /**
     * 创建插件扫码任务。
     * @param plugin 扫码插件实例，生命周期由注册器持有。
     * @param imagePath 输入图像路径。
     * @param parent 父对象。
     */
    CodeScanPluginTask(markshot::plugin::CodeScanProviderPlugin *plugin,
                       QString imagePath,
                       QObject *parent = nullptr);

    void start(int timeoutMs) override;
    void cancel() override;

private:
    markshot::plugin::CodeScanProviderPlugin *m_plugin = nullptr;
    QString m_imagePath;
    QFutureWatcher<TaskResult> m_watcher;
    QTimer m_timeoutTimer;
};

}  // namespace markshot::providers
