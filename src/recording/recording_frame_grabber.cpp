#include "recording/recording_frame_grabber.h"

#include "debug_log.h"
#include "recording/recording_capture_backend.h"

#include <QPointer>
#include <QStringList>
#include <QTimer>

#include <utility>

namespace markshot::recording {
namespace {

/**
 * 【录制】【采集后端】拼接后端尝试顺序文本。
 * @param backends 后端列表。
 * @return 日志文本。
 */
QString backendOrderText(const QVector<RecordingCaptureBackend> &backends)
{
    QStringList names;
    names.reserve(backends.size());
    for (RecordingCaptureBackend backend : backends) {
        names.append(recordingCaptureBackendName(backend));
    }
    return names.join(QStringLiteral("->"));
}

}  // namespace

RecordingFrameGrabber::RecordingFrameGrabber(RecordingOptions options,
                                             RecordingCaptureStreamFactory streamFactory,
                                             QObject *parent)
    : QObject(parent)
    , m_options(std::move(options))
    , m_streamFactory(std::move(streamFactory))
{
}

void RecordingFrameGrabber::start()
{
    stop();
    m_running = true;

    QString error;
    if (!startCaptureStream(&error)) {
        m_running = false;
        emit failed(error.isEmpty()
                        ? QStringLiteral("recording capture stream failed to start")
                        : error);
    }
}

void RecordingFrameGrabber::stop()
{
    m_running = false;
    m_fallbackPending = false;
    m_receivedFirstFrame = false;
    m_activeBackend = RecordingCaptureBackend::Auto;
    m_captureBackends.clear();
    m_nextBackendIndex = 0;
    if (m_stream) {
        m_stream->stop();
    }
    m_stream.reset();
}

void RecordingFrameGrabber::setBackpressureActive(bool active)
{
    m_backpressureActive = active;
    if (m_stream) {
        m_stream->setBackpressureActive(active);
    }
}

bool RecordingFrameGrabber::startCaptureStream(QString *error)
{
    if (error) {
        error->clear();
    }

    const RecordingCaptureBackend environmentBackend = recordingCaptureBackendFromEnvironment();
    const RecordingCaptureBackend requested = environmentBackend == RecordingCaptureBackend::Auto
        ? m_options.captureBackend
        : environmentBackend;
    m_captureBackends = recordingCaptureBackendOrder(requested);
    m_nextBackendIndex = 0;
    markshot::debugLog("recording",
                       "【录制】【采集后端选择】requested=%s order=%s",
                       recordingCaptureBackendName(requested).toUtf8().constData(),
                       backendOrderText(m_captureBackends).toUtf8().constData());

    return startNextCaptureStream(error);
}

bool RecordingFrameGrabber::startNextCaptureStream(QString *error)
{
    if (error) {
        error->clear();
    }

    QString lastError;
    while (m_running && m_nextBackendIndex < m_captureBackends.size()) {
        const RecordingCaptureBackend backend = m_captureBackends.at(m_nextBackendIndex++);
        std::unique_ptr<RecordingCaptureStream> stream =
            m_streamFactory ? m_streamFactory(backend, m_options, this) : nullptr;
        if (!stream) {
            lastError = QStringLiteral("recording capture backend %1 is not available")
                            .arg(recordingCaptureBackendName(backend));
            markshot::debugLog("recording",
                               "【录制】【采集后端跳过】backend=%s reason=unavailable",
                               recordingCaptureBackendName(backend).toUtf8().constData());
            continue;
        }

        m_stream = std::move(stream);
        m_activeBackend = backend;
        m_receivedFirstFrame = false;
        m_fallbackPending = false;
        connectCaptureStream(m_stream.get());
        QString streamError;
        if (m_stream->start(&streamError)) {
            markshot::debugLog("recording",
                               "【录制】【采集后端】backend=%s",
                               recordingCaptureBackendName(backend).toUtf8().constData());
            m_stream->setBackpressureActive(m_backpressureActive);
            return true;
        }
        lastError = streamError;
        markshot::debugLog("recording",
                           "【录制】【采集后端回退】backend=%s error=%s",
                           recordingCaptureBackendName(backend).toUtf8().constData(),
                           streamError.toUtf8().constData());
        m_stream->stop();
        m_stream.reset();
        m_activeBackend = RecordingCaptureBackend::Auto;
        m_fallbackPending = false;
    }

    if (error) {
        *error = lastError.isEmpty()
            ? QStringLiteral("no recording capture backend is available")
            : lastError;
    }
    return false;
}

void RecordingFrameGrabber::connectCaptureStream(RecordingCaptureStream *stream)
{
    connect(stream,
            &RecordingCaptureStream::frameReady,
            this,
            [this, stream](const RecordingFrameSample &sample) {
                handleFrameReady(stream, sample);
            });
    connect(stream,
            &RecordingCaptureStream::failed,
            this,
            [this, stream](const QString &error) {
                handleCaptureFailure(stream, error);
            });
}

void RecordingFrameGrabber::handleFrameReady(RecordingCaptureStream *stream,
                                             const RecordingFrameSample &sample)
{
    if (!m_running || m_fallbackPending || stream != m_stream.get()) {
        return;
    }
    m_receivedFirstFrame = true;
    emit frameReady(sample);
}

void RecordingFrameGrabber::handleCaptureFailure(RecordingCaptureStream *stream,
                                                 const QString &error)
{
    if (!m_running || stream != m_stream.get()) {
        return;
    }
    if (m_receivedFirstFrame) {
        m_running = false;
        emit failed(error);
        return;
    }
    if (m_fallbackPending) {
        return;
    }

    m_fallbackPending = true;
    markshot::debugLog("recording",
                       "【录制】【采集后端异步回退】backend=%s error=%s",
                       recordingCaptureBackendName(m_activeBackend).toUtf8().constData(),
                       error.toUtf8().constData());
    QPointer<RecordingCaptureStream> failedStream(stream);
    QTimer::singleShot(0, this, [this, failedStream, error] {
        continueCaptureFallback(failedStream.data(), error);
    });
}

void RecordingFrameGrabber::continueCaptureFallback(RecordingCaptureStream *stream,
                                                    const QString &error)
{
    if (!m_running || !stream || stream != m_stream.get()) {
        return;
    }

    m_stream->stop();
    m_stream.reset();
    m_activeBackend = RecordingCaptureBackend::Auto;
    m_fallbackPending = false;

    QString fallbackError;
    if (startNextCaptureStream(&fallbackError)) {
        return;
    }

    m_running = false;
    emit failed(fallbackError.isEmpty() ? error : fallbackError);
}

}  // namespace markshot::recording
