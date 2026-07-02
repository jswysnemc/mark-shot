#pragma once

#include "recording/recording_options.h"

#include <QObject>
#include <QTimer>

class QImage;

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

signals:
    void frameReady(const QImage &frame);
    void failed(const QString &error);

private:
    /**
     * 抓取单帧屏幕图像。
     * @return 无返回值。
     */
    void captureFrame();

    RecordingOptions m_options;
    QTimer m_timer;
    bool m_capturing = false;
};

}  // namespace markshot::recording
