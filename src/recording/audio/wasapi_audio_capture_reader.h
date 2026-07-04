#pragma once

#include "recording/audio/audio_capture_reader.h"

#include <QByteArray>

#include <atomic>
#include <thread>

struct IAudioCaptureClient;
struct IAudioClient;
struct IMMDevice;

namespace markshot::recording {

class WasapiAudioCaptureReader final : public AudioCaptureReader {
public:
    WasapiAudioCaptureReader();
    ~WasapiAudioCaptureReader() override;

    /**
     * 返回默认播放设备的混音采样率。
     * @return 采样率。
     */
    int preferredSampleRate() const override;

    /**
     * 初始化 WASAPI 默认播放设备回环采集。
     * @param frameBytes 每个音频块的字节数。
     * @param sampleRate 采样率。
     * @param callback 音频块回调。
     * @param error 输出错误信息。
     * @return 初始化成功时返回 true。
     */
    bool init(int frameBytes, int sampleRate, SampleCallback callback, QString *error) override;

    /**
     * 启动 WASAPI 音频采集线程。
     * @return 无返回值。
     */
    void start() override;

    /**
     * 停止 WASAPI 音频采集线程并释放设备资源。
     * @return 无返回值。
     */
    void stop() override;

private:
    /**
     * 循环读取 WASAPI 回环音频包。
     * @return 无返回值。
     */
    void readLoop();

    /**
     * 释放 WASAPI COM 接口。
     * @return 无返回值。
     */
    void releaseResources();

    /**
     * 重置音频缓存状态。
     * @return 无返回值。
     */
    void resetBuffers();

    IAudioClient *m_audioClient = nullptr;
    IAudioCaptureClient *m_captureClient = nullptr;
    IMMDevice *m_device = nullptr;
    SampleCallback m_callback;
    QByteArray m_pendingPcm;
    std::thread m_thread;
    std::atomic<bool> m_running{false};
    int m_frameBytes = 0;
    int m_sampleRate = 48000;
    int m_channels = 2;
    int m_bitsPerSample = 32;
    bool m_floatSamples = true;
    bool m_comInitialized = false;
    std::int64_t m_sequence = 0;
};

}  // namespace markshot::recording
