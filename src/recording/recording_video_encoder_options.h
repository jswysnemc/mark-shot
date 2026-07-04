#pragma once

#include "recording/recording_options.h"

#include <QString>
#include <QVector>

namespace markshot::recording {

struct RecordingVideoEncoderOptions {
    QString id;
    QString label;
    bool hardware = false;
};

/**
 * 构建视频编码候选列表。
 * @param options 录制配置。
 * @param fps 目标帧率。
 * @return 按优先级排序的编码候选，最后总是软件编码回退。
 */
QVector<RecordingVideoEncoderOptions> recordingVideoEncoderCandidates(const RecordingOptions &options,
                                                                      int fps);

}  // namespace markshot::recording
