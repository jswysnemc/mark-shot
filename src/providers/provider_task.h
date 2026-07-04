#pragma once

#include <QByteArray>
#include <QObject>
#include <QString>

namespace markshot::providers {

enum class TaskError {
    None,
    StartFailed,
    Timeout,
    Failed,
};

struct TaskResult {
    bool ok = false;
    TaskError error = TaskError::None;
    QByteArray output;
    QByteArray errorOutput;
    QString providerName;
};

/**
 * provider 异步任务抽象。
 *
 * 统一 helper 进程、内置实现与插件三种执行形态；输出沿用
 * {backend, tokens/results, errors} JSON 契约，调用点解析逻辑保持不变。
 */
class ProviderTask : public QObject {
    Q_OBJECT

public:
    /**
     * 创建任务。
     * @param providerName provider 展示名。
     * @param parent 父对象。
     */
    explicit ProviderTask(QString providerName, QObject *parent = nullptr)
        : QObject(parent)
        , m_providerName(std::move(providerName))
    {
    }

    /**
     * 启动任务。
     * @param timeoutMs 超时毫秒数，超时后任务以 Timeout 结束。
     * @return 无返回值。
     */
    virtual void start(int timeoutMs) = 0;

    /**
     * 取消任务，取消后不再发出 finished。
     * @return 无返回值。
     */
    virtual void cancel() = 0;

    /**
     * 阻塞等待任务结束，内部运行局部事件循环。
     * @return 任务结果。
     */
    TaskResult waitForResult();

    /**
     * 读取 provider 展示名。
     * @return provider 名称。
     */
    QString providerName() const { return m_providerName; }

signals:
    /**
     * 任务结束信号。
     * @param result 任务结果。
     */
    void finished(const markshot::providers::TaskResult &result);

protected:
    /**
     * 发出结束信号，保证只发一次。
     * @param result 任务结果。
     * @return 无返回值。
     */
    void emitFinished(TaskResult result);

    QString m_providerName;

private:
    bool m_finishedEmitted = false;
    TaskResult m_lastResult;
};

}  // namespace markshot::providers

Q_DECLARE_METATYPE(markshot::providers::TaskResult)
