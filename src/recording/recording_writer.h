#pragma once

#include <QImage>
#include <QSize>
#include <QString>

namespace markshot::recording {

class RecordingWriter {
public:
    virtual ~RecordingWriter() = default;

    /**
     * 启动写出器。
     * @param frameSize 录制帧尺寸。
     * @param fps 帧率。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    virtual bool start(QSize frameSize, int fps, QString *error) = 0;

    /**
     * 写入一帧图像。
     * @param frame 需要写入的图像。
     * @param error 输出错误信息。
     * @return 写入成功时返回 true。
     */
    virtual bool writeFrame(const QImage &frame, QString *error) = 0;

    /**
     * 完成写出并关闭文件。
     * @param error 输出错误信息。
     * @return 完成成功时返回 true。
     */
    virtual bool finish(QString *error) = 0;

    /**
     * 取消写出并终止底层进程。
     * @return 无返回值。
     */
    virtual void cancel() = 0;
};

}  // namespace markshot::recording
