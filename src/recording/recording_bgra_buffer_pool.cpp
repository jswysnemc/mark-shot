#include "recording/recording_bgra_buffer_pool.h"

namespace markshot::recording {

QByteArray &RecordingBgraBufferPool::acquire(qsizetype byteCount)
{
    // 1. 从游标开始寻找未被下游引用的槽位，复用其已有内存
    for (int offset = 0; offset < kSlotCount; ++offset) {
        const int index = (m_cursor + offset) % kSlotCount;
        if (!m_slots[index].isDetached()) {
            continue;
        }
        m_cursor = (index + 1) % kSlotCount;
        m_slots[index].resize(byteCount);
        return m_slots[index];
    }

    // 2. 全部槽位仍被引用时退化为新分配，行为等同无池
    const int index = m_cursor;
    m_cursor = (m_cursor + 1) % kSlotCount;
    m_slots[index] = QByteArray();
    m_slots[index].resize(byteCount);
    return m_slots[index];
}

}  // namespace markshot::recording
