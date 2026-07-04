#include "recording/video_recording_writer.h"

#include "debug_log.h"

#include <QDir>
#include <QFileInfo>
#include <QImage>

#include <algorithm>
#include <utility>

namespace markshot::recording {

VideoRecordingWriter::VideoRecordingWriter(RecordingOptions options)
    : m_options(std::move(options))
{
}

bool VideoRecordingWriter::start(QSize frameSize, int fps, QString *error)
{
    if (error) {
        error->clear();
    }
    const QFileInfo outputInfo(m_options.outputPath);
    if (!QDir().mkpath(outputInfo.absolutePath())) {
        if (error) {
            *error = QStringLiteral("Cannot create output directory");
        }
        return false;
    }

    m_frameSize = frameSize;
    m_fps = std::max(1, fps);

    m_candidates = recordingVideoEncoderCandidates(m_options, m_fps);
    m_candidateIndex = 0;
    m_writtenFrames = 0;
    m_lastSample = {};
    m_pacer.reset(m_fps);
    return startCandidate(m_candidateIndex, error);
}

bool VideoRecordingWriter::writeFrame(const RecordingFrameSample &sample, QString *error)
{
    if (writeSampleWithPacing(sample, error)) {
        return true;
    }

    if (!canFallbackAfterWriteFailure(sample) || !startNextCandidate(error)) {
        return false;
    }
    markshot::debugLog("recording",
                       "【录制】【编码器回退】retry current frame timestamp_ms=%lld written_frames=%d",
                       static_cast<long long>(sample.timestampMs),
                       m_writtenFrames);
    m_pacer.reset(m_fps);
    m_lastSample = {};
    m_writtenFrames = 0;
    return writeSampleWithPacing(sample, error);
}

bool VideoRecordingWriter::finish(QString *error)
{
    return m_libavProcess.finish(error);
}

void VideoRecordingWriter::cancel()
{
    m_libavProcess.cancel();
}

bool VideoRecordingWriter::startCandidate(int candidateIndex, QString *error)
{
    if (candidateIndex < 0 || candidateIndex >= m_candidates.size()) {
        if (error) {
            *error = QStringLiteral("No video encoder candidate is available");
        }
        return false;
    }

    const RecordingVideoEncoderOptions &candidate = m_candidates.at(candidateIndex);
    markshot::debugLog("recording",
                       "【录制】【库内编码】start encoder=%s hardware=%d",
                       candidate.id.toUtf8().constData(),
                       candidate.hardware ? 1 : 0);
    return m_libavProcess.start(m_options, candidate, m_frameSize, m_fps, error);
}

bool VideoRecordingWriter::startNextCandidate(QString *error)
{
    const QString previousError = error ? *error : QString();
    QString lastError = previousError;
    cancel();
    for (int index = m_candidateIndex + 1; index < m_candidates.size(); ++index) {
        QString candidateError;
        if (startCandidate(index, &candidateError)) {
            markshot::debugLog("recording",
                               "【录制】【编码器回退】from=%s to=%s reason=%s",
                               m_candidates.at(m_candidateIndex).id.toUtf8().constData(),
                               m_candidates.at(index).id.toUtf8().constData(),
                               previousError.toUtf8().constData());
            m_candidateIndex = index;
            if (error) {
                error->clear();
            }
            return true;
        }
        markshot::debugLog("recording",
                           "【录制】【编码器回退】candidate=%s failed=%s",
                           m_candidates.at(index).id.toUtf8().constData(),
                           candidateError.toUtf8().constData());
        lastError = candidateError;
    }
    if (error && !lastError.isEmpty()) {
        *error = lastError;
    }
    return false;
}

bool VideoRecordingWriter::canFallbackAfterWriteFailure(const RecordingFrameSample &sample) const
{
    constexpr qint64 kStartupFallbackWindowMs = 3000;
    constexpr qint64 kSlowStartupFallbackWindowMs = 10000;
    const int startupFrameLimit = std::max(3, m_fps / 2);
    return m_writtenFrames == 0
        || sample.timestampMs <= kStartupFallbackWindowMs
        || (sample.timestampMs <= kSlowStartupFallbackWindowMs
            && m_writtenFrames <= startupFrameLimit);
}

bool VideoRecordingWriter::writeSampleWithPacing(const RecordingFrameSample &sample, QString *error)
{
    if (!sample.hasFrameData()) {
        if (error) {
            *error = QStringLiteral("Cannot write an empty recording frame");
        }
        return false;
    }

    const int duplicates = m_pacer.duplicatesBefore(sample.timestampMs);
    if (m_lastSample.hasFrameData()) {
        for (int i = 0; i < duplicates; ++i) {
            if (!m_libavProcess.writeFrame(m_lastSample, error)) {
                return false;
            }
            ++m_writtenFrames;
        }
    }

    if (!m_libavProcess.writeFrame(sample, error)) {
        return false;
    }
    m_lastSample = sample;
    ++m_writtenFrames;
    return true;
}

}  // namespace markshot::recording
