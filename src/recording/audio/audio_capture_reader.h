#pragma once

#include "recording/audio/audio_capture_sample.h"

#include <QString>

#include <functional>

namespace markshot::recording {

class AudioCaptureReader {
public:
    using SampleCallback = std::function<void(const AudioCaptureSample &)>;

    virtual ~AudioCaptureReader() = default;

    /**
     * 返回当前采集后端偏好的采样率。
     * @return 采样率。
     */
    virtual int preferredSampleRate() const
    {
        return 48000;
    }

    /**
     * 初始化音频采集流。
     * @param frameBytes 每个音频块的字节数。
     * @param sampleRate 采样率。
     * @param callback 音频块回调。
     * @param error 输出错误信息。
     * @return 初始化成功时返回 true。
     */
    virtual bool init(int frameBytes, int sampleRate, SampleCallback callback, QString *error) = 0;

    /**
     * 启动音频采集线程。
     * @return 无返回值。
     */
    virtual void start() = 0;

    /**
     * 停止音频采集线程并释放采集资源。
     * @return 无返回值。
     */
    virtual void stop() = 0;
};

}  // namespace markshot::recording
