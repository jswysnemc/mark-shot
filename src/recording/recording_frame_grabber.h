#pragma once

#include "recording/recording_capture_stream.h"
#include "recording/recording_frame_sample.h"
#include "recording/recording_options.h"

#include <QObject>

#include <memory>

namespace markshot::recording {

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
     * 连接采集流信号。
     * @return 无返回值。
     */
    void connectCaptureStream();

    RecordingOptions m_options;
    std::unique_ptr<RecordingCaptureStream> m_stream;
    bool m_backpressureActive = false;
};

}  // namespace markshot::recording
