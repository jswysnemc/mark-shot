#include "recording/recording_bgra_buffer_pool.h"

#include <algorithm>
#include <limits>

namespace markshot::recording {

RecordingBgraBufferPool::RecordingBgraBufferPool(quint64 budgetBytes)
    : m_budgetBytes(budgetBytes)
{
}

QByteArray *RecordingBgraBufferPool::tryAcquire(qsizetype byteCount, qint64 nowMs)
{
    if (byteCount <= 0 || byteCount > std::numeric_limits<int>::max()
        || byteCount > std::numeric_limits<qsizetype>::max() / kSlotCount) {
        return nullptr;
    }
    reclaimIdle(nowMs);
    // 1. 【录制】【帧缓冲】只复用未被共享的存储，不解除借出槽位来绕过预算
    for (int offset = 0; offset < kSlotCount; ++offset) {
        const int index = (m_cursor + offset) % kSlotCount;
        if (m_slots[index].isDetached() && m_slots[index].size() == byteCount) {
            m_cursor = index;
            m_idleSince[index] = -1;
            return &m_slots[index];
        }
    }

    // 2. 【录制】【帧缓冲】尺寸改变时立即清除空闲旧布局，借出的旧分辨率帧继续占用预算
    int selected = -1;
    quint64 heldBytes = 0;
    quint64 largestHeld = 0;
    for (int index = 0; index < kSlotCount; ++index) {
        auto &slot = m_slots[index];
        if (slot.isEmpty() || slot.isDetached()) {
            slot = {};
            if (selected < 0) selected = index;
        } else {
            heldBytes += static_cast<quint64>(slot.capacity());
            largestHeld = std::max(largestHeld, static_cast<quint64>(slot.capacity()));
        }
    }
    const quint64 budget = recordingEffectiveCaptureBudget(byteCount, largestHeld, m_budgetBytes);
    if (selected < 0 || heldBytes > budget || static_cast<quint64>(byteCount) > budget - heldBytes) {
        return nullptr;
    }

    // 3. 【录制】【帧缓冲】按请求尺寸分配，不使用 resize 的几何增长策略
    QByteArray buffer(byteCount, Qt::Uninitialized);
    if (static_cast<quint64>(buffer.capacity()) > budget - heldBytes) {
        return nullptr;
    }
    m_slots[selected] = std::move(buffer);
    m_cursor = selected;
    m_idleSince[selected] = -1;
    return &m_slots[selected];
}

void RecordingBgraBufferPool::reclaimIdle(qint64 nowMs)
{
    for (int index = 0; index < kSlotCount; ++index) {
        if (m_slots[index].isEmpty() || !m_slots[index].isDetached()) {
            m_idleSince[index] = -1;
            continue;
        }
        if (m_idleSince[index] < 0) {
            m_idleSince[index] = nowMs;
        } else if (nowMs - m_idleSince[index] >= kRecordingCaptureIdleMs) {
            m_slots[index] = {};
            m_idleSince[index] = -1;
        }
    }
}

void RecordingBgraBufferPool::reset()
{
    m_slots = {};
    m_idleSince.fill(-1);
}

qsizetype RecordingBgraBufferPool::retainedCapacity() const
{
    qsizetype bytes = 0;
    for (const QByteArray &slot : m_slots) bytes += slot.capacity();
    return bytes;
}

}  // namespace markshot::recording
