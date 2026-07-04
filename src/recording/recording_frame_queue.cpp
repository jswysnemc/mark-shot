#include "recording/recording_frame_queue.h"

#include <algorithm>

namespace markshot::recording {

RecordingFrameQueue::RecordingFrameQueue(int capacity)
{
    setCapacity(capacity);
}

void RecordingFrameQueue::setCapacity(int capacity)
{
    m_capacity = std::max(1, capacity);
    m_pending = std::min(m_pending, m_capacity);
}

bool RecordingFrameQueue::tryEnqueue()
{
    if (m_pending >= m_capacity) {
        ++m_dropped;
        return false;
    }
    ++m_pending;
    return true;
}

void RecordingFrameQueue::completeOne()
{
    if (m_pending > 0) {
        --m_pending;
    }
}

void RecordingFrameQueue::reset()
{
    m_pending = 0;
    m_dropped = 0;
}

bool RecordingFrameQueue::backpressureActive() const
{
    return m_pending >= m_capacity;
}

int RecordingFrameQueue::droppedFrames() const
{
    return m_dropped;
}

}  // namespace markshot::recording
