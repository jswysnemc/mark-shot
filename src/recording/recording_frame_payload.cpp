#include "recording/recording_frame_payload.h"

namespace markshot::recording {

/**
 * 【录制】【帧载荷】判断 raw BGRA 帧是否可用于写入编码器。
 * @return 数据完整时返回 true。
 */
bool RecordingRawBgraFrame::isValid() const
{
    if (size.isEmpty() || stride < size.width() * 4) {
        return false;
    }
    const qsizetype minimumBytes = static_cast<qsizetype>(stride) * size.height();
    return bytes.size() >= minimumBytes;
}

}  // namespace markshot::recording
