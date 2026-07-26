#include "recording/recording_frame_heartbeat.h"

#include <algorithm>

namespace markshot::recording {
namespace {

// 心跳检查间隔，兼顾补帧及时性与空转开销
constexpr int kTickIntervalMs = 200;

}  // namespace

RecordingFrameHeartbeat::RecordingFrameHeartbeat(QObject *parent)
    : QObject(parent)
{
    m_timer.setInterval(kTickIntervalMs);
    m_timer.setTimerType(Qt::CoarseTimer);
    connect(&m_timer, &QTimer::timeout, this, &RecordingFrameHeartbeat::tick);
}

void RecordingFrameHeartbeat::start(int fps)
{
    const int safeFps = std::max(1, fps);
    // 阈值取若干帧间隔，正常出帧的录制不会触发补帧
    m_idleThresholdMs = std::clamp<qint64>(1000 / safeFps * 3, 120, 500);
    m_lastTimestampMs = 0;
    m_hasFrame = false;
    m_paused = false;
    m_sinceLastFrame.invalidate();
    m_timer.start();
}

void RecordingFrameHeartbeat::stop()
{
    m_timer.stop();
    m_hasFrame = false;
    m_paused = false;
    m_sinceLastFrame.invalidate();
}

void RecordingFrameHeartbeat::setPaused(bool paused)
{
    if (m_paused == paused) {
        return;
    }
    m_paused = paused;
    // 恢复后重新计时，暂停时长不应触发补帧
    if (!m_paused && m_hasFrame) {
        m_sinceLastFrame.restart();
    }
}

void RecordingFrameHeartbeat::noteFrameWritten(qint64 timestampMs)
{
    m_lastTimestampMs = timestampMs;
    m_hasFrame = true;
    m_sinceLastFrame.restart();
}

void RecordingFrameHeartbeat::tick()
{
    if (m_paused || !m_hasFrame || !m_sinceLastFrame.isValid()) {
        return;
    }

    const qint64 idleMs = m_sinceLastFrame.elapsed();
    if (idleMs < m_idleThresholdMs) {
        return;
    }

    // 按墙钟推进采集时间轴，保持与采集端时间戳同源
    emit repeatFrameNeeded(m_lastTimestampMs + idleMs);
}

}  // namespace markshot::recording
