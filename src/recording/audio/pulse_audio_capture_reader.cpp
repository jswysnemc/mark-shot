#include "recording/audio/pulse_audio_capture_reader.h"

#include <QByteArray>

#ifdef HAVE_PULSE_RECORDING
#include <pulse/error.h>
#include <pulse/simple.h>
#endif

namespace markshot::recording {

PulseAudioCaptureReader::PulseAudioCaptureReader() = default;

PulseAudioCaptureReader::~PulseAudioCaptureReader()
{
    stop();
}

/**
 * 初始化 PulseAudio 采集流。
 * @param frameBytes 每个音频块的字节数。
 * @param sampleRate 采样率。
 * @param callback 音频块回调。
 * @param error 输出错误信息。
 * @return 初始化成功时返回 true。
 */
bool PulseAudioCaptureReader::init(int frameBytes,
                                   int sampleRate,
                                   SampleCallback callback,
                                   QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_PULSE_RECORDING
    Q_UNUSED(frameBytes)
    Q_UNUSED(sampleRate)
    Q_UNUSED(callback)
    if (error) {
        *error = QStringLiteral("PulseAudio recording support is not linked");
    }
    return false;
#else
    if (frameBytes <= 0 || sampleRate <= 0 || !callback) {
        if (error) {
            *error = QStringLiteral("invalid PulseAudio capture parameters");
        }
        return false;
    }

    pa_sample_spec sampleSpec;
    sampleSpec.format = PA_SAMPLE_FLOAT32LE;
    sampleSpec.rate = static_cast<uint32_t>(sampleRate);
    sampleSpec.channels = 2;

    pa_channel_map channelMap;
    pa_channel_map_init_stereo(&channelMap);

    pa_buffer_attr attr;
    attr.maxlength = static_cast<uint32_t>(frameBytes * 4);
    attr.tlength = static_cast<uint32_t>(-1);
    attr.prebuf = static_cast<uint32_t>(-1);
    attr.minreq = static_cast<uint32_t>(-1);
    attr.fragsize = static_cast<uint32_t>(frameBytes * 4);

    int pulseError = 0;
    m_connection = pa_simple_new(nullptr,
                                 "mark-shot",
                                 PA_STREAM_RECORD,
                                 nullptr,
                                 "mark-shot recording",
                                 &sampleSpec,
                                 &channelMap,
                                 &attr,
                                 &pulseError);
    if (!m_connection) {
        if (error) {
            *error = QStringLiteral("failed to connect PulseAudio: %1")
                         .arg(QString::fromLocal8Bit(pa_strerror(pulseError)));
        }
        return false;
    }

    m_frameBytes = frameBytes;
    m_callback = std::move(callback);
    m_sequence = 0;
    return true;
#endif
}

/**
 * 启动音频采集线程。
 * @return 无返回值。
 */
void PulseAudioCaptureReader::start()
{
    if (m_running || !m_connection || !m_callback || m_frameBytes <= 0) {
        return;
    }
    m_running = true;
    m_thread = std::thread([this] {
        readLoop();
    });
}

/**
 * 停止音频采集线程并释放连接。
 * @return 无返回值。
 */
void PulseAudioCaptureReader::stop()
{
    if (m_running) {
        m_running = false;
        if (m_thread.joinable()) {
            m_thread.join();
        }
    }
#ifdef HAVE_PULSE_RECORDING
    if (m_connection) {
        pa_simple_free(m_connection);
        m_connection = nullptr;
    }
#endif
}

/**
 * 循环读取 PulseAudio 音频块。
 * @return 无返回值。
 */
void PulseAudioCaptureReader::readLoop()
{
#ifdef HAVE_PULSE_RECORDING
    while (m_running) {
        AudioCaptureSample sample;
        sample.pcm.resize(m_frameBytes);
        int pulseError = 0;
        if (pa_simple_read(m_connection, sample.pcm.data(), sample.pcm.size(), &pulseError) < 0) {
            m_running = false;
            break;
        }
        sample.sequence = ++m_sequence;
        m_callback(sample);
    }
#endif
}

}  // namespace markshot::recording
