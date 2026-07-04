#pragma once

#include "recording/recording_frame_payload.h"

#include <QImage>
#include <QMetaType>
#include <QSize>

namespace markshot::recording {

struct RecordingFrameSample {
    QImage image;
    RecordingRawBgraFrame bgra;
    qint64 timestampMs = 0;
    qint64 sequence = 0;

    /**
     * 读取当前样本的帧尺寸。
     * @return 图像或 raw BGRA 帧尺寸。
     */
    QSize frameSize() const
    {
        return !image.isNull() ? image.size() : bgra.size;
    }

    /**
     * 判断当前样本是否包含可写入的帧数据。
     * @return 包含 QImage 或 raw BGRA 数据时返回 true。
     */
    bool hasFrameData() const
    {
        return !image.isNull() || bgra.isValid();
    }
};

}  // namespace markshot::recording

Q_DECLARE_METATYPE(markshot::recording::RecordingFrameSample)
