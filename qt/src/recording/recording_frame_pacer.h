#pragma once

#include <QtGlobal>

namespace markshot::recording {

class RecordingFramePacer final {
public:
    struct Decision {
        qint64 frameIndex = -1;
        int repeatCount = 0;
    };

    /**
     * 创建固定帧率节奏器。
     * @param fps 目标帧率。
     */
    explicit RecordingFramePacer(int fps = 30);

    /**
     * 重置节奏器状态。
     * @param fps 目标帧率。
     * @return 无返回值。
     */
    void reset(int fps);

    /**
     * 【录制】【帧率对齐】把采集时间映射到固定输出时间格。
     * @param timestampMs 当前样本相对录制开始的时间。
     * @return frameIndex 为 -1 时丢弃，否则按 repeatCount 补帧并写入该序号；单次最多补十秒。
     */
    Decision plan(qint64 timestampMs);

    /**
     * 【录制】【采集调度】计算下一时间格的等待时长，不修改状态。
     * @param timestampMs 与 plan 使用同一时钟的当前毫秒时间。
     * @return 尚未到期的毫秒数，首帧或已到期时为零。
     */
    qint64 delayBeforeNext(qint64 timestampMs) const;

private:
    qint64 m_fps = 30;
    qint64 m_originMs = 0;
    qint64 m_nextFrame = 0;
    bool m_started = false;
};

}  // namespace markshot::recording
