#include "recording/recording_storage_config.h"

#include "app_config_store.h"
#include "config_value.h"

#include <QDir>
#include <QJsonValue>
#include <QStandardPaths>

namespace markshot::recording {
namespace {

/**
 * 返回录制默认存储根目录。
 * @return 默认存储根目录。
 */
QString defaultRecordingRootDirectory()
{
    QString pictures = QStandardPaths::writableLocation(QStandardPaths::PicturesLocation);
    if (pictures.isEmpty()) {
        pictures = QDir::home().filePath(QStringLiteral("Pictures"));
    }
    return QDir(pictures).filePath(QStringLiteral("mark-shot"));
}

/**
 * 展开用户目录缩写。
 * @param path 用户输入路径。
 * @return 展开后的路径。
 */
QString expandHomePath(QString path)
{
    path = path.trimmed();
    if (path == QStringLiteral("~")) {
        return QDir::homePath();
    }
    if (path.startsWith(QStringLiteral("~/"))) {
        return QDir::home().filePath(path.mid(2));
    }
    return path;
}

/**
 * 从对象中读取第一个字符串字段。
 * @param object JSON 对象。
 * @param keys 候选字段。
 * @return 字符串字段。
 */
QString stringForKeys(const QJsonObject &object, const QStringList &keys)
{
    const QJsonValue value = markshot::config::valueForKeys(object, keys);
    return value.isString() ? value.toString().trimmed() : QString();
}

}  // namespace

RecordingStorageConfig defaultRecordingStorageConfig()
{
    const QDir root(defaultRecordingRootDirectory());
    RecordingStorageConfig config;
    config.videoDirectory = root.filePath(QStringLiteral("videos"));
    config.gifDirectory = root.filePath(QStringLiteral("gifs"));
    return config;
}

RecordingStorageConfig recordingStorageConfigFromRoot(const QJsonObject &root)
{
    RecordingStorageConfig config = defaultRecordingStorageConfig();
    const QJsonObject recording = markshot::config::firstObjectValue(root,
                                                                     {QStringLiteral("recording"),
                                                                      QStringLiteral("recorder")});
    const QJsonObject storage = markshot::config::firstObjectValue(recording,
                                                                   {QStringLiteral("storage"),
                                                                    QStringLiteral("output"),
                                                                    QStringLiteral("save")});

    const QString videoDirectory = stringForKeys(storage,
                                                 {QStringLiteral("videoDirectory"),
                                                  QStringLiteral("videosDirectory"),
                                                  QStringLiteral("videoDir"),
                                                  QStringLiteral("videos"),
                                                  QStringLiteral("video")});
    const QString gifDirectory = stringForKeys(storage,
                                               {QStringLiteral("gifDirectory"),
                                                QStringLiteral("gifsDirectory"),
                                                QStringLiteral("gifDir"),
                                                QStringLiteral("gifs"),
                                                QStringLiteral("gif")});
    config.videoDirectory = normalizedRecordingDirectory(videoDirectory, config.videoDirectory);
    config.gifDirectory = normalizedRecordingDirectory(gifDirectory, config.gifDirectory);
    return config;
}

RecordingStorageConfig configuredRecordingStorageConfig()
{
    bool ok = false;
    const QJsonObject root = markshot::readAppConfigRoot(&ok);
    Q_UNUSED(ok);
    return recordingStorageConfigFromRoot(root);
}

QString recordingDirectoryForMode(RecordingMode mode)
{
    const RecordingStorageConfig config = configuredRecordingStorageConfig();
    return mode == RecordingMode::Gif ? config.gifDirectory : config.videoDirectory;
}

QString normalizedRecordingDirectory(QString path, const QString &fallback)
{
    path = expandHomePath(path);
    if (path.isEmpty()) {
        path = fallback;
    }
    return QDir::cleanPath(path);
}

}  // namespace markshot::recording
