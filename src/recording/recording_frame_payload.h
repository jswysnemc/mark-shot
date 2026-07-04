#pragma once

#include <QByteArray>
#include <QSize>

namespace markshot::recording {

struct RecordingRawBgraFrame {
    QByteArray bytes;
    QSize size;
    int stride = 0;

    /**
     * 判断 raw BGRA 帧是否可用于写入编码器。
     * @return 数据完整时返回 true。
     */
    bool isValid() const;
};

}  // namespace markshot::recording
