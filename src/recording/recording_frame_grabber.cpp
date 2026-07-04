#include "recording/recording_frame_grabber.h"

#include "debug_log.h"
#include "platform/wayland/wlroots_screencopy_capture_stream.h"
#include "recording/recording_capture_backend.h"
#include "recording/recording_pipewire_capture_stream.h"
#include "recording/recording_polling_capture_stream.h"
#include "recording/recording_windows_wgc_capture_stream.h"

#include <QStringList>

#include <utility>

namespace markshot::recording {
namespace {

/**
 * 【录制】【采集后端】创建指定类型的采集流。
 * @param backend 采集后端。
 * @param options 录制配置。
 * @param parent 父对象。
 * @return 可用时返回采集流，否则返回空。
 */
std::unique_ptr<RecordingCaptureStream> createStreamForBackend(RecordingCaptureBackend backend,
                                                               const RecordingOptions &options,
                                                               QObject *parent)
{
    switch (backend) {
    case RecordingCaptureBackend::Wlroots:
        return createWlrootsScreencopyCaptureStream(options, parent);
    case RecordingCaptureBackend::PipeWire:
        return createPipeWireRecordingCaptureStream(options, parent);
    case RecordingCaptureBackend::WindowsWgc:
        return createWindowsWgcRecordingCaptureStream(options, parent);
    case RecordingCaptureBackend::Polling:
        return std::make_unique<RecordingPollingCaptureStream>(options, parent);
    case RecordingCaptureBackend::Auto:
        break;
    }
    return nullptr;
}

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

RecordingFrameGrabber::RecordingFrameGrabber(RecordingOptions options, QObject *parent)
    : QObject(parent)
    , m_options(std::move(options))
{
}

void RecordingFrameGrabber::start()
{
    QString error;
    if (!startCaptureStream(&error)) {
        emit failed(error.isEmpty()
                        ? QStringLiteral("recording capture stream failed to start")
                        : error);
    }
}

void RecordingFrameGrabber::stop()
{
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
    const QVector<RecordingCaptureBackend> backends = recordingCaptureBackendOrder(requested);
    markshot::debugLog("recording",
                       "【录制】【采集后端选择】requested=%s order=%s",
                       recordingCaptureBackendName(requested).toUtf8().constData(),
                       backendOrderText(backends).toUtf8().constData());

    QString lastError;
    for (RecordingCaptureBackend backend : backends) {
        std::unique_ptr<RecordingCaptureStream> stream =
            createStreamForBackend(backend, m_options, this);
        if (!stream) {
            lastError = QStringLiteral("recording capture backend %1 is not available")
                            .arg(recordingCaptureBackendName(backend));
            markshot::debugLog("recording",
                               "【录制】【采集后端跳过】backend=%s reason=unavailable",
                               recordingCaptureBackendName(backend).toUtf8().constData());
            continue;
        }

        m_stream = std::move(stream);
        connectCaptureStream();
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
        m_stream.reset();
    }

    if (error) {
        *error = lastError.isEmpty()
            ? QStringLiteral("no recording capture backend is available")
            : lastError;
    }
    return false;
}

void RecordingFrameGrabber::connectCaptureStream()
{
    connect(m_stream.get(),
            &RecordingCaptureStream::frameReady,
            this,
            &RecordingFrameGrabber::frameReady);
    connect(m_stream.get(),
            &RecordingCaptureStream::failed,
            this,
            &RecordingFrameGrabber::failed);
}

}  // namespace markshot::recording
