#pragma once

#include <QtGlobal>

namespace markshot::recording {

/**
 * 【录制】【帧率限制】按目标帧率丢弃过密的采集帧。
 *
 * 事件驱动的采集后端按合成器刷新率出帧，帧率高于录制目标时
 * 多余帧只会增加编码负载，需要在写出前按时间戳筛除。
 */
class RecordingFrameRateLimiter final {
public:
    /**
     * 创建帧率限制器。
     * @param fps 目标帧率。
     */
    explicit RecordingFrameRateLimiter(int fps = 12);

    /**
     * 重置时间轴并设置目标帧率。
     * @param fps 目标帧率。
     * @return 无返回值。
     */
    void reset(int fps);

    /**
     * 判断该帧是否应当写出。
     * @param timestampMs 采集时间戳。
     * @return 距上一写出帧已达目标间隔时返回 true。
     */
    bool shouldWrite(qint64 timestampMs);

private:
    qint64 m_intervalMs = 0;
    qint64 m_lastAcceptedMs = 0;
    bool m_started = false;
};

}  // namespace markshot::recording
