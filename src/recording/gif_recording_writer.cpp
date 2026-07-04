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

    return m_process.start(m_options.outputPath, frameSize, fps, error);
}

bool GifRecordingWriter::writeFrame(const RecordingFrameSample &sample, QString *error)
{
    return m_process.writeFrame(sample, error);
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
