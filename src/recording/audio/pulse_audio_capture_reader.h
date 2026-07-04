#pragma once

#include "recording/audio/audio_capture_sample.h"

#include <QString>

#include <atomic>
#include <functional>
#include <thread>

struct pa_simple;

namespace markshot::recording {

class PulseAudioCaptureReader final {
public:
    using SampleCallback = std::function<void(const AudioCaptureSample &)>;

    PulseAudioCaptureReader();
    ~PulseAudioCaptureReader();

    /**
     * 初始化 PulseAudio 采集流。
     * @param frameBytes 每个音频块的字节数。
     * @param sampleRate 采样率。
     * @param callback 音频块回调。
     * @param error 输出错误信息。
     * @return 初始化成功时返回 true。
     */
    bool init(int frameBytes, int sampleRate, SampleCallback callback, QString *error);

    /**
     * 启动音频采集线程。
     * @return 无返回值。
     */
    void start();

    /**
     * 停止音频采集线程并释放连接。
     * @return 无返回值。
     */
    void stop();

private:
    /**
     * 循环读取 PulseAudio 音频块。
     * @return 无返回值。
     */
    void readLoop();

    pa_simple *m_connection = nullptr;
    SampleCallback m_callback;
    std::thread m_thread;
    std::atomic<bool> m_running{false};
    int m_frameBytes = 0;
    std::int64_t m_sequence = 0;
};

}  // namespace markshot::recording
