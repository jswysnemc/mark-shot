#pragma once

#include "recording/recording_frame_queue.h"
#include "recording/recording_options.h"
#include "recording/recording_writer.h"

#include <QObject>
#include <QThread>

namespace markshot::recording {

class RecordingAsyncWriterWorker;

class RecordingAsyncWriter final : public QObject, public RecordingWriter {
    Q_OBJECT

public:
    /**
     * 创建异步录制写出器。
     * @param options 录制配置。
     * @param parent 父对象。
     */
    explicit RecordingAsyncWriter(RecordingOptions options, QObject *parent = nullptr);
    ~RecordingAsyncWriter() override;

    /**
     * 在工作线程启动底层写出器。
     * @param frameSize 录制帧尺寸。
     * @param fps 帧率。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(QSize frameSize, int fps, QString *error) override;

    /**
     * 异步提交一帧样本，队列满时丢弃该帧。
     * @param sample 需要写入的帧样本。
     * @param error 输出错误信息。
     * @return 提交成功或丢帧成功时返回 true，写出器已失败时返回 false。
     */
    bool writeFrame(const RecordingFrameSample &sample, QString *error) override;

    /**
     * 异步完成写出。
     * @param error 输出错误信息。
     * @return 成功提交完成请求时返回 true。
     */
    bool finish(QString *error) override;

    /**
     * 取消写出并停止工作线程。
     * @return 无返回值。
     */
    void cancel() override;

signals:
    void failed(QString error);
    void finished(bool ok, QString error);
    void backpressureChanged(bool active);

private:
    /**
     * 处理工作线程写帧结果。
     * @param ok 写帧是否成功。
     * @param error 错误信息。
     * @return 无返回值。
     */
    void handleWriteComplete(bool ok, const QString &error);

    /**
     * 处理工作线程完成结果。
     * @param ok 完成是否成功。
     * @param error 错误信息。
     * @return 无返回值。
     */
    void handleFinishComplete(bool ok, const QString &error);

    /**
     * 发布背压状态变化。
     * @return 无返回值。
     */
    void publishBackpressure();

    /**
     * 停止工作线程。
     * @return 无返回值。
     */
    void stopThread();

    RecordingOptions m_options;
    QThread m_thread;
    RecordingAsyncWriterWorker *m_worker = nullptr;
    RecordingFrameQueue m_queue;
    bool m_started = false;
    bool m_finishing = false;
    bool m_failed = false;
    bool m_backpressureActive = false;
    QString m_error;
};

}  // namespace markshot::recording
