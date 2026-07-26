#include "recording/recording_pause_state.h"

#include <algorithm>

namespace markshot::recording {

bool RecordingPauseState::pause()
{
    if (m_paused) {
        return false;
    }
    m_paused = true;
    m_pauseClock.restart();
    return true;
}

bool RecordingPauseState::resume()
{
    if (!m_paused) {
        return false;
    }
    m_paused = false;
    if (m_pauseClock.isValid()) {
        m_pausedTotalMs += m_pauseClock.elapsed();
    }
    m_pauseClock.invalidate();
    return true;
}

void RecordingPauseState::reset()
{
    m_pauseClock.invalidate();
    m_baseTimestampMs = 0;
    m_pausedTotalMs = 0;
    m_started = false;
    m_paused = false;
}

qint64 RecordingPauseState::pausedTotalMs() const
{
    // 正在暂停时把当前这段也计入，便于界面实时显示已录时长
    if (m_paused && m_pauseClock.isValid()) {
        return m_pausedTotalMs + m_pauseClock.elapsed();
    }
    return m_pausedTotalMs;
}

qint64 RecordingPauseState::timelineFor(qint64 timestampMs)
{
    // 1. 首帧确立时间轴起点
    if (!m_started) {
        m_started = true;
        m_baseTimestampMs = timestampMs;
    }

    // 2. 输出时间轴不含起点偏移与累计暂停时长
    return std::max<qint64>(0, timestampMs - m_baseTimestampMs - m_pausedTotalMs);
}

}  // namespace markshot::recording
