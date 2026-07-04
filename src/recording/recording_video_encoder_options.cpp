#include "recording/recording_video_encoder_options.h"

#include <QFileInfo>
#include <QProcess>
#include <QProcessEnvironment>
#include <QStandardPaths>

#include <algorithm>

namespace markshot::recording {
namespace {

/**
 * 解析 FFmpeg 可执行文件路径。
 * @param program 配置中的 FFmpeg 路径。
 * @return 可执行文件路径，找不到时返回空。
 */
QString resolvedFfmpegProgram(QString program)
{
    program = program.trimmed();
    if (program.isEmpty()) {
        program = QStringLiteral("ffmpeg");
    }

    if (program.contains(QLatin1Char('/')) || program.contains(QLatin1Char('\\'))) {
        const QFileInfo info(program);
        return info.exists() ? info.absoluteFilePath() : QString();
    }
    return QStandardPaths::findExecutable(program);
}

/**
 * 读取 FFmpeg 编码器清单。
 * @param program FFmpeg 可执行文件。
 * @return 编码器清单文本。
 */
QString ffmpegEncodersText(const QString &program)
{
    const QString executable = resolvedFfmpegProgram(program);
    if (executable.isEmpty()) {
        return {};
    }

    QProcess process;
    process.start(executable,
                  {QStringLiteral("-hide_banner"), QStringLiteral("-encoders")},
                  QIODevice::ReadOnly);
    if (!process.waitForFinished(3000)) {
        process.kill();
        process.waitForFinished(1000);
        return {};
    }
    return QString::fromLocal8Bit(process.readAllStandardOutput())
        + QString::fromLocal8Bit(process.readAllStandardError());
}

/**
 * 判断 FFmpeg 是否包含指定编码器。
 * @param encodersText 编码器清单文本。
 * @param encoderId 编码器标识。
 * @return 包含编码器时返回 true。
 */
bool hasEncoder(const QString &encodersText, const QString &encoderId)
{
    return encodersText.contains(encoderId, Qt::CaseSensitive);
}

/**
 * 判断文件路径是否存在。
 * @param path 文件路径。
 * @return 存在时返回 true。
 */
bool exists(const QString &path)
{
    return QFileInfo::exists(path);
}

/**
 * 读取硬件编码选择。
 * @return auto、software 或具体编码器标识。
 */
QString requestedEncoderMode()
{
    return QProcessEnvironment::systemEnvironment()
        .value(QStringLiteral("MARK_SHOT_VIDEO_ENCODER"),
               QStringLiteral("auto"))
        .trimmed()
        .toLower();
}

/**
 * 创建软件编码回退配置。
 * @param fps 目标帧率。
 * @return 软件编码配置。
 */
RecordingVideoEncoderOptions softwareEncoder(int fps)
{
    const QString videoPreset = fps >= 48 ? QStringLiteral("ultrafast")
                                          : QStringLiteral("veryfast");
    return {
        QStringLiteral("libx264"),
        QStringLiteral("libx264"),
        {},
        {QStringLiteral("-c:v"),
         QStringLiteral("libx264"),
         QStringLiteral("-preset"),
         videoPreset,
         QStringLiteral("-tune"),
         QStringLiteral("zerolatency"),
         QStringLiteral("-crf"),
         QStringLiteral("23"),
         QStringLiteral("-pix_fmt"),
         QStringLiteral("yuv420p")},
        false,
    };
}

/**
 * 创建 VAAPI 编码配置。
 * @return VAAPI 编码配置。
 */
RecordingVideoEncoderOptions vaapiEncoder()
{
    return {
        QStringLiteral("h264_vaapi"),
        QStringLiteral("VAAPI H.264"),
        {QStringLiteral("-vaapi_device"), QStringLiteral("/dev/dri/renderD128")},
        {QStringLiteral("-vf"),
         QStringLiteral("format=nv12,hwupload"),
         QStringLiteral("-c:v"),
         QStringLiteral("h264_vaapi"),
         QStringLiteral("-qp"),
         QStringLiteral("23"),
         QStringLiteral("-profile:v"),
         QStringLiteral("high")},
        true,
    };
}

/**
 * 创建 NVENC 编码配置。
 * @return NVENC 编码配置。
 */
RecordingVideoEncoderOptions nvencEncoder()
{
    return {
        QStringLiteral("h264_nvenc"),
        QStringLiteral("NVENC H.264"),
        {},
        {QStringLiteral("-c:v"),
         QStringLiteral("h264_nvenc"),
         QStringLiteral("-preset"),
         QStringLiteral("p1"),
         QStringLiteral("-tune"),
         QStringLiteral("ll"),
         QStringLiteral("-rc"),
         QStringLiteral("vbr"),
         QStringLiteral("-cq"),
         QStringLiteral("23"),
         QStringLiteral("-pix_fmt"),
         QStringLiteral("yuv420p")},
        true,
    };
}

/**
 * 按用户指定编码器创建候选。
 * @param id 编码器标识。
 * @return 编码器配置，不支持时返回空 id。
 */
RecordingVideoEncoderOptions requestedHardwareEncoder(const QString &id)
{
    if (id == QStringLiteral("h264_vaapi") || id == QStringLiteral("vaapi")) {
        return vaapiEncoder();
    }
    if (id == QStringLiteral("h264_nvenc") || id == QStringLiteral("nvenc")) {
        return nvencEncoder();
    }
    return {};
}

}  // namespace

QVector<RecordingVideoEncoderOptions> recordingVideoEncoderCandidates(const RecordingOptions &options,
                                                                      int fps)
{
    QVector<RecordingVideoEncoderOptions> candidates;
    const QString mode = requestedEncoderMode();
    const QString encodersText = ffmpegEncodersText(options.ffmpegPath);

    if (mode != QStringLiteral("software") && mode != QStringLiteral("libx264")) {
        if (mode != QStringLiteral("auto")) {
            RecordingVideoEncoderOptions requested = requestedHardwareEncoder(mode);
            if (!requested.id.isEmpty() && hasEncoder(encodersText, requested.id)) {
                candidates.append(requested);
            }
        } else {
            if (exists(QStringLiteral("/dev/dri/renderD128"))
                && hasEncoder(encodersText, QStringLiteral("h264_vaapi"))) {
                candidates.append(vaapiEncoder());
            }
            if ((exists(QStringLiteral("/dev/nvidiactl"))
                 || exists(QStringLiteral("/proc/driver/nvidia/version")))
                && hasEncoder(encodersText, QStringLiteral("h264_nvenc"))) {
                candidates.append(nvencEncoder());
            }
        }
    }

    candidates.append(softwareEncoder(fps));
    return candidates;
}

}  // namespace markshot::recording
