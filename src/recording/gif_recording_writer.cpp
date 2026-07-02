#include "recording/gif_recording_writer.h"

#include <QDir>
#include <QFileInfo>

#include <utility>

namespace markshot::recording {

GifRecordingWriter::GifRecordingWriter(RecordingOptions options)
    : m_options(std::move(options))
{
}

bool GifRecordingWriter::start(QSize frameSize, int fps, QString *error)
{
    const QFileInfo outputInfo(m_options.outputPath);
    if (!QDir().mkpath(outputInfo.absolutePath())) {
        if (error) {
            *error = QStringLiteral("Cannot create output directory");
        }
        return false;
    }

    const QString sizeText = QStringLiteral("%1x%2").arg(frameSize.width()).arg(frameSize.height());
    const QStringList arguments{
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
        QStringLiteral("-loop"),
        QStringLiteral("0"),
        m_options.outputPath,
    };
    return m_process.start(m_options.ffmpegPath, arguments, frameSize, error);
}

bool GifRecordingWriter::writeFrame(const QImage &frame, QString *error)
{
    return m_process.writeFrame(frame, error);
}

bool GifRecordingWriter::finish(QString *error)
{
    return m_process.finish(error);
}

void GifRecordingWriter::cancel()
{
    m_process.cancel();
}

}  // namespace markshot::recording
