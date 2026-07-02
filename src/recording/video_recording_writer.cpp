#include "recording/video_recording_writer.h"

#include "recording/audio_capture_options.h"

#include <QDir>
#include <QFileInfo>

#include <utility>

namespace markshot::recording {

VideoRecordingWriter::VideoRecordingWriter(RecordingOptions options)
    : m_options(std::move(options))
{
}

bool VideoRecordingWriter::start(QSize frameSize, int fps, QString *error)
{
    const QFileInfo outputInfo(m_options.outputPath);
    if (!QDir().mkpath(outputInfo.absolutePath())) {
        if (error) {
            *error = QStringLiteral("Cannot create output directory");
        }
        return false;
    }

    const QStringList audioArguments = m_options.includeAudio ? defaultAudioInputArguments() : QStringList();
    if (m_options.includeAudio && audioArguments.isEmpty()) {
        if (error) {
            *error = QStringLiteral("Audio recording is not supported on this platform");
        }
        return false;
    }

    const QString sizeText = QStringLiteral("%1x%2").arg(frameSize.width()).arg(frameSize.height());
    QStringList arguments{
        QStringLiteral("-hide_banner"),
        QStringLiteral("-loglevel"),
        QStringLiteral("error"),
        QStringLiteral("-y"),
        QStringLiteral("-f"),
        QStringLiteral("rawvideo"),
        QStringLiteral("-pix_fmt"),
        QStringLiteral("bgra"),
        QStringLiteral("-video_size"),
        sizeText,
        QStringLiteral("-framerate"),
        QString::number(fps),
        QStringLiteral("-i"),
        QStringLiteral("pipe:0"),
    };
    arguments.append(audioArguments);
    arguments << QStringLiteral("-map") << QStringLiteral("0:v");
    if (m_options.includeAudio) {
        arguments << QStringLiteral("-map") << QStringLiteral("1:a") << QStringLiteral("-shortest");
    }
    arguments << QStringLiteral("-c:v")
              << QStringLiteral("libx264")
              << QStringLiteral("-preset")
              << QStringLiteral("veryfast")
              << QStringLiteral("-crf")
              << QStringLiteral("23")
              << QStringLiteral("-pix_fmt")
              << QStringLiteral("yuv420p");
    if (m_options.includeAudio) {
        arguments << QStringLiteral("-c:a") << QStringLiteral("aac");
    }
    arguments << QStringLiteral("-movflags") << QStringLiteral("+faststart") << m_options.outputPath;
    return m_process.start(m_options.ffmpegPath, arguments, frameSize, error);
}

bool VideoRecordingWriter::writeFrame(const QImage &frame, QString *error)
{
    return m_process.writeFrame(frame, error);
}

bool VideoRecordingWriter::finish(QString *error)
{
    return m_process.finish(error);
}

void VideoRecordingWriter::cancel()
{
    m_process.cancel();
}

}  // namespace markshot::recording
