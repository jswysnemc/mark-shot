#pragma once

#include <QtGlobal>

namespace markshot::recording {

class RecordingFramePacer final {
public:
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
     * 计算写入当前样本前需要补写的上一帧数量。
     * @param timestampMs 当前样本相对录制开始的时间。
     * @return 需要重复上一帧的数量。
     */
    int duplicatesBefore(qint64 timestampMs);

private:
    qint64 m_intervalUs = 33333;
    qint64 m_nextFrameUs = 0;
    int m_maxCatchUpFrames = 30;
    bool m_started = false;
};

}  // namespace markshot::recording
