#include "recording/recording_frame_rate_limiter.h"

#include <algorithm>

namespace markshot::recording {
namespace {

// 允许略早于目标间隔的帧通过，避免采集抖动导致整帧被系统性丢弃
constexpr double kIntervalTolerance = 0.85;

}  // namespace

RecordingFrameRateLimiter::RecordingFrameRateLimiter(int fps)
{
    reset(fps);
}

void RecordingFrameRateLimiter::reset(int fps)
{
    const int safeFps = std::max(1, fps);
    m_intervalMs = static_cast<qint64>(1000.0 / safeFps * kIntervalTolerance);
    m_lastAcceptedMs = 0;
    m_started = false;
}

bool RecordingFrameRateLimiter::shouldWrite(qint64 timestampMs)
{
    const qint64 timestamp = std::max<qint64>(0, timestampMs);
    // 1. 首帧总是写出，作为时间轴起点
    if (!m_started) {
        m_started = true;
        m_lastAcceptedMs = timestamp;
        return true;
    }

    // 2. 时间戳回退时按新起点重新计时
    if (timestamp < m_lastAcceptedMs) {
        m_lastAcceptedMs = timestamp;
        return true;
    }

    if (timestamp - m_lastAcceptedMs < m_intervalMs) {
        return false;
    }
    m_lastAcceptedMs = timestamp;
    return true;
}

}  // namespace markshot::recording
