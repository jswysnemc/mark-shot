#include "recording/recording_video_encoder_options.h"

namespace markshot::recording {
namespace {

/**
 * 创建软件编码回退配置。
 * @param fps 目标帧率。
 * @return 软件编码配置。
 */
RecordingVideoEncoderOptions softwareEncoder(int fps)
{
    Q_UNUSED(fps)
    return {
        QStringLiteral("libx264"),
        QStringLiteral("libx264"),
        false,
    };
}

}  // namespace

QVector<RecordingVideoEncoderOptions> recordingVideoEncoderCandidates(const RecordingOptions &options,
                                                                      int fps)
{
    Q_UNUSED(options)
    QVector<RecordingVideoEncoderOptions> candidates;
    candidates.append(softwareEncoder(fps));
    return candidates;
}

}  // namespace markshot::recording
