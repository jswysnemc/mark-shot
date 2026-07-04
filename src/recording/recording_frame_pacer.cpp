#include "recording/recording_frame_pacer.h"

#include <algorithm>

namespace markshot::recording {

RecordingFramePacer::RecordingFramePacer(int fps)
{
    reset(fps);
}

void RecordingFramePacer::reset(int fps)
{
    const int safeFps = std::max(1, fps);
    m_intervalUs = std::max<qint64>(1, 1000000 / safeFps);
    m_maxCatchUpFrames = std::max(1, safeFps);
    m_nextFrameUs = 0;
    m_started = false;
}

int RecordingFramePacer::duplicatesBefore(qint64 timestampMs)
{
    const qint64 timestampUs = std::max<qint64>(0, timestampMs) * 1000;
    if (!m_started) {
        m_started = true;
        m_nextFrameUs = timestampUs + m_intervalUs;
        return 0;
    }

    int duplicates = 0;
    const qint64 halfIntervalUs = m_intervalUs / 2;
    while (m_nextFrameUs + halfIntervalUs < timestampUs
           && duplicates < m_maxCatchUpFrames) {
        ++duplicates;
        m_nextFrameUs += m_intervalUs;
    }

    m_nextFrameUs = std::max(m_nextFrameUs + m_intervalUs,
                             timestampUs + m_intervalUs);
    return duplicates;
}

}  // namespace markshot::recording
