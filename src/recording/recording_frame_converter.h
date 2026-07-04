#pragma once

#include <QByteArray>
#include <QImage>
#include <QSize>
#include <QString>

namespace markshot::recording {

struct RecordingBgraFrame {
    const char *data = nullptr;
    qsizetype size = 0;
    int stride = 0;
    bool yInverted = false;
};

class RecordingFrameConverter final {
public:
    /**
     * 把图像转换为 FFmpeg rawvideo 需要的连续 BGRA 字节视图。
     * @param frame 输入图像。
     * @param targetSize 目标帧尺寸。
     * @param error 输出错误信息。
     * @return 可写入 FFmpeg 的 BGRA 字节视图。
     */
    RecordingBgraFrame convertToBgra(const QImage &frame, QSize targetSize, QString *error);

private:
    /**
     * 判断图像格式是否可以按 BGRA 字节顺序直接写入。
     * @param format 图像格式。
     * @return 可以直接写入时返回 true。
     */
    static bool isBgraCompatible(QImage::Format format);

    /**
     * 返回连续 BGRA 字节视图，必要时把按行数据复制到复用缓冲。
     * @param image 输入图像。
     * @param rowBytes 每行有效字节数。
     * @return 连续 BGRA 字节视图。
     */
    RecordingBgraFrame contiguousView(const QImage &image, int rowBytes);

    QImage m_converted;
    QByteArray m_buffer;
};

}  // namespace markshot::recording
