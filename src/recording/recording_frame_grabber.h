#pragma once

#include "recording/recording_capture_stream.h"
#include "recording/recording_frame_sample.h"
#include "recording/recording_options.h"

#include <QObject>
#include <QVector>

#include <functional>
#include <memory>

namespace markshot::recording {

using RecordingCaptureStreamFactory =
    std::function<std::unique_ptr<RecordingCaptureStream>(RecordingCaptureBackend,
                                                          const RecordingOptions &,
                                                          QObject *)>;

class RecordingFrameGrabber final : public QObject {
    Q_OBJECT

public:
    /**
     * 创建录制帧抓取器。
     * @param options 录制配置。
     * @param parent 父对象。
     */
    explicit RecordingFrameGrabber(RecordingOptions options, QObject *parent = nullptr);

    /**
     * 使用指定后端工厂创建录制帧抓取器。
     * @param options 录制配置。
     * @param streamFactory 采集流工厂。
     * @param parent 父对象。
     */
    RecordingFrameGrabber(RecordingOptions options,
                          RecordingCaptureStreamFactory streamFactory,
                          QObject *parent = nullptr);

    /**
     * 开始按帧率抓取屏幕帧。
     * @return 无返回值。
     */
    void start();

    /**
     * 停止抓取屏幕帧。
     * @return 无返回值。
     */
    void stop();

    /**
     * 设置背压暂停状态。
     * @param active 队列繁忙时为 true。
     * @return 无返回值。
     */
    void setBackpressureActive(bool active);

signals:
    void frameReady(const RecordingFrameSample &sample);
    void failed(const QString &error);

private:
    /**
     * 创建并启动采集流。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool startCaptureStream(QString *error);

    /**
     * 按后端顺序创建并启动下一个采集流。
     * @param error 输出最后一次启动错误。
     * @return 启动成功时返回 true。
     */
    bool startNextCaptureStream(QString *error);

    /**
     * 连接指定采集流信号。
     * @param stream 当前采集流。
     * @return 无返回值。
     */
    void connectCaptureStream(RecordingCaptureStream *stream);

    /**
     * 处理当前采集流产生的帧。
     * @param stream 发送信号的采集流。
     * @param sample 录制帧。
     * @return 无返回值。
     */
    void handleFrameReady(RecordingCaptureStream *stream,
                          const RecordingFrameSample &sample);

    /**
     * 处理当前采集流的异步错误。
     * @param stream 发送信号的采集流。
     * @param error 错误信息。
     * @return 无返回值。
     */
    void handleCaptureFailure(RecordingCaptureStream *stream, const QString &error);

    /**
     * 在失败信号栈退出后销毁当前采集流并尝试剩余后端。
     * @param stream 发生错误的采集流。
     * @param error 原始错误信息。
     * @return 无返回值。
     */
    void continueCaptureFallback(RecordingCaptureStream *stream, const QString &error);

    RecordingOptions m_options;
    RecordingCaptureStreamFactory m_streamFactory;
    QVector<RecordingCaptureBackend> m_captureBackends;
    std::unique_ptr<RecordingCaptureStream> m_stream;
    RecordingCaptureBackend m_activeBackend = RecordingCaptureBackend::Auto;
    int m_nextBackendIndex = 0;
    bool m_backpressureActive = false;
    bool m_running = false;
    bool m_receivedFirstFrame = false;
    bool m_fallbackPending = false;
};

}  // namespace markshot::recording
