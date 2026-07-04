#pragma once

#include "recording/recording_capture_stream.h"
#include "recording/recording_options.h"

#include <memory>

namespace markshot::recording {

/**
 * 创建 Windows Graphics Capture 录制采集流。
 * @param options 录制配置。
 * @param parent 父对象。
 * @return 支持当前系统时返回采集流，否则返回空。
 */
std::unique_ptr<RecordingCaptureStream> createWindowsWgcRecordingCaptureStream(const RecordingOptions &options,
                                                                               QObject *parent = nullptr);

}  // namespace markshot::recording
