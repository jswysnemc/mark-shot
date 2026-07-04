#include "recording/recording_file_naming.h"

#include "recording/recording_storage_config.h"

#include <QDateTime>
#include <QDir>
#include <QFileInfo>

namespace markshot::recording {
namespace {

/**
 * 返回录制模式对应的文件扩展名。
 * @param mode 录制模式。
 * @return 不含点号的扩展名。
 */
QString extensionForMode(RecordingMode mode)
{
    return mode == RecordingMode::Gif ? QStringLiteral("gif") : QStringLiteral("mp4");
}

}  // namespace

QString defaultRecordingPath(RecordingMode mode)
{
    return defaultRecordingPathInDirectory(recordingDirectoryForMode(mode), mode);
}

QString defaultRecordingPathInDirectory(const QString &directory, RecordingMode mode)
{
    const QString timestamp = QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-HHmmss"));
    const QString targetDirectory = directory.trimmed().isEmpty()
        ? recordingDirectoryForMode(mode)
        : directory.trimmed();
    QDir().mkpath(targetDirectory);
    return QDir(targetDirectory)
        .filePath(QStringLiteral("mark-shot-recording-%1.%2").arg(timestamp, extensionForMode(mode)));
}

QString normalizedRecordingPath(QString path, RecordingMode mode)
{
    path = path.trimmed();
    if (path.isEmpty()) {
        return defaultRecordingPath(mode);
    }

    const QString expected = extensionForMode(mode);
    QFileInfo info(path);
    if (info.suffix().compare(expected, Qt::CaseInsensitive) == 0) {
        return path;
    }
    return QStringLiteral("%1.%2").arg(path, expected);
}

}  // namespace markshot::recording
