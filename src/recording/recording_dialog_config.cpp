#include "recording/recording_dialog_config.h"

#include "app_config_store.h"
#include "config_value.h"
#include "recording/recording_file_naming.h"

#include <QFileInfo>
#include <QJsonValue>

#include <algorithm>

namespace markshot::recording {
namespace {

/**
 * 把录制范围转换为配置文本。
 * @param scope 录制范围。
 * @return 配置文本。
 */
QString scopeName(RecordingScope scope)
{
    return scope == RecordingScope::Display ? QStringLiteral("display") : QStringLiteral("region");
}

/**
 * 从配置文本读取录制范围。
 * @param text 配置文本。
 * @return 录制范围。
 */
RecordingScope scopeFromName(QString text)
{
    text = text.trimmed().toLower();
    return text == QStringLiteral("display") ? RecordingScope::Display : RecordingScope::Region;
}

/**
 * 从配置文本读取采集后端。
 * @param text 配置文本。
 * @return 采集后端。
 */
RecordingCaptureBackend backendFromName(QString text)
{
    text = text.trimmed().toLower();
    if (text == QStringLiteral("wlroots") || text == QStringLiteral("wlroots-screencopy")) {
        return RecordingCaptureBackend::Wlroots;
    }
    if (text == QStringLiteral("pipewire") || text == QStringLiteral("portal")) {
        return RecordingCaptureBackend::PipeWire;
    }
    if (text == QStringLiteral("windows-wgc")
        || text == QStringLiteral("wgc")
        || text == QStringLiteral("windows-graphics-capture")) {
        return RecordingCaptureBackend::WindowsWgc;
    }
    if (text == QStringLiteral("polling") || text == QStringLiteral("fallback")) {
        return RecordingCaptureBackend::Polling;
    }
    return RecordingCaptureBackend::Auto;
}

/**
 * 写入整数配置并限制范围。
 * @param value JSON 值。
 * @param fallback 默认值。
 * @param minimum 最小值。
 * @param maximum 最大值。
 * @return 限制后的整数。
 */
int boundedInt(QJsonValue value, int fallback, int minimum, int maximum)
{
    const int number = value.isDouble() ? value.toInt(fallback) : fallback;
    return std::clamp(number, minimum, maximum);
}

/**
 * 判断旧版模式文本是否对应指定录制模式。
 * @param text 旧版模式文本。
 * @param mode 录制模式。
 * @return 匹配时返回 true。
 */
bool legacyModeMatches(QString text, RecordingMode mode)
{
    text = text.trimmed().toLower();
    if (text.isEmpty()) {
        return true;
    }
    if (mode == RecordingMode::Gif) {
        return text == QStringLiteral("gif");
    }
    return text == QStringLiteral("video");
}

/**
 * 读取指定模式的独立帧率配置。
 * @param dialog 录制对话框配置对象。
 * @param key 独立帧率配置键。
 * @param mode 录制模式。
 * @param fallback 默认帧率。
 * @return 限制后的帧率。
 */
int fpsForModeFromDialogConfig(const QJsonObject &dialog,
                               const QString &key,
                               RecordingMode mode,
                               int fallback)
{
    if (dialog.contains(key)) {
        return boundedInt(dialog.value(key), fallback, 1, 120);
    }
    if (dialog.contains(QStringLiteral("fps"))
        && legacyModeMatches(dialog.value(QStringLiteral("mode")).toString(), mode)) {
        return boundedInt(dialog.value(QStringLiteral("fps")), fallback, 1, 120);
    }
    return fallback;
}

/**
 * 从持久化配置读取输出目录并生成新的默认文件名。
 * @param dialog 录制对话框配置对象。
 * @param mode 录制模式。
 * @return 带新时间戳的输出路径。
 */
QString generatedOutputPathFromDialogConfig(const QJsonObject &dialog, RecordingMode mode)
{
    QString directory = dialog.value(QStringLiteral("outputDirectory")).toString().trimmed();
    if (directory.isEmpty()) {
        const QString legacyPath = dialog.value(QStringLiteral("outputPath")).toString().trimmed();
        if (!legacyPath.isEmpty()) {
            directory = QFileInfo(normalizedRecordingPath(legacyPath, mode)).absolutePath();
        }
    }
    return directory.isEmpty()
        ? defaultRecordingPath(mode)
        : defaultRecordingPathInDirectory(directory, mode);
}

}  // namespace

RecordingDialogConfig recordingDialogConfigFromRoot(const QJsonObject &root, RecordingMode mode)
{
    const QJsonObject recording = markshot::config::firstObjectValue(root,
                                                                     {QStringLiteral("recording"),
                                                                      QStringLiteral("recorder")});
    const QJsonObject dialog = markshot::config::firstObjectValue(recording,
                                                                  {QStringLiteral("dialog"),
                                                                   QStringLiteral("startDialog"),
                                                                   QStringLiteral("ui")});

    RecordingDialogConfig config;
    config.mode = mode;
    config.scope = scopeFromName(dialog.value(QStringLiteral("scope")).toString());
    config.backend = backendFromName(dialog.value(QStringLiteral("backend")).toString());
    config.videoFps = fpsForModeFromDialogConfig(dialog,
                                                 QStringLiteral("videoFps"),
                                                 RecordingMode::Video,
                                                 30);
    config.gifFps = fpsForModeFromDialogConfig(dialog,
                                               QStringLiteral("gifFps"),
                                               RecordingMode::Gif,
                                               12);
    config.fps = config.mode == RecordingMode::Gif ? config.gifFps : config.videoFps;
    config.includeAudio = dialog.value(QStringLiteral("includeAudio")).toBool(false);
    config.outputPath = generatedOutputPathFromDialogConfig(dialog, config.mode);
    config.displayKey = dialog.value(QStringLiteral("displayKey")).toString().trimmed();
    return config;
}

RecordingDialogConfig configuredRecordingDialogConfig(RecordingMode mode)
{
    bool ok = false;
    const QJsonObject root = markshot::readAppConfigRoot(&ok);
    Q_UNUSED(ok);
    return recordingDialogConfigFromRoot(root, mode);
}

bool saveRecordingDialogConfig(const RecordingDialogConfig &config, QString *error)
{
    bool ok = false;
    const QJsonObject root = markshot::readAppConfigRoot(&ok);
    const QJsonObject recording = markshot::config::firstObjectValue(root,
                                                                     {QStringLiteral("recording"),
                                                                      QStringLiteral("recorder")});
    QJsonObject dialog = markshot::config::firstObjectValue(recording,
                                                            {QStringLiteral("dialog"),
                                                             QStringLiteral("startDialog"),
                                                             QStringLiteral("ui")});
    Q_UNUSED(ok);

    dialog.remove(QStringLiteral("mode"));
    dialog.remove(QStringLiteral("fps"));
    dialog.insert(QStringLiteral("scope"), scopeName(config.scope));
    dialog.insert(QStringLiteral("backend"), recordingCaptureBackendName(config.backend));
    dialog.insert(config.mode == RecordingMode::Gif ? QStringLiteral("gifFps") : QStringLiteral("videoFps"),
                  config.fps);
    dialog.insert(QStringLiteral("includeAudio"), config.includeAudio);
    dialog.insert(QStringLiteral("outputDirectory"), QFileInfo(config.outputPath).absolutePath());
    dialog.insert(QStringLiteral("displayKey"), config.displayKey);
    return markshot::writeAppConfigValue({QStringLiteral("recording"),
                                          QStringLiteral("dialog")},
                                         dialog,
                                         error);
}

RecordingDialogConfig recordingDialogConfigFromOptions(const RecordingOptions &options)
{
    RecordingDialogConfig config;
    config.mode = options.mode;
    config.scope = options.scope;
    config.backend = options.captureBackend;
    config.fps = options.fps;
    config.videoFps = options.mode == RecordingMode::Video ? options.fps : 30;
    config.gifFps = options.mode == RecordingMode::Gif ? options.fps : 12;
    config.includeAudio = options.includeAudio;
    config.outputPath = options.outputPath.trimmed().isEmpty()
        ? defaultRecordingPath(options.mode)
        : defaultRecordingPathInDirectory(QFileInfo(options.outputPath).absolutePath(), options.mode);
    config.displayKey = recordingDisplayPersistenceKey(options.display);
    return config;
}

QString recordingDisplayPersistenceKey(const DisplaySource &source)
{
    if (source.allOutputs) {
        return QStringLiteral("all");
    }
    if (!source.outputName.trimmed().isEmpty()) {
        return QStringLiteral("output:%1").arg(source.outputName.trimmed());
    }
    if (!source.screenName.trimmed().isEmpty()) {
        return QStringLiteral("screen:%1").arg(source.screenName.trimmed());
    }
    return QStringLiteral("geometry:%1,%2,%3,%4")
        .arg(source.geometry.x())
        .arg(source.geometry.y())
        .arg(source.geometry.width())
        .arg(source.geometry.height());
}

}  // namespace markshot::recording
