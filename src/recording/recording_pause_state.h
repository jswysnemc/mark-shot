#pragma once

#include <QElapsedTimer>

namespace markshot::recording {

/**
 * 【录制】【暂停】维护录制时间轴，把暂停时长从输出时间轴中扣除。
 *
 * 采集后端各自维护时间戳起点，暂停期间不会有帧到达，
 * 因此暂停时长按墙钟累计，并在恢复后从后续帧的时间戳中减去。
 */
class RecordingPauseState final {
public:
    /**
     * 进入暂停状态。
     * @return 状态发生变化时返回 true。
     */
    bool pause();

    /**
     * 退出暂停状态并累计本次暂停时长。
     * @return 状态发生变化时返回 true。
     */
    bool resume();

    /**
     * 清空时间轴与暂停累计。
     * @return 无返回值。
     */
    void reset();

    /**
     * 判断当前是否处于暂停状态。
     * @return 暂停时返回 true。
     */
    bool isPaused() const
    {
        return m_paused;
    }

    /**
     * 读取累计暂停时长。
     * @return 累计暂停毫秒数。
     */
    qint64 pausedTotalMs() const;

    /**
     * 把采集时间戳换算为输出时间轴。
     * @param timestampMs 采集时间戳。
     * @return 扣除起点与暂停时长后的时间轴毫秒数。
     */
    qint64 timelineFor(qint64 timestampMs);

private:
    QElapsedTimer m_pauseClock;
    qint64 m_baseTimestampMs = 0;
    qint64 m_pausedTotalMs = 0;
    bool m_started = false;
    bool m_paused = false;
};

}  // namespace markshot::recording
