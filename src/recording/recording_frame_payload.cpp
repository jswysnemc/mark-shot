#include "recording/recording_frame_payload.h"

#include <cstdlib>

namespace markshot::recording {

/**
 * 【录制】【帧载荷】判断 raw BGRA 帧是否可用于写入编码器。
 * @return 数据完整时返回 true。
 */
bool RecordingRawBgraFrame::isValid() const
{
    if (size.isEmpty() || std::abs(stride) < size.width() * 4) {
        return false;
    }
    const qsizetype minimumBytes = static_cast<qsizetype>(std::abs(stride)) * size.height();
    if (mappedData && mappedSize >= minimumBytes) {
        return true;
    }
    return bytes.size() >= minimumBytes;
}

/**
 * 【录制】【帧载荷】读取 raw BGRA 帧首地址。
 * @return 有效时返回帧数据首地址，否则返回空。
 */
const char *RecordingRawBgraFrame::constData() const
{
    if (mappedData && mappedSize > 0) {
        return mappedData;
    }
    return bytes.constData();
}

/**
 * 【录制】【帧载荷】读取 raw BGRA 帧可读字节数。
 * @return 可读字节数。
 */
qsizetype RecordingRawBgraFrame::byteSize() const
{
    if (mappedData && mappedSize > 0) {
        return mappedSize;
    }
    return bytes.size();
}

}  // namespace markshot::recording
