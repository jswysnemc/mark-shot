#pragma once

#include "recording/recording_options.h"
#include "recording/recording_status.h"
#include "recording/recording_writer.h"

#include <QElapsedTimer>
#include <QObject>
#include <memory>

class QImage;

namespace markshot::recording {

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
     * 处理抓取到的录制帧。
     * @param frame 屏幕帧。
     * @return 无返回值。
     */
    void handleFrame(const QImage &frame);

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

    RecordingOptions m_options;
    std::unique_ptr<RecordingWriter> m_writer;
    RecordingFrameGrabber *m_grabber = nullptr;
    bool m_writerStarted = false;
    bool m_stopping = false;
    int m_frameCount = 0;
    QElapsedTimer m_elapsed;
};

}  // namespace markshot::recording
