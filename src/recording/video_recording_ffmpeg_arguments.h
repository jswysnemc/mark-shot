#pragma once

#include "recording/recording_options.h"
#include "recording/recording_video_encoder_options.h"

#include <QSize>
#include <QStringList>

namespace markshot::recording {

/**
 * 构建视频录制 FFmpeg 参数。
 * @param options 录制配置。
 * @param frameSize 帧尺寸。
 * @param fps 目标帧率。
 * @param audioArguments 音频输入参数。
 * @param encoder 编码器配置。
 * @return FFmpeg 参数列表。
 */
QStringList buildVideoRecordingFfmpegArguments(const RecordingOptions &options,
                                               QSize frameSize,
                                               int fps,
                                               const QStringList &audioArguments,
                                               const RecordingVideoEncoderOptions &encoder);

}  // namespace markshot::recording
