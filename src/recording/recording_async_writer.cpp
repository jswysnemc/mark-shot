#include "recording/recording_async_writer.h"

#include "debug_log.h"
#include "recording/gif_recording_writer.h"
#include "recording/video_recording_writer.h"

#include <QImage>
#include <QPointer>

#include <algorithm>
#include <memory>

namespace markshot::recording {
namespace {

/**
 * 按录制模式创建同步写出器。
 * @param options 录制配置。
 * @return 同步写出器实例。
 */
std::unique_ptr<RecordingWriter> createSynchronousWriter(const RecordingOptions &options)
{
    if (options.mode == RecordingMode::Gif) {
        return std::make_unique<GifRecordingWriter>(options);
    }
    return std::make_unique<VideoRecordingWriter>(options);
}

/**
 * 按帧率计算待写帧上限。
 * @param fps 录制帧率。
 * @return 队列容量。
 */
int queueCapacityForFps(int fps)
{
    return fps >= 48 ? 1 : 2;
}

}  // namespace

class RecordingAsyncWriterWorker final : public QObject {
public:
    /**
     * 创建异步写出工作对象。
     * @param options 录制配置。
     */
    explicit RecordingAsyncWriterWorker(RecordingOptions options)
        : m_options(std::move(options))
    {
    }

    /**
     * 启动底层同步写出器。
     * @param frameSize 录制帧尺寸。
     * @param fps 帧率。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool startWriter(QSize frameSize, int fps, QString *error)
    {
        m_writer = createSynchronousWriter(m_options);
        return m_writer->start(frameSize, fps, error);
    }

    /**
     * 写入单帧样本。
     * @param sample 需要写入的帧样本。
     * @param error 输出错误信息。
     * @return 写入成功时返回 true。
     */
    bool writeFrame(const RecordingFrameSample &sample, QString *error)
    {
        return m_writer && m_writer->writeFrame(sample, error);
    }

    /**
     * 完成写出并关闭底层资源。
     * @param error 输出错误信息。
     * @return 完成成功时返回 true。
     */
    bool finishWriter(QString *error)
    {
        return m_writer && m_writer->finish(error);
    }

    /**
     * 取消写出。
     * @return 无返回值。
     */
    void cancelWriter()
    {
        if (m_writer) {
            m_writer->cancel();
        }
    }

private:
    RecordingOptions m_options;
    std::unique_ptr<RecordingWriter> m_writer;
};

RecordingAsyncWriter::RecordingAsyncWriter(RecordingOptions options, QObject *parent)
    : QObject(parent)
    , m_options(std::move(options))
{
    m_worker = new RecordingAsyncWriterWorker(m_options);
    m_worker->moveToThread(&m_thread);
    connect(&m_thread, &QThread::finished, m_worker, &QObject::deleteLater);
    m_thread.start();
}

RecordingAsyncWriter::~RecordingAsyncWriter()
{
    cancel();
}

bool RecordingAsyncWriter::start(QSize frameSize, int fps, QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_thread.isRunning() || !m_worker) {
        if (error) {
            *error = QStringLiteral("recording writer thread is not running");
        }
        return false;
    }

    QString workerError;
    bool ok = false;
    QMetaObject::invokeMethod(
        m_worker,
        [this, frameSize, fps, &workerError, &ok] {
            ok = m_worker->startWriter(frameSize, fps, &workerError);
        },
        Qt::BlockingQueuedConnection);
    if (!ok) {
        if (error) {
            *error = workerError;
        }
        stopThread();
        return false;
    }

    m_queue.reset();
    m_queue.setCapacity(queueCapacityForFps(fps));
    m_started = true;
    m_failed = false;
    m_finishing = false;
    m_error.clear();
    publishBackpressure();
    return true;
}

bool RecordingAsyncWriter::writeFrame(const RecordingFrameSample &sample, QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_started || m_finishing || m_failed) {
        if (error) {
            *error = m_error.isEmpty()
                ? QStringLiteral("recording writer is not accepting frames")
                : m_error;
        }
        return false;
    }

    if (!m_queue.tryEnqueue()) {
        publishBackpressure();
        return true;
    }
    publishBackpressure();

    QPointer<RecordingAsyncWriter> self(this);
    RecordingAsyncWriterWorker *worker = m_worker;
    const RecordingFrameSample queuedSample = sample;
    QMetaObject::invokeMethod(
        worker,
        [worker, self, queuedSample] {
            QString writeError;
            const bool ok = worker->writeFrame(queuedSample, &writeError);
            if (self) {
                QMetaObject::invokeMethod(
                    self,
                    [self, ok, writeError] {
                        if (self) {
                            self->handleWriteComplete(ok, writeError);
                        }
                    },
                    Qt::QueuedConnection);
            }
        },
        Qt::QueuedConnection);
    return true;
}

bool RecordingAsyncWriter::finish(QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_started || m_finishing) {
        return true;
    }

    m_finishing = true;
    QPointer<RecordingAsyncWriter> self(this);
    RecordingAsyncWriterWorker *worker = m_worker;
    QMetaObject::invokeMethod(
        worker,
        [worker, self] {
            QString finishError;
            const bool ok = worker->finishWriter(&finishError);
            if (self) {
                QMetaObject::invokeMethod(
                    self,
                    [self, ok, finishError] {
                        if (self) {
                            self->handleFinishComplete(ok, finishError);
                        }
                    },
                    Qt::QueuedConnection);
            }
        },
        Qt::QueuedConnection);
    return true;
}

void RecordingAsyncWriter::cancel()
{
    if (m_thread.isRunning() && m_worker) {
        QMetaObject::invokeMethod(
            m_worker,
            [this] {
                m_worker->cancelWriter();
            },
            Qt::BlockingQueuedConnection);
    }
    stopThread();
    m_started = false;
}

void RecordingAsyncWriter::handleWriteComplete(bool ok, const QString &error)
{
    m_queue.completeOne();
    publishBackpressure();
    if (ok || m_failed || m_finishing) {
        return;
    }

    m_failed = true;
    m_error = error.isEmpty()
        ? QStringLiteral("recording writer failed")
        : error;
    emit failed(m_error);
}

void RecordingAsyncWriter::handleFinishComplete(bool ok, const QString &error)
{
    const int droppedFrames = m_queue.droppedFrames();
    m_queue.reset();
    publishBackpressure();
    m_started = false;
    m_finishing = false;
    stopThread();

    if (droppedFrames > 0) {
        markshot::debugLog("recording",
                           "【录制】【丢帧】dropped=%d",
                           droppedFrames);
    }
    emit finished(ok, error);
}

void RecordingAsyncWriter::publishBackpressure()
{
    const bool active = m_queue.backpressureActive();
    if (active == m_backpressureActive) {
        return;
    }
    m_backpressureActive = active;
    emit backpressureChanged(active);
}

void RecordingAsyncWriter::stopThread()
{
    if (!m_thread.isRunning()) {
        return;
    }
    m_thread.quit();
    m_thread.wait();
}

}  // namespace markshot::recording
