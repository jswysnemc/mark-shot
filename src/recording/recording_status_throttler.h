#pragma once

#include <QElapsedTimer>

namespace markshot::recording {

class RecordingStatusThrottler final {
public:
    /**
     * 创建录制状态节流器。
     * @param intervalMs 两次自动发布之间的最小间隔毫秒数。
     */
    explicit RecordingStatusThrottler(int intervalMs = 1000);

    /**
     * 重置节流计时状态。
     * @return 无返回值。
     */
    void reset();

    /**
     * 判断当前是否应该发布录制状态。
     * @param force 是否强制发布并重置计时。
     * @return 需要发布状态时返回 true。
     */
    bool shouldPublish(bool force);

private:
    int m_intervalMs = 1000;
    QElapsedTimer m_elapsed;
};

}  // namespace markshot::recording
