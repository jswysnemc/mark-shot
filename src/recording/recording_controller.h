#pragma once

#include "recording/recording_options.h"
#include "recording/recording_status.h"
#include "recording/recording_status_throttler.h"
#include "recording/recording_writer.h"

#include <QObject>
#include <QtGlobal>
#include <memory>

class QImage;

namespace markshot::recording {

class RecordingAsyncWriter;
class RecordingFrameGrabber;
class RecordingController final : public QObject {
    Q_OBJECT

public:
    /**
     * 创建录制控制器。
     * @param parent 父对象。
     */
    explicit RecordingController(QObject *parent = nullptr);

    /**
     * 启动录制会话。
     * @param options 录制配置。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(const RecordingOptions &options, QString *error);

    /**
     * 请求停止当前录制。
     * @return 无返回值。
     */
    void requestStop();

    /**
     * 读取当前录制状态。
     * @return 录制状态。
     */
    RecordingStatus status() const;

signals:
    void statusChanged();
    void finished(bool ok, QString outputPath, QString message);

private:
    /**
     * 处理抓取到的录制帧样本。
     * @param sample 屏幕帧样本。
     * @return 无返回值。
     */
    void handleFrame(const RecordingFrameSample &sample);

    /**
     * 停止录制并保存文件。
     * @return 无返回值。
     */
    void stop();

    /**
     * 失败后取消录制。
     * @param message 错误信息。
     * @return 无返回值。
     */
    void fail(const QString &message);

    /**
     * 处理异步写出完成结果。
     * @param ok 写出是否成功。
     * @param error 错误信息。
     * @return 无返回值。
     */
    void handleWriterFinished(bool ok, const QString &error);

    /**
     * 完成录制停止流程。
     * @param ok 录制文件是否保存成功。
     * @param error 错误信息。
     * @return 无返回值。
     */
    void completeStop(bool ok, const QString &error);

    /**
     * 读取异步写出器。
     * @return 异步写出器指针，不是异步写出器时返回空。
     */
    RecordingAsyncWriter *asyncWriter() const;

    /**
     * 按节流策略发布录制状态。
     * @param force 是否强制发布。
     * @return 无返回值。
     */
    void publishStatus(bool force);

    RecordingOptions m_options;
    std::unique_ptr<RecordingWriter> m_writer;
    RecordingFrameGrabber *m_grabber = nullptr;
    bool m_writerStarted = false;
    bool m_stopping = false;
    bool m_finishEmitted = false;
    int m_frameCount = 0;
    qint64 m_recordedElapsedMs = 0;
    RecordingStatusThrottler m_statusThrottler;
};

}  // namespace markshot::recording
