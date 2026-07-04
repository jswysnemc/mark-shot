#pragma once

#include "recording/recording_frame_sample.h"

#include <QSize>
#include <QString>

namespace markshot::recording {

class LibavGifRecordingProcess final {
public:
    LibavGifRecordingProcess();
    ~LibavGifRecordingProcess();

    /**
     * 启动库内 GIF 编码器。
     * @param outputPath 输出 GIF 路径。
     * @param frameSize 帧尺寸。
     * @param fps 帧率。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(const QString &outputPath, QSize frameSize, int fps, QString *error);

    /**
     * 写入一帧 GIF 图像。
     * @param sample 录制帧样本。
     * @param error 输出错误信息。
     * @return 写入成功时返回 true。
     */
    bool writeFrame(const RecordingFrameSample &sample, QString *error);

    /**
     * 完成 GIF 写出。
     * @param error 输出错误信息。
     * @return 完成成功时返回 true。
     */
    bool finish(QString *error);

    /**
     * 取消并释放 GIF 编码资源。
     * @return 无返回值。
     */
    void cancel();

private:
    class Private;
    Private *d = nullptr;
};

}  // namespace markshot::recording
