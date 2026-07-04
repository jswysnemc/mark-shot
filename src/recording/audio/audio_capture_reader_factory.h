#pragma once

#include <QString>

#include <memory>

namespace markshot::recording {

class AudioCaptureReader;

/**
 * 判断当前构建是否支持录制音频采集。
 * @return 支持录音采集时返回 true。
 */
bool recordingAudioCaptureAvailable();

/**
 * 返回当前构建不支持录音采集时展示给用户的说明文本。
 * @return 不支持录音采集的说明文本。
 */
QString recordingAudioUnavailableText();

/**
 * 创建当前平台的音频采集器。
 * @return 音频采集器实例；当前平台不支持时返回空指针。
 */
std::unique_ptr<AudioCaptureReader> createPlatformAudioCaptureReader();

}  // namespace markshot::recording
