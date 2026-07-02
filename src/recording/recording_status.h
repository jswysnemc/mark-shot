#pragma once

#include "recording/recording_options.h"

#include <QString>

namespace markshot::recording {

struct RecordingStatus {
    bool active = false;
    RecordingMode mode = RecordingMode::Gif;
    int fps = 0;
    int frameCount = 0;
    qint64 elapsedMs = 0;
    QString outputPath;
};

/**
 * 返回录制模式名称。
 * @param mode 录制模式。
 * @return 用于界面和命令行输出的模式名称。
 */
QString recordingModeName(RecordingMode mode);

}  // namespace markshot::recording
