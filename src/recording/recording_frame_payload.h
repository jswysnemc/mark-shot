#pragma once

#include <QByteArray>
#include <QSize>

#include <memory>

namespace markshot::recording {

struct RecordingRawBgraFrame {
    QByteArray bytes;
    std::shared_ptr<const void> mappedOwner;
    const char *mappedData = nullptr;
    qsizetype mappedSize = 0;
    QSize size;
    int stride = 0;
    bool yInverted = false;

    /**
     * 判断 raw BGRA 帧是否可用于写入编码器。
     * @return 数据完整时返回 true。
     */
    bool isValid() const;

    /**
     * 读取 raw BGRA 帧首地址。
     * @return 有效时返回帧数据首地址，否则返回空。
     */
    const char *constData() const;

    /**
     * 读取 raw BGRA 帧可读字节数。
     * @return 可读字节数。
     */
    qsizetype byteSize() const;
};

}  // namespace markshot::recording
