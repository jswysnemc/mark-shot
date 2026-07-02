#pragma once

#include <QProcess>
#include <QSize>
#include <QString>

class QImage;

namespace markshot::recording {

class FfmpegRecordingProcess final {
public:
    /**
     * 启动 FFmpeg 进程。
     * @param program FFmpeg 可执行文件。
     * @param arguments FFmpeg 参数。
     * @param frameSize 录制帧尺寸。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(const QString &program, const QStringList &arguments, QSize frameSize, QString *error);

    /**
     * 写入一帧 BGRA 原始图像。
     * @param frame 需要写入的图像。
     * @param error 输出错误信息。
     * @return 写入成功时返回 true。
     */
    bool writeFrame(const QImage &frame, QString *error);

    /**
     * 关闭输入并等待 FFmpeg 完成编码。
     * @param error 输出错误信息。
     * @return 编码成功时返回 true。
     */
    bool finish(QString *error);

    /**
     * 终止 FFmpeg 进程。
     * @return 无返回值。
     */
    void cancel();

private:
    /**
     * 返回最近的 FFmpeg 错误输出。
     * @return 错误输出摘要。
     */
    QString stderrTail() const;

    QProcess m_process;
    QByteArray m_stderr;
    QSize m_frameSize;
};

}  // namespace markshot::recording
