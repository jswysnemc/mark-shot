#include "recording/recording_frame_pacer.h"

#include <algorithm>
#include <limits>

namespace markshot::recording {

RecordingFramePacer::RecordingFramePacer(int fps)
{
    reset(fps);
}

void RecordingFramePacer::reset(int fps)
{
    m_fps = std::max(1, fps);
    m_originMs = 0;
    m_nextFrame = 0;
    m_started = false;
}

RecordingFramePacer::Decision RecordingFramePacer::plan(qint64 timestampMs)
{
    const qint64 timestamp = std::max<qint64>(0, timestampMs);
    if (!m_started) {
        m_started = true;
        m_originMs = timestamp;
        m_nextFrame = 1;
        return {0, 0};
    }
    if (timestamp < m_originMs) {
        return {};
    }

    // 1. 【录制】【帧率对齐】使用绝对有理数时间格，兼容采集端向下取整的毫秒时间戳
    const qint64 elapsed = timestamp - m_originMs;
    const qint64 seconds = elapsed / 1000;
    if (seconds >= std::numeric_limits<qint64>::max() / m_fps - 1) {
        return {};
    }
    const qint64 frame = seconds * m_fps + ((elapsed % 1000 + 1) * m_fps - 1) / 1000;
    if (frame < m_nextFrame) {
        return {};
    }

    // 2. 【录制】【帧率对齐】限制追帧工作量，但保留真实序号，长时间断流不再压缩时间轴
    const auto repeats = std::min({frame - m_nextFrame, m_fps * 10,
                                  static_cast<qint64>(std::numeric_limits<int>::max())});
    m_nextFrame = frame + 1;
    return {frame, static_cast<int>(repeats)};
}

qint64 RecordingFramePacer::delayBeforeNext(qint64 timestampMs) const
{
    if (!m_started) {
        return 0;
    }
    const qint64 nextMs = m_nextFrame / m_fps * 1000 + m_nextFrame % m_fps * 1000 / m_fps;
    const qint64 elapsed = std::max<qint64>(0, timestampMs) - m_originMs;
    return std::max<qint64>(0, nextMs - elapsed);
}

}  // namespace markshot::recording
