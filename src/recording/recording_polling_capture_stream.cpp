#include "recording/recording_polling_capture_stream.h"

#include "screen_capture.h"

#include <QImage>

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
    request.allowInteractivePortal = m_options.mode != RecordingMode::Video;
    request.allowPortalScreenshotFallback = m_options.mode != RecordingMode::Video;
    request.includeCursor = true;
    request.targetFps = m_options.mode == RecordingMode::Video ? m_options.fps : 0;

    const CaptureResult result = captureScreenFrame(request);
    const qint64 captureMs = captureElapsed.elapsed();
    if (result.image.isNull()) {
        m_capturing = false;
        emit failed(result.error.isEmpty()
                        ? QStringLiteral("screen recording frame capture failed")
                        : result.error);
        return;
    }

    RecordingFrameSample sample;
    sample.image = result.image;
    sample.timestampMs = m_clock.elapsed();
    sample.sequence = ++m_sequence;
    m_capturing = false;
    advanceNextCaptureTime();
    updateAdaptivePacing(captureMs);
    emit frameReady(sample);
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
