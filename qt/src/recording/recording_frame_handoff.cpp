#include "recording/recording_frame_handoff.h"

#include <QMetaObject>
#include <QThread>

#include <mutex>
#include <utility>

namespace markshot::recording {

struct RecordingFrameHandoff::State {
    std::mutex mutex;
    RecordingFrameHandoff *owner = nullptr;
    RecordingFrameSample pending;
    QString error;
    qint64 latestTimestamp = 0;
    qint64 latestSequence = 0;
    bool hasTimestamp = false;
    bool wakePending = false;
    bool enabled = true;
    bool failed = false;

    /** 在持有 state 锁时合并 Qt 唤醒通知，事件仅持有状态，不持有帧副本，无返回值。 */
    static void scheduleLocked(const std::shared_ptr<State> &state)
    {
        if (state->wakePending) {
            return;
        }
        state->wakePending = true;
        if (!QMetaObject::invokeMethod(state->owner, [state] { deliver(state); }, Qt::QueuedConnection)) {
            state->wakePending = false;
        }
    }

    /** 在接收线程取走 state 中的最新帧或首个错误，锁外通知接收方，无返回值。 */
    static void deliver(const std::shared_ptr<State> &state)
    {
        RecordingFrameSample sample;
        QString error;
        RecordingFrameHandoff *owner;
        {
            std::lock_guard lock(state->mutex);
            state->wakePending = false;
            owner = state->owner;
            sample = std::exchange(state->pending, {});
            error = std::exchange(state->error, {});
        }
        if (!owner) {
            return;
        }
        // 1. 【录制】【帧交接】回调可能同步触发背压、关闭甚至销毁交接器，通知后不再访问 owner
        if (!error.isEmpty()) {
            emit owner->failed(error);
        } else if (sample.hasFrameData()) {
            emit owner->frameReady(sample);
        }
    }
};

RecordingFrameHandoff::RecordingFrameHandoff(QObject *parent)
    : QObject(parent), m_state(std::make_shared<State>())
{
    m_state->owner = this;
}

RecordingFrameHandoff::~RecordingFrameHandoff()
{
    close();
}

RecordingFrameHandoff::FramePublisher RecordingFrameHandoff::publisher() const
{
    return [state = m_state](RecordingFrameSample sample) {
        std::lock_guard lock(state->mutex);
        if (!state->owner || !state->enabled || state->failed || !sample.hasFrameData()) {
            return;
        }
        if (state->hasTimestamp
            && (sample.timestampMs < state->latestTimestamp
                || (sample.timestampMs == state->latestTimestamp && sample.sequence <= state->latestSequence))) {
            return;
        }
        state->hasTimestamp = true;
        state->latestTimestamp = sample.timestampMs;
        state->latestSequence = sample.sequence;
        // 1. 【录制】【帧交接】替换尚未交付的旧帧，旧载荷通过参数在解锁之后析构
        sample = std::exchange(state->pending, std::move(sample));
        State::scheduleLocked(state);
    };
}

RecordingFrameHandoff::ErrorPublisher RecordingFrameHandoff::errorPublisher() const
{
    return [state = m_state](QString error) {
        RecordingFrameSample released;
        std::lock_guard lock(state->mutex);
        if (!state->owner || state->failed || error.isEmpty()) {
            return;
        }
        state->failed = true;
        state->error = std::move(error);
        released = std::exchange(state->pending, {});
        State::scheduleLocked(state);
    };
}

void RecordingFrameHandoff::setEnabled(bool enabled)
{
    Q_ASSERT(QThread::currentThread() == thread());
    RecordingFrameSample released;
    std::lock_guard lock(m_state->mutex);
    m_state->enabled = enabled;
    if (!enabled) {
        released = std::exchange(m_state->pending, {});
    }
}

void RecordingFrameHandoff::close()
{
    Q_ASSERT(QThread::currentThread() == thread());
    RecordingFrameSample released;
    std::lock_guard lock(m_state->mutex);
    m_state->owner = nullptr;
    released = std::exchange(m_state->pending, {});
    m_state->error.clear();
}

}  // namespace markshot::recording
