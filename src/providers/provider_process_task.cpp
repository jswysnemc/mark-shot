#include "providers/provider_process_task.h"

#include "shell_command.h"

namespace markshot::providers {

ProviderProcessTask::ProviderProcessTask(QString providerName, QObject *parent)
    : ProviderTask(std::move(providerName), parent)
{
    m_timeoutTimer.setSingleShot(true);
    connect(&m_timeoutTimer, &QTimer::timeout, this, [this] {
        // 超时先记录状态再杀进程，结束回调按 Timeout 上报
        m_timedOut = true;
        if (m_process.state() != QProcess::NotRunning) {
            m_process.kill();
        }
    });
    connect(&m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart) {
            emitFinished({false, TaskError::StartFailed, {}, m_process.readAllStandardError(), {}});
        }
    });
    connect(&m_process,
            QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
            this,
            &ProviderProcessTask::handleFinished);
}

ProviderProcessTask *ProviderProcessTask::fromProgram(const QString &providerName,
                                                      const QString &program,
                                                      const QStringList &arguments,
                                                      QObject *parent)
{
    auto *task = new ProviderProcessTask(providerName, parent);
    task->m_process.setProgram(program);
    task->m_process.setArguments(arguments);
    return task;
}

ProviderProcessTask *ProviderProcessTask::fromShellCommand(const QString &providerName,
                                                           const QString &commandLine,
                                                           QObject *parent)
{
    auto *task = new ProviderProcessTask(providerName, parent);
    markshot::setShellCommand(&task->m_process, commandLine);
    return task;
}

void ProviderProcessTask::start(int timeoutMs)
{
    if (timeoutMs > 0) {
        m_timeoutTimer.start(timeoutMs);
    }
    m_process.start();
}

void ProviderProcessTask::cancel()
{
    m_timeoutTimer.stop();
    disconnect(&m_process, nullptr, this, nullptr);
    if (m_process.state() != QProcess::NotRunning) {
        m_process.kill();
        m_process.waitForFinished(1000);
    }
}

void ProviderProcessTask::handleFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    m_timeoutTimer.stop();
    const QByteArray output = m_process.readAllStandardOutput();
    const QByteArray errorOutput = m_process.readAllStandardError();
    if (m_timedOut) {
        emitFinished({false, TaskError::Timeout, output, errorOutput, {}});
        return;
    }
    const bool ok = exitStatus == QProcess::NormalExit && exitCode == 0;
    emitFinished({ok, ok ? TaskError::None : TaskError::Failed, output, errorOutput, {}});
}

}  // namespace markshot::providers
