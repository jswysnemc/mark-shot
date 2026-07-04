#include "recording/recording_status_throttler.h"

#include <algorithm>

namespace markshot::recording {

RecordingStatusThrottler::RecordingStatusThrottler(int intervalMs)
    : m_intervalMs(std::max(1, intervalMs))
{
}

void RecordingStatusThrottler::reset()
{
    m_elapsed.invalidate();
}

bool RecordingStatusThrottler::shouldPublish(bool force)
{
    if (force || !m_elapsed.isValid() || m_elapsed.elapsed() >= m_intervalMs) {
        m_elapsed.restart();
        return true;
    }
    return false;
}

}  // namespace markshot::recording
