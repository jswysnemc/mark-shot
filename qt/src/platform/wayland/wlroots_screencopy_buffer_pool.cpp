#include "platform/wayland/wlroots_screencopy_buffer_pool.h"

#include <algorithm>

namespace markshot::recording {

WlrootsScreencopyBufferPool::WlrootsScreencopyBufferPool(quint64 budgetBytes)
    : m_budgetBytes(budgetBytes)
{
}

WlrootsScreencopyBufferPool::~WlrootsScreencopyBufferPool()
{
    reset();
}

std::shared_ptr<WlrootsScreencopyShmBuffer> WlrootsScreencopyBufferPool::acquire(wl_shm *shm,
                                                                                 std::uint32_t format,
                                                                                 int width,
                                                                                 int height,
                                                                                 int stride,
                                                                                 QString *error, qint64 nowMs)
{
    if (error) error->clear();
    const qsizetype byteCount = stride <= 0 ? 0 : recordingBgraFrameBytes(QSize(width, height), stride);
    if (!shm || !byteCount) {
        if (error) *error = QStringLiteral("invalid wl_shm buffer parameters");
        return nullptr;
    }
    reclaimIdle(nowMs);
    // 1. 【录制】【wlroots缓冲池】复用相同布局，已借出的存储不会重新映射或重复计费
    for (auto &entry : m_buffers) {
        if (entry.buffer.use_count() == 1 && entry.buffer->matches(format, width, height, stride)) {
            entry.idleSince = -1;
            return entry.buffer;
        }
    }

    // 2. 【录制】【wlroots缓冲池】新布局优先释放空闲旧帧，借出的旧像素保留并继续计入容量
    m_buffers.erase(std::remove_if(m_buffers.begin(), m_buffers.end(), [](const Entry &entry) {
        return entry.buffer.use_count() == 1;
    }), m_buffers.end());
    quint64 largestHeld = 0;
    for (const auto &entry : m_buffers) {
        largestHeld = std::max(largestHeld, static_cast<quint64>(entry.buffer->byteSize()));
    }
    const quint64 budget = recordingEffectiveCaptureBudget(byteCount, largestHeld, m_budgetBytes);
    const quint64 heldBytes = retainedBytes();
    if (m_buffers.size() >= kMaxBuffers || heldBytes > budget
        || static_cast<quint64>(byteCount) > budget - heldBytes) {
        if (error) *error = QStringLiteral("wlroots screencopy buffer pool is temporarily full");
        return nullptr;
    }
    auto buffer = std::make_shared<WlrootsScreencopyShmBuffer>();
    if (!buffer->ensure(shm, format, width, height, stride, error)) return nullptr;
    m_buffers.push_back({buffer, -1});
    return buffer;
}

void WlrootsScreencopyBufferPool::reclaimIdle(qint64 nowMs)
{
    // 1. 【录制】【空闲回收】从首次观察到归还开始计时，长时间借出不等于长时间空闲
    for (auto &entry : m_buffers) {
        if (entry.buffer.use_count() != 1) {
            entry.idleSince = -1;
        } else if (entry.idleSince < 0) {
            entry.idleSince = nowMs;
        }
    }
    m_buffers.erase(std::remove_if(m_buffers.begin(), m_buffers.end(), [nowMs](const Entry &entry) {
        return entry.buffer.use_count() == 1 && entry.idleSince >= 0
            && nowMs - entry.idleSince >= kRecordingCaptureIdleMs;
    }), m_buffers.end());
}

quint64 WlrootsScreencopyBufferPool::retainedBytes() const
{
    quint64 bytes = 0;
    for (const auto &entry : m_buffers) bytes += entry.buffer->byteSize();
    return bytes;
}

void WlrootsScreencopyBufferPool::reset()
{
    // 1. 【录制】【映射寿命】先销毁仍属于当前连接的 wl_buffer，避免借出的帧晚于连接释放协议对象
    for (const auto &entry : m_buffers) {
        entry.buffer->releaseWaylandBuffer();
    }
    m_buffers.clear();
}

}  // namespace markshot::recording
