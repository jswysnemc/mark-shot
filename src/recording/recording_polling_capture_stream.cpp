#include "recording/recording_polling_capture_stream.h"

#include "screen_capture.h"

#include <QImage>
#include <QPointer>

#include <algorithm>
#include <utility>

namespace markshot::recording {

RecordingPollingCaptureStream::RecordingPollingCaptureStream(RecordingOptions options, QObject *parent)
    : RecordingCaptureStream(parent)
    , m_options(std::move(options))
{
    m_timer.setTimerType(Qt::PreciseTimer);
    m_timer.setSingleShot(true);
    connect(&m_timer, &QTimer::timeout, this, &RecordingPollingCaptureStream::captureFrame);
}

bool RecordingPollingCaptureStream::start(QString *error)
{
    if (error) {
        error->clear();
    }
    m_intervalUs = std::max<qint64>(1, 1000000 / std::max(1, m_options.fps));
    m_nextCaptureUs = 0;
    m_sequence = 0;
    m_screencastFailed = false;
    m_running = true;
    m_clock.restart();
    scheduleNextCapture(0);
    return true;
}

void RecordingPollingCaptureStream::stop()
{
    m_running = false;
    m_timer.stop();
}

void RecordingPollingCaptureStream::setBackpressureActive(bool active)
{
    if (m_backpressureActive == active) {
        return;
    }
    m_backpressureActive = active;
    if (!m_backpressureActive && m_running) {
        scheduleNextCapture(0);
    }
}

void RecordingPollingCaptureStream::captureFrame()
{
    if (!m_running || m_capturing || m_backpressureActive) {
        return;
    }
    if (m_options.mode == RecordingMode::Video && m_clock.isValid()
        && m_clock.nsecsElapsed() / 1000 < m_nextCaptureUs) {
        scheduleNextCapture();
        return;
    }

    m_capturing = true;
    QElapsedTimer captureElapsed;
    captureElapsed.start();
    CaptureRequest request;
    request.sourceGeometry = m_options.captureGeometry;
    request.preferredOutputName = m_options.display.outputName;
    request.allOutputs = m_options.display.allOutputs && m_options.scope == RecordingScope::Display;
    request.preferScreencast = true;
    // 1. 【录制】【轮询授权】逐帧采集只走非交互路径，GIF 首帧可在原生回退失败后授权一次
    request.allowInteractivePortal = false;
    request.allowPortalScreenshotFallback = false;
    request.allowInteractiveScreencastInit = m_options.mode == RecordingMode::Gif && m_sequence == 0;
    request.allowScreencast = !m_screencastFailed;
    request.includeCursor = true;
    request.targetFps = m_options.mode == RecordingMode::Video ? m_options.fps : 0;

    // 2. 【录制】【轮询采集】门户请求可能进入嵌套事件循环，期间录制可被停止并销毁本对象
    QPointer<RecordingPollingCaptureStream> self(this);
    const CaptureResult result = captureScreenFrame(request);
    if (!self) {
        return;
    }
    m_capturing = false;
    if (!m_running) {
        return;
    }
    m_screencastFailed = m_screencastFailed || result.screencastFailed;
    const qint64 captureMs = captureElapsed.elapsed();
    if (result.image.isNull()) {
        emit failed(result.error.isEmpty()
                        ? QStringLiteral("screen recording frame capture failed")
                        : result.error);
        return;
    }

    RecordingFrameSample sample;
    sample.image = result.image;
    sample.timestampMs = m_clock.elapsed();
    sample.sequence = ++m_sequence;
    advanceNextCaptureTime();
    updateAdaptivePacing(captureMs);
    // 3. 【录制】【帧分发】接收方可能同步停止录制并销毁采集流，发射后需再次确认对象存活
    emit frameReady(sample);
    if (!self) {
        return;
    }
    scheduleNextCapture();
}

void RecordingPollingCaptureStream::scheduleNextCapture(int delayOverrideMs)
{
    if (!m_running || m_timer.isActive()) {
        return;
    }

    int delayMs = delayOverrideMs;
    if (delayMs < 0) {
        const qint64 nowUs = m_clock.isValid() ? m_clock.nsecsElapsed() / 1000 : 0;
        const qint64 remainingUs = std::max<qint64>(0, m_nextCaptureUs - nowUs);
        delayMs = static_cast<int>((remainingUs + 999) / 1000);
    }
    m_timer.start(std::max(0, delayMs));
}

void RecordingPollingCaptureStream::advanceNextCaptureTime()
{
    if (!m_clock.isValid()) {
        return;
    }
    const qint64 nowUs = m_clock.nsecsElapsed() / 1000;
    if (m_nextCaptureUs <= 0) {
        m_nextCaptureUs = nowUs + m_intervalUs;
        return;
    }
    do {
        m_nextCaptureUs += m_intervalUs;
    } while (m_nextCaptureUs <= nowUs);
}

void RecordingPollingCaptureStream::updateAdaptivePacing(qint64 captureMs)
{
    if (m_options.mode != RecordingMode::Video || !m_clock.isValid()) {
        return;
    }
    if (captureMs * 1000 <= m_intervalUs / 2) {
        return;
    }

    const qint64 intervalMs = std::max<qint64>(1, m_intervalUs / 1000);
    const qint64 cooldownMs = std::min<qint64>(intervalMs * 4,
                                               std::max<qint64>(intervalMs, captureMs * 2));
    const qint64 cooldownUntilUs = (m_clock.nsecsElapsed() / 1000) + cooldownMs * 1000;
    m_nextCaptureUs = std::max(m_nextCaptureUs, cooldownUntilUs);
}

}  // namespace markshot::recording
