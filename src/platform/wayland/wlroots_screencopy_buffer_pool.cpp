#include "platform/wayland/wlroots_screencopy_buffer_pool.h"

#include <algorithm>
#include <chrono>
#include <thread>

namespace markshot::recording {

std::shared_ptr<WlrootsScreencopyShmBuffer> WlrootsScreencopyBufferPool::acquire(wl_shm *shm,
                                                                                 std::uint32_t format,
                                                                                 int width,
                                                                                 int height,
                                                                                 int stride,
                                                                                 QString *error)
{
    if (error) {
        error->clear();
    }

    // 1. 【录制】【wlroots缓冲池】优先复用没有被编码队列持有的缓冲
    for (const std::shared_ptr<WlrootsScreencopyShmBuffer> &buffer : m_buffers) {
        if (buffer.use_count() != 1) {
            continue;
        }
        if (!buffer->ensure(shm, format, width, height, stride, error)) {
            return nullptr;
        }
        return buffer;
    }

    // 2. 【录制】【wlroots缓冲池】编码侧短暂持有旧帧时扩容，最多保持 16 个缓冲
    if (static_cast<int>(m_buffers.size()) >= kMaxBuffers) {
        if (error) {
            *error = QStringLiteral("wlroots screencopy buffer pool is temporarily full");
        }
        return nullptr;
    }

    auto buffer = std::make_shared<WlrootsScreencopyShmBuffer>();
    if (!buffer->ensure(shm, format, width, height, stride, error)) {
        return nullptr;
    }
    m_buffers.push_back(buffer);
    return buffer;
}

void WlrootsScreencopyBufferPool::reset()
{
    m_buffers.clear();
}

bool WlrootsScreencopyBufferPool::waitUntilIdle(int timeoutMs) const
{
    const auto deadline = std::chrono::steady_clock::now()
        + std::chrono::milliseconds(std::max(0, timeoutMs));
    while (true) {
        bool idle = true;
        for (const std::shared_ptr<WlrootsScreencopyShmBuffer> &buffer : m_buffers) {
            if (buffer.use_count() != 1) {
                idle = false;
                break;
            }
        }
        if (idle) {
            return true;
        }
        if (std::chrono::steady_clock::now() >= deadline) {
            return false;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(2));
    }
}

}  // namespace markshot::recording
