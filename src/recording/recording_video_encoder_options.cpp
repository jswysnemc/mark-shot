#include "recording/recording_video_encoder_options.h"

#include <QtGlobal>

namespace markshot::recording {
namespace {

/**
 * 【录制】【软件编码】创建指定的软件编码候选。
 * @param id FFmpeg 编码器名称。
 * @return 软件编码配置。
 */
RecordingVideoEncoderOptions softwareEncoder(const QString &id)
{
    return {
        id,
        id,
        false,
    };
}

/**
 * 创建硬件编码候选配置。
 * @param id FFmpeg 编码器名称。
 * @return 硬件编码配置。
 */
RecordingVideoEncoderOptions hardwareEncoder(const QString &id)
{
    return {id, id, true};
}

/**
 * 判断是否禁用硬件编码候选。
 * @return 环境变量要求仅软件编码时返回 true。
 */
bool hardwareEncodersDisabled()
{
    return qEnvironmentVariableIsSet("MARK_SHOT_RECORDING_SW_ENCODER");
}

}  // namespace

QVector<RecordingVideoEncoderOptions> recordingVideoEncoderCandidates(const RecordingOptions &options,
                                                                      int fps)
{
    Q_UNUSED(options)
    Q_UNUSED(fps)
    QVector<RecordingVideoEncoderOptions> candidates;
    if (!hardwareEncodersDisabled()) {
        // 1. 只挑选接受系统内存帧输入的硬件编码器，打开失败时自动回退软件编码
#if defined(Q_OS_WIN)
        candidates.append(hardwareEncoder(QStringLiteral("h264_nvenc")));
        candidates.append(hardwareEncoder(QStringLiteral("h264_amf")));
        candidates.append(hardwareEncoder(QStringLiteral("h264_mf")));
#else
        candidates.append(hardwareEncoder(QStringLiteral("h264_nvenc")));
#endif
    }
    // 2. 优先使用画质更好的 libx264
    candidates.append(softwareEncoder(QStringLiteral("libx264")));
    // 3. mpeg4 是 FFmpeg 原生编码器，Fedora ffmpeg-free 默认提供该候选
    candidates.append(softwareEncoder(QStringLiteral("mpeg4")));
    return candidates;
}

}  // namespace markshot::recording
