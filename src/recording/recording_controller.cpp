#include "recording/recording_controller.h"

#include "debug_log.h"
#include "recording/recording_async_writer.h"
#include "recording/recording_frame_grabber.h"

#include <QImage>

#include <algorithm>

namespace markshot::recording {
namespace {

/**
 * 按录制模式创建写出器。
 * @param options 录制配置。
 * @return 写出器实例。
 */
std::unique_ptr<RecordingWriter> createWriter(const RecordingOptions &options)
{
    return std::make_unique<RecordingAsyncWriter>(options);
}

}  // namespace

RecordingController::RecordingController(QObject *parent)
    : QObject(parent)
{
}

bool RecordingController::start(const RecordingOptions &options, QString *error)
{
    if (error) {
        error->clear();
    }
    if (options.captureGeometry.isEmpty()) {
        if (error) {
            *error = QStringLiteral("recording capture geometry is empty");
        }
        return false;
    }
    if (options.outputPath.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("recording output path is empty");
        }
        return false;
    }

    m_options = options;
    m_writer = createWriter(m_options);
    m_grabber = new RecordingFrameGrabber(m_options, this);
    if (RecordingAsyncWriter *writer = asyncWriter()) {
        connect(writer, &RecordingAsyncWriter::failed, this, &RecordingController::fail);
        connect(writer, &RecordingAsyncWriter::finished, this, &RecordingController::handleWriterFinished);
        connect(writer, &RecordingAsyncWriter::backpressureChanged,
                m_grabber, &RecordingFrameGrabber::setBackpressureActive);
    }
    connect(m_grabber, &RecordingFrameGrabber::frameReady, this, &RecordingController::handleFrame);
    connect(m_grabber, &RecordingFrameGrabber::failed, this, &RecordingController::fail);
    m_statusThrottler.reset();
    m_recordedElapsedMs = 0;
    m_finishEmitted = false;

    markshot::debugLog("recording",
                       "【录制】【开始】mode=%s fps=%d audio=%d geometry=%d,%d %dx%d output=%s",
                       m_options.mode == RecordingMode::Gif ? "gif" : "video",
                       m_options.fps,
                       m_options.includeAudio ? 1 : 0,
                       m_options.captureGeometry.x(),
                       m_options.captureGeometry.y(),
                       m_options.captureGeometry.width(),
                       m_options.captureGeometry.height(),
                       m_options.outputPath.toUtf8().constData());
    m_grabber->start();
    publishStatus(true);
    return true;
}

void RecordingController::requestStop()
{
    stop();
}

RecordingStatus RecordingController::status() const
{
    RecordingStatus result;
    result.active = !m_stopping;
    result.mode = m_options.mode;
    result.fps = m_options.fps;
    result.frameCount = m_frameCount;
    result.elapsedMs = m_recordedElapsedMs;
    result.outputPath = m_options.outputPath;
    return result;
}

void RecordingController::handleFrame(const RecordingFrameSample &sample)
{
    if (m_stopping) {
        return;
    }

    QString error;
    if (!m_writerStarted) {
        const QSize frameSize = sample.frameSize();
        if (frameSize.isEmpty()) {
            fail(QStringLiteral("recording frame size is empty"));
            return;
        }
        if (!m_writer->start(frameSize, m_options.fps, &error)) {
            fail(error);
            return;
        }
        m_writerStarted = true;
    }

    if (!m_writer->writeFrame(sample, &error)) {
        fail(error);
        return;
    }
    m_recordedElapsedMs = std::max(m_recordedElapsedMs, sample.timestampMs);
    ++m_frameCount;
    publishStatus(false);
}

void RecordingController::stop()
{
    if (m_stopping) {
        return;
    }
    m_stopping = true;
    if (m_grabber) {
        m_grabber->stop();
    }

    QString error;
    bool ok = false;
    if (!m_writerStarted || m_frameCount <= 0) {
        error = QStringLiteral("No recording frames were captured");
        if (m_writer) {
            m_writer->cancel();
        }
        completeStop(false, error);
        return;
    }

    if (RecordingAsyncWriter *writer = asyncWriter()) {
        if (!writer->finish(&error)) {
            completeStop(false, error);
        } else {
            publishStatus(true);
        }
        return;
    } else {
        ok = m_writer->finish(&error);
    }
    completeStop(ok, error);
}

void RecordingController::fail(const QString &message)
{
    if (m_stopping) {
        return;
    }
    m_stopping = true;
    if (m_grabber) {
        m_grabber->stop();
    }
    if (m_writer) {
        m_writer->cancel();
    }
    markshot::debugLog("recording",
                       "【录制】【失败】frames=%d error=%s",
                       m_frameCount,
                       message.toUtf8().constData());
    publishStatus(true);
    emit finished(false, m_options.outputPath, message);
    m_finishEmitted = true;
    deleteLater();
}

void RecordingController::handleWriterFinished(bool ok, const QString &error)
{
    if (!m_stopping) {
        if (!ok) {
            fail(error);
        }
        return;
    }
    completeStop(ok, error);
}

void RecordingController::completeStop(bool ok, const QString &error)
{
    if (m_finishEmitted) {
        return;
    }
    m_finishEmitted = true;
    markshot::debugLog("recording",
                       "【录制】【结束】ok=%d frames=%d output=%s error=%s",
                       ok ? 1 : 0,
                       m_frameCount,
                       m_options.outputPath.toUtf8().constData(),
                       error.toUtf8().constData());
    publishStatus(true);
    emit finished(ok, m_options.outputPath, error);
    deleteLater();
}

RecordingAsyncWriter *RecordingController::asyncWriter() const
{
    return dynamic_cast<RecordingAsyncWriter *>(m_writer.get());
}

void RecordingController::publishStatus(bool force)
{
    if (m_statusThrottler.shouldPublish(force)) {
        emit statusChanged();
    }
}

}  // namespace markshot::recording
