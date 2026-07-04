#pragma once

#include "recording/audio/audio_capture_sample.h"

#include <QString>

#include <mutex>

struct AVFormatContext;

namespace markshot::recording {

class LibavAudioEncoder final {
public:
    LibavAudioEncoder();
    ~LibavAudioEncoder();

    /**
     * 初始化音频编码器和输出流。
     * @param formatContext 输出容器上下文。
     * @param sampleRate 采样率。
     * @param error 输出错误信息。
     * @return 初始化成功时返回 true。
     */
    bool open(AVFormatContext *formatContext, int sampleRate, QString *error);

    /**
     * 读取每个音频块需要的 PCM 字节数。
     * @return 音频块字节数。
     */
    int frameBytes() const;

    /**
     * 编码并写入一个音频块。
     * @param sample 音频采集样本。
     * @param writeMutex 容器写入互斥锁。
     * @param error 输出错误信息。
     * @return 编码写入成功时返回 true。
     */
    bool encode(const AudioCaptureSample &sample, std::mutex &writeMutex, QString *error);

    /**
     * 冲刷音频编码器。
     * @param writeMutex 容器写入互斥锁。
     * @param error 输出错误信息。
     * @return 冲刷成功时返回 true。
     */
    bool flush(std::mutex &writeMutex, QString *error);

    /**
     * 释放音频编码资源。
     * @return 无返回值。
     */
    void close();

private:
    class Private;
    Private *d = nullptr;
};

}  // namespace markshot::recording
