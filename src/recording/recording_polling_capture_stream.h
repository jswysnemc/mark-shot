#pragma once

#include "recording/recording_capture_stream.h"
#include "recording/recording_options.h"

#include <QElapsedTimer>
#include <QTimer>

namespace markshot::recording {

class RecordingPollingCaptureStream final : public RecordingCaptureStream {
    Q_OBJECT

public:
    /**
     * 创建轮询式录制采集流。
     * @param options 录制配置。
     * @param parent 父对象。
     */
    explicit RecordingPollingCaptureStream(RecordingOptions options, QObject *parent = nullptr);

    /**
     * 启动轮询采集。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(QString *error) override;

    /**
     * 停止轮询采集。
     * @return 无返回值。
     */
    void stop() override;

    /**
     * 设置背压暂停状态。
     * @param active 队列繁忙时为 true。
     * @return 无返回值。
     */
    void setBackpressureActive(bool active) override;

private:
    /**
     * 抓取单帧屏幕图像。
     * @return 无返回值。
     */
    void captureFrame();

    /**
     * 安排下一次抓取。
     * @param delayOverrideMs 指定延迟，负数表示按目标时间计算。
     * @return 无返回值。
     */
    void scheduleNextCapture(int delayOverrideMs = -1);

    /**
     * 推进下一帧目标时间。
     * @return 无返回值。
     */
    void advanceNextCaptureTime();

    /**
     * 按单帧耗时更新自适应限速时间。
     * @param captureMs 单帧抓取耗时。
     * @return 无返回值。
     */
    void updateAdaptivePacing(qint64 captureMs);

    RecordingOptions m_options;
    QTimer m_timer;
    QElapsedTimer m_clock;
    qint64 m_intervalUs = 16666;
    qint64 m_nextCaptureUs = 0;
    qint64 m_sequence = 0;
    bool m_capturing = false;
    bool m_running = false;
    bool m_backpressureActive = false;
};

}  // namespace markshot::recording
