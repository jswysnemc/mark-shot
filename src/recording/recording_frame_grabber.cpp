#include "recording/recording_frame_grabber.h"

#include "screen_capture.h"

#include <QImage>

#include <algorithm>
#include <utility>

namespace markshot::recording {

RecordingFrameGrabber::RecordingFrameGrabber(RecordingOptions options, QObject *parent)
    : QObject(parent)
    , m_options(std::move(options))
{
    m_timer.setTimerType(Qt::PreciseTimer);
    connect(&m_timer, &QTimer::timeout, this, &RecordingFrameGrabber::captureFrame);
}

void RecordingFrameGrabber::start()
{
    const int intervalMs = std::max(1, 1000 / std::max(1, m_options.fps));
    m_timer.start(intervalMs);
    QTimer::singleShot(0, this, &RecordingFrameGrabber::captureFrame);
}

void RecordingFrameGrabber::stop()
{
    m_timer.stop();
}

void RecordingFrameGrabber::captureFrame()
{
    if (m_capturing) {
        return;
    }

    m_capturing = true;
    CaptureRequest request;
    request.sourceGeometry = m_options.captureGeometry;
    request.preferredOutputName = m_options.display.outputName;
    request.allOutputs = m_options.display.allOutputs && m_options.scope == RecordingScope::Display;
    request.preferScreencast = true;
    request.allowInteractivePortal = true;
    request.allowPortalScreenshotFallback = true;
    request.includeCursor = true;

    const CaptureResult result = captureScreenFrame(request);
    if (result.image.isNull()) {
        m_capturing = false;
        emit failed(result.error.isEmpty()
                        ? QStringLiteral("screen recording frame capture failed")
                        : result.error);
        return;
    }

    m_capturing = false;
    emit frameReady(result.image);
}

}  // namespace markshot::recording
