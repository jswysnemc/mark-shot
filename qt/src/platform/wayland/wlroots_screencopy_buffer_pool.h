#pragma once

#include "platform/wayland/wlroots_screencopy_shm_buffer.h"
#include "recording/recording_capture_memory_policy.h"

#include <QString>

#include <cstdint>
#include <memory>
#include <vector>

struct wl_shm;

namespace markshot::recording {

class WlrootsScreencopyBufferPool final {
public:
    /** 创建采集池，budgetBytes 为像素预算；默认值读取环境配置。 */
    explicit WlrootsScreencopyBufferPool(quint64 budgetBytes = recordingCaptureMemoryBudget());
    /** 在连接仍有效的采集线程解除协议资源，借出的映射继续有效。 */
    ~WlrootsScreencopyBufferPool();
    WlrootsScreencopyBufferPool(const WlrootsScreencopyBufferPool &) = delete;
    WlrootsScreencopyBufferPool &operator=(const WlrootsScreencopyBufferPool &) = delete;
    /**
     * 获取满足当前帧参数的共享内存缓冲。
     * @param shm Wayland 共享内存接口。
     * @param format wl_shm 像素格式。
     * @param width 帧宽度。
     * @param height 帧高度。
     * @param stride 行跨度。
     * @param error 输出错误信息。
     * @param nowMs 单调毫秒时钟，可注入测试时间。
     * @return 可写入的共享内存缓冲，暂时无可用缓冲时返回空。
     */
    std::shared_ptr<WlrootsScreencopyShmBuffer> acquire(wl_shm *shm,
                                                        std::uint32_t format,
                                                        int width,
                                                        int height,
                                                        int stride,
                                                        QString *error,
                                                        qint64 nowMs = recordingCaptureClockMs());

    /** 按 nowMs 回收连续空闲两秒的缓冲，不释放借出的像素，无返回值。 */
    void reclaimIdle(qint64 nowMs = recordingCaptureClockMs());

    /** 返回所有池槽位的映射字节数，每块共享存储只统计一次，无参数。 */
    quint64 retainedBytes() const;

    /**
     * 【录制】【映射寿命】关闭所有协议缓冲并归还池引用，借出的像素映射由最后一个帧释放。
     * 必须在采集线程且 Wayland 连接仍有效时调用。
     * @return 无返回值。
     */
    void reset();

private:
    struct Entry {
        std::shared_ptr<WlrootsScreencopyShmBuffer> buffer;
        qint64 idleSince = -1;
    };
    static constexpr int kMaxBuffers = 16;
    quint64 m_budgetBytes;
    std::vector<Entry> m_buffers;
};

}  // namespace markshot::recording
