#include "recording/ffmpeg_recording_process.h"

#include <QDir>
#include <QFileInfo>
#include <QImage>
#include <QStandardPaths>

namespace markshot::recording {
namespace {

/**
 * 解析 FFmpeg 可执行文件路径。
 * @param program 用户输入的可执行文件。
 * @return 可执行文件路径，找不到时为空。
 */
QString resolvedProgram(QString program)
{
    program = program.trimmed();
    if (program.isEmpty()) {
        program = QStringLiteral("ffmpeg");
    }

    if (program.contains(QLatin1Char('/')) || program.contains(QLatin1Char('\\'))) {
        QFileInfo info(program);
        return info.exists() ? info.absoluteFilePath() : QString();
    }

    return QStandardPaths::findExecutable(program);
}

/**
 * 把图像转换为连续 BGRA 字节。
 * @param frame 输入图像。
 * @param size 目标帧尺寸。
 * @return 可写入 FFmpeg 的原始帧数据。
 */
QByteArray bgraBytes(const QImage &frame, QSize size)
{
    QImage image = frame.convertToFormat(QImage::Format_ARGB32);
    if (image.size() != size) {
        image = image.scaled(size, Qt::IgnoreAspectRatio, Qt::SmoothTransformation)
                    .convertToFormat(QImage::Format_ARGB32);
    }

    QByteArray bytes;
    bytes.reserve(size.width() * size.height() * 4);
    const int rowBytes = size.width() * 4;
    for (int y = 0; y < image.height(); ++y) {
        bytes.append(reinterpret_cast<const char *>(image.constScanLine(y)), rowBytes);
    }
    return bytes;
}

}  // namespace

bool FfmpegRecordingProcess::start(const QString &program,
                                   const QStringList &arguments,
                                   QSize frameSize,
                                   QString *error)
{
    if (error) {
        error->clear();
    }
    const QString executable = resolvedProgram(program);
    if (executable.isEmpty()) {
        if (error) {
            *error = QStringLiteral("FFmpeg executable was not found");
        }
        return false;
    }

    m_frameSize = frameSize;
    m_stderr.clear();
    m_process.setProcessChannelMode(QProcess::SeparateChannels);
    QObject::connect(&m_process, &QProcess::readyReadStandardError, &m_process, [this] {
        m_stderr.append(m_process.readAllStandardError());
        constexpr qsizetype kMaxStderrBytes = 32768;
        if (m_stderr.size() > kMaxStderrBytes) {
            m_stderr.remove(0, m_stderr.size() - kMaxStderrBytes);
        }
    });

    m_process.start(executable, arguments, QIODevice::WriteOnly);
    if (!m_process.waitForStarted(3000)) {
        if (error) {
            *error = QStringLiteral("FFmpeg failed to start: %1").arg(m_process.errorString());
        }
        return false;
    }
    return true;
}

bool FfmpegRecordingProcess::writeFrame(const QImage &frame, QString *error)
{
    if (error) {
        error->clear();
    }
    if (m_process.state() != QProcess::Running) {
        if (error) {
            *error = QStringLiteral("FFmpeg is not running: %1").arg(stderrTail());
        }
        return false;
    }

    const QByteArray bytes = bgraBytes(frame, m_frameSize);
    qsizetype offset = 0;
    while (offset < bytes.size()) {
        const qint64 written = m_process.write(bytes.constData() + offset, bytes.size() - offset);
        if (written < 0) {
            if (error) {
                *error = QStringLiteral("Failed to write video frame to FFmpeg: %1").arg(m_process.errorString());
            }
            return false;
        }
        if (written == 0 && !m_process.waitForBytesWritten(1500)) {
            if (error) {
                *error = QStringLiteral("Timed out while writing video frame to FFmpeg");
            }
            return false;
        }
        offset += written;
        if (m_process.bytesToWrite() > bytes.size() && !m_process.waitForBytesWritten(1500)) {
            if (error) {
                *error = QStringLiteral("Timed out while writing video frame to FFmpeg");
            }
            return false;
        }
    }
    return true;
}

bool FfmpegRecordingProcess::finish(QString *error)
{
    if (error) {
        error->clear();
    }
    if (m_process.state() == QProcess::NotRunning) {
        return true;
    }

    m_process.closeWriteChannel();
    if (!m_process.waitForFinished(180000)) {
        m_process.kill();
        m_process.waitForFinished(3000);
        if (error) {
            *error = QStringLiteral("Timed out while finalizing FFmpeg output");
        }
        return false;
    }

    if (m_process.exitStatus() != QProcess::NormalExit || m_process.exitCode() != 0) {
        if (error) {
            *error = QStringLiteral("FFmpeg failed: %1").arg(stderrTail());
        }
        return false;
    }
    return true;
}

void FfmpegRecordingProcess::cancel()
{
    if (m_process.state() == QProcess::NotRunning) {
        return;
    }
    m_process.kill();
    m_process.waitForFinished(3000);
}

QString FfmpegRecordingProcess::stderrTail() const
{
    const QString text = QString::fromLocal8Bit(m_stderr).trimmed();
    return text.isEmpty() ? QStringLiteral("no FFmpeg error output") : text;
}

}  // namespace markshot::recording
