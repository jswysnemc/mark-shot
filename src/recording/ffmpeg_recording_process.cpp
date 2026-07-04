#include "recording/ffmpeg_recording_process.h"

#include <QDir>
#include <QFileInfo>
#include <QImage>
#include <QStandardPaths>

#include <algorithm>

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

    m_process.start(executable, arguments, QIODevice::ReadWrite);
    if (!m_process.waitForStarted(3000)) {
        if (error) {
            *error = QStringLiteral("FFmpeg failed to start: %1").arg(m_process.errorString());
        }
        return false;
    }
    return true;
}

/**
 * 【录制】【FFmpeg写入】写入一帧 QImage。
 * @param frame 需要写入的图像。
 * @param error 输出错误信息。
 * @return 写入成功时返回 true。
 */
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

    const RecordingBgraFrame bytes = m_frameConverter.convertToBgra(frame, m_frameSize, error);
    if (!bytes.data || bytes.size <= 0) {
        return false;
    }
    return writeBgraBytes(bytes, error);
}

/**
 * 【录制】【FFmpeg写入】写入一帧录制样本。
 * @param sample 录制帧样本。
 * @param error 输出错误信息。
 * @return 写入成功时返回 true。
 */
bool FfmpegRecordingProcess::writeFrame(const RecordingFrameSample &sample, QString *error)
{
    if (error) {
        error->clear();
    }
    if (sample.bgra.isValid() && sample.bgra.size == m_frameSize) {
        const int rowBytes = m_frameSize.width() * 4;
        if (sample.bgra.stride == rowBytes) {
            return writeBgraBytes({sample.bgra.bytes.constData(), sample.bgra.bytes.size()}, error);
        }
    }
    if (!sample.image.isNull()) {
        return writeFrame(sample.image, error);
    }
    if (error) {
        *error = QStringLiteral("Cannot write an empty recording frame");
    }
    return false;
}

/**
 * 【录制】【FFmpeg写入】写入连续 BGRA 原始字节。
 * @param bytes BGRA 字节视图。
 * @param error 输出错误信息。
 * @return 写入成功时返回 true。
 */
bool FfmpegRecordingProcess::writeBgraBytes(RecordingBgraFrame bytes, QString *error)
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

    qsizetype offset = 0;
    constexpr qint64 kWriteChunkBytes = 1024 * 1024;
    constexpr qint64 kMaxPendingBytes = 2 * 1024 * 1024;
    constexpr int kWriteDrainTimeoutMs = 5000;
    while (offset < bytes.size) {
        const qint64 chunkBytes = std::min<qint64>(bytes.size - offset, kWriteChunkBytes);
        const qint64 written = m_process.write(bytes.data + offset, chunkBytes);
        if (written < 0) {
            if (error) {
                *error = QStringLiteral("Failed to write video frame to FFmpeg: %1\n%2")
                             .arg(m_process.errorString(), stderrTail());
            }
            return false;
        }
        if (written == 0 && !m_process.waitForBytesWritten(kWriteDrainTimeoutMs)) {
            if (error) {
                *error = QStringLiteral("Timed out while writing video frame to FFmpeg: %1")
                             .arg(stderrTail());
            }
            return false;
        }
        offset += written;
        while (m_process.bytesToWrite() > kMaxPendingBytes) {
            if (!m_process.waitForBytesWritten(kWriteDrainTimeoutMs)) {
                if (error) {
                    *error = QStringLiteral("Timed out while writing video frame to FFmpeg: %1")
                                 .arg(stderrTail());
                }
                return false;
            }
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

QString FfmpegRecordingProcess::stderrTail()
{
    m_stderr.append(m_process.readAllStandardError());
    constexpr qsizetype kMaxStderrBytes = 32768;
    if (m_stderr.size() > kMaxStderrBytes) {
        m_stderr.remove(0, m_stderr.size() - kMaxStderrBytes);
    }
    const QString text = QString::fromLocal8Bit(m_stderr).trimmed();
    return text.isEmpty() ? QStringLiteral("no FFmpeg error output") : text;
}

}  // namespace markshot::recording
