#pragma once

#include "providers/provider_task.h"

#include <QProcess>
#include <QStringList>
#include <QTimer>

namespace markshot::providers {

/**
 * 以子进程形态执行的 provider 任务。
 *
 * 同时承载旧 Python helper 与用户自定义 shell 命令两种兼容形态。
 */
class ProviderProcessTask final : public ProviderTask {
    Q_OBJECT

public:
    /**
     * 创建按程序 + 参数执行的任务。
     * @param providerName provider 展示名。
     * @param program 程序路径。
     * @param arguments 程序参数。
     * @param parent 父对象。
     * @return 任务实例。
     */
    static ProviderProcessTask *fromProgram(const QString &providerName,
                                            const QString &program,
                                            const QStringList &arguments,
                                            QObject *parent = nullptr);

    /**
     * 创建按 shell 命令行执行的任务。
     * @param providerName provider 展示名。
     * @param commandLine 完整 shell 命令行。
     * @param parent 父对象。
     * @return 任务实例。
     */
    static ProviderProcessTask *fromShellCommand(const QString &providerName,
                                                 const QString &commandLine,
                                                 QObject *parent = nullptr);

    void start(int timeoutMs) override;
    void cancel() override;

private:
    explicit ProviderProcessTask(QString providerName, QObject *parent);

    /**
     * 处理子进程结束。
     * @param exitCode 退出码。
     * @param exitStatus 退出状态。
     * @return 无返回值。
     */
    void handleFinished(int exitCode, QProcess::ExitStatus exitStatus);

    QProcess m_process;
    QTimer m_timeoutTimer;
    bool m_timedOut = false;
};

}  // namespace markshot::providers
