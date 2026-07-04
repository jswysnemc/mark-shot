#pragma once

#include "recording/recording_frame_sample.h"

#include <QObject>
#include <QString>

namespace markshot::recording {

class RecordingCaptureStream : public QObject {
    Q_OBJECT

public:
    /**
     * 创建录制采集流。
     * @param parent 父对象。
     */
    explicit RecordingCaptureStream(QObject *parent = nullptr)
        : QObject(parent)
    {
    }

    /**
     * 销毁录制采集流。
     */
    ~RecordingCaptureStream() override = default;

    /**
     * 启动采集流。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    virtual bool start(QString *error) = 0;

    /**
     * 停止采集流。
     * @return 无返回值。
     */
    virtual void stop() = 0;

    /**
     * 设置写出背压状态。
     * @param active 队列繁忙时为 true。
     * @return 无返回值。
     */
    virtual void setBackpressureActive(bool active) = 0;

signals:
    void frameReady(const markshot::recording::RecordingFrameSample &sample);
    void failed(const QString &error);
};

}  // namespace markshot::recording
