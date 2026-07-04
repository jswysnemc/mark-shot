#pragma once

#include "platform/wayland/wlroots_screencopy_shm_buffer.h"

#include <QString>

#include <cstdint>
#include <memory>
#include <vector>

struct wl_shm;

namespace markshot::recording {

class WlrootsScreencopyBufferPool final {
public:
    /**
     * 获取满足当前帧参数的共享内存缓冲。
     * @param shm Wayland 共享内存接口。
     * @param format wl_shm 像素格式。
     * @param width 帧宽度。
     * @param height 帧高度。
     * @param stride 行跨度。
     * @param error 输出错误信息。
     * @return 可写入的共享内存缓冲，暂时无可用缓冲时返回空。
     */
    std::shared_ptr<WlrootsScreencopyShmBuffer> acquire(wl_shm *shm,
                                                        std::uint32_t format,
                                                        int width,
                                                        int height,
                                                        int stride,
                                                        QString *error);

    /**
     * 释放所有共享内存缓冲。
     * @return 无返回值。
     */
    void reset();

    /**
     * 等待所有出租中的缓冲归还。
     * @param timeoutMs 最长等待毫秒数。
     * @return 全部归还时返回 true，超时时返回 false。
     */
    bool waitUntilIdle(int timeoutMs) const;

private:
    static constexpr int kMaxBuffers = 16;
    std::vector<std::shared_ptr<WlrootsScreencopyShmBuffer>> m_buffers;
};

}  // namespace markshot::recording
