#pragma once

namespace markshot::recording {

class RecordingFrameQueue final {
public:
    /**
     * 创建录制帧队列计数器。
     * @param capacity 最大待写帧数。
     */
    explicit RecordingFrameQueue(int capacity = 2);

    /**
     * 设置最大待写帧数。
     * @param capacity 最大待写帧数。
     * @return 无返回值。
     */
    void setCapacity(int capacity);

    /**
     * 尝试登记一个待写帧。
     * @return 队列有容量时返回 true，队列满时记录丢帧并返回 false。
     */
    bool tryEnqueue();

    /**
     * 标记一个待写帧已完成。
     * @return 无返回值。
     */
    void completeOne();

    /**
     * 清空队列计数。
     * @return 无返回值。
     */
    void reset();

    /**
     * 判断队列是否进入背压状态。
     * @return 待写帧达到容量上限时返回 true。
     */
    bool backpressureActive() const;

    /**
     * 读取已丢弃帧数。
     * @return 丢弃帧数。
     */
    int droppedFrames() const;

private:
    int m_capacity = 2;
    int m_pending = 0;
    int m_dropped = 0;
};

}  // namespace markshot::recording
