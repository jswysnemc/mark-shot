#include "providers/provider_task.h"

#include <QEventLoop>

namespace markshot::providers {

TaskResult ProviderTask::waitForResult()
{
    // 1. 任务可能在 start 内同步结束，此时直接返回缓存结果避免事件循环挂起
    if (m_finishedEmitted) {
        return m_lastResult;
    }

    TaskResult captured;
    QEventLoop loop;
    // 2. 局部事件循环等待 finished，供同步调用点（截图 OCR/扫码）复用
    connect(this, &ProviderTask::finished, &loop, [&captured, &loop](const TaskResult &result) {
        captured = result;
        loop.quit();
    });
    loop.exec();
    return captured;
}

void ProviderTask::emitFinished(TaskResult result)
{
    if (m_finishedEmitted) {
        return;
    }
    m_finishedEmitted = true;
    result.providerName = m_providerName;
    m_lastResult = result;
    emit finished(result);
}

}  // namespace markshot::providers
