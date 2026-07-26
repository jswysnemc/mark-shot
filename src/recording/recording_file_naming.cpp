#include "recording/recording_file_naming.h"

#include "recording/recording_storage_config.h"

#include <QDateTime>
#include <QDir>
#include <QFileInfo>

namespace markshot::recording {
namespace {

/**
 * 返回录制模式与容器对应的文件扩展名。
 * @param mode 录制模式。
 * @param container 视频容器格式。
 * @return 不含点号的扩展名。
 */
QString extensionForMode(RecordingMode mode, RecordingContainerFormat container)
{
    return mode == RecordingMode::Gif ? QStringLiteral("gif")
                                      : recordingContainerExtension(container);
}

}  // namespace

QString defaultRecordingPath(RecordingMode mode, RecordingContainerFormat container)
{
    return defaultRecordingPathInDirectory(recordingDirectoryForMode(mode), mode, container);
}

QString defaultRecordingPathInDirectory(const QString &directory,
                                        RecordingMode mode,
                                        RecordingContainerFormat container)
{
    const QString timestamp = QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-HHmmss"));
    const QString targetDirectory = directory.trimmed().isEmpty()
        ? recordingDirectoryForMode(mode)
        : directory.trimmed();
    QDir().mkpath(targetDirectory);
    return QDir(targetDirectory)
        .filePath(QStringLiteral("mark-shot-recording-%1.%2")
                      .arg(timestamp, extensionForMode(mode, container)));
}

QString normalizedRecordingPath(QString path, RecordingMode mode, RecordingContainerFormat container)
{
    path = path.trimmed();
    if (path.isEmpty()) {
        return defaultRecordingPath(mode, container);
    }

    const QString expected = extensionForMode(mode, container);
    QFileInfo info(path);
    if (info.suffix().compare(expected, Qt::CaseInsensitive) == 0) {
        return path;
    }

    // 已带同类容器扩展名时替换，避免生成 sample.mp4.mkv 之类的路径
    const QString suffix = info.suffix().toLower();
    if (suffix == QStringLiteral("mp4") || suffix == QStringLiteral("mkv")
        || suffix == QStringLiteral("gif")) {
        const QString base = path.left(path.size() - suffix.size() - 1);
        return QStringLiteral("%1.%2").arg(base, expected);
    }
    return QStringLiteral("%1.%2").arg(path, expected);
}

}  // namespace markshot::recording
