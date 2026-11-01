#pragma once

#include <QSize>

#include <algorithm>
#include <cstdlib>
#include <limits>

namespace markshot::recording {

constexpr qint64 kRecordingCaptureIdleMs = 2000;

/** 返回单个活动采集池的像素容量预算，默认 96 MiB，可用 MARK_SHOT_RECORDING_CAPTURE_MIB 覆盖。 */
quint64 recordingCaptureMemoryBudget();

/** 返回单调时钟毫秒值，用于缓冲空闲判断，无参数。 */
qint64 recordingCaptureClockMs();

/**
 * 【录制】【采集预算】保留至少两帧容量，防止上一帧被心跳持有时无法捕获下一帧。
 * @param requestedBytes 本次帧所需字节数。
 * @param largestHeldBytes 尚未归还的最大缓冲容量，保证分辨率降低时仍能继续采集。
 * @param configuredBytes 配置预算。
 * @return 当前准入预算；它不是编码器、驱动或全进程内存上限。
 */
quint64 recordingEffectiveCaptureBudget(quint64 requestedBytes, quint64 largestHeldBytes,
                                       quint64 configuredBytes);

/**
 * 【录制】【尺寸校验】验证 BGRA 行跨度及总字节数，拒绝有符号溢出和超出 wl_shm 范围的帧。
 * @param size 帧尺寸。
 * @param stride 行跨度；零表示紧密排列，负数允许倒序行。
 * @return 有效时返回字节数，非法参数返回零。
 */
inline qsizetype recordingBgraFrameBytes(QSize size, int stride = 0)
{
    if (size.isEmpty() || size.width() > std::numeric_limits<int>::max() / 4) {
        return 0;
    }
    const qint64 packed = static_cast<qint64>(size.width()) * 4;
    const qint64 rowBytes = stride == 0 ? packed : std::abs(static_cast<qint64>(stride));
    const qint64 limit = std::min<qint64>(std::numeric_limits<int>::max(),
                                         std::numeric_limits<qsizetype>::max() / 4);
    if (rowBytes < packed || rowBytes > limit / size.height()) {
        return 0;
    }
    return static_cast<qsizetype>(rowBytes * size.height());
}

}  // namespace markshot::recording
