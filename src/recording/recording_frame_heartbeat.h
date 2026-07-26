#pragma once

#include <QElapsedTimer>
#include <QObject>
#include <QTimer>

namespace markshot::recording {

/**
 * 【录制】【静止补帧】在采集端长时间不出帧时驱动补写上一帧。
 *
 * 事件驱动的采集后端只在画面变化时出帧，画面静止时时间轴会停滞，
 * 写出端的追帧上限又限制了单次补帧数量，最终使静止时段被压缩。
 * 心跳按固定间隔检查距上一帧的时长，超过阈值即请求补写一帧。
 */
class RecordingFrameHeartbeat final : public QObject {
    Q_OBJECT

public:
    /**
     * 创建静止补帧心跳。
     * @param parent 父对象。
     */
    explicit RecordingFrameHeartbeat(QObject *parent = nullptr);

    /**
     * 按目标帧率启动心跳。
     * @param fps 录制帧率。
     * @return 无返回值。
     */
    void start(int fps);

    /**
     * 停止心跳。
     * @return 无返回值。
     */
    void stop();

    /**
     * 设置暂停状态，暂停期间不请求补帧。
     * @param paused 暂停时为 true。
     * @return 无返回值。
     */
    void setPaused(bool paused);

    /**
     * 记录一帧已经写出。
     * @param timestampMs 该帧的采集时间戳。
     * @return 无返回值。
     */
    void noteFrameWritten(qint64 timestampMs);

signals:
    /**
     * 请求补写一帧。
     * @param timestampMs 按墙钟推算出的采集时间戳。
     */
    void repeatFrameNeeded(qint64 timestampMs);

private:
    /**
     * 检查距上一帧的时长并决定是否请求补帧。
     * @return 无返回值。
     */
    void tick();

    QTimer m_timer;
    QElapsedTimer m_sinceLastFrame;
    qint64 m_lastTimestampMs = 0;
    qint64 m_idleThresholdMs = 250;
    bool m_hasFrame = false;
    bool m_paused = false;
};

}  // namespace markshot::recording
