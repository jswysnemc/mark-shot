#include "recording/audio/wasapi_audio_capture_reader.h"

#include "debug_log.h"

#include <QByteArray>

#include <algorithm>
#include <cstring>
#include <utility>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#define INITGUID
#include <initguid.h>
#include <audioclient.h>
#include <ksmedia.h>
#include <mmdeviceapi.h>
#include <objbase.h>
#endif

namespace markshot::recording {
namespace {

/**
 * 写入错误文本。
 * @param error 输出错误信息。
 * @param text 错误文本。
 * @return 固定返回 false。
 */
bool failWith(QString *error, const QString &text)
{
    if (error) {
        *error = text;
    }
    return false;
}

#ifdef _WIN32

/**
 * 读取默认播放设备的混音采样率。
 * @return 成功时返回设备采样率，失败时返回 48000。
 */
int defaultRenderMixSampleRate()
{
    const HRESULT comResult = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    const bool shouldUninitialize = SUCCEEDED(comResult);
    if (FAILED(comResult) && comResult != RPC_E_CHANGED_MODE) {
        return 48000;
    }

    int sampleRate = 48000;
    IMMDeviceEnumerator *enumerator = nullptr;
    IMMDevice *device = nullptr;
    IAudioClient *audioClient = nullptr;
    WAVEFORMATEX *mixFormat = nullptr;

    HRESULT result = CoCreateInstance(CLSID_MMDeviceEnumerator,
                                      nullptr,
                                      CLSCTX_ALL,
                                      IID_PPV_ARGS(&enumerator));
    if (SUCCEEDED(result) && enumerator) {
        result = enumerator->GetDefaultAudioEndpoint(eRender, eConsole, &device);
    }
    if (SUCCEEDED(result) && device) {
        result = device->Activate(IID_IAudioClient,
                                  CLSCTX_ALL,
                                  nullptr,
                                  reinterpret_cast<void **>(&audioClient));
    }
    if (SUCCEEDED(result) && audioClient) {
        result = audioClient->GetMixFormat(&mixFormat);
    }
    if (SUCCEEDED(result) && mixFormat && mixFormat->nSamplesPerSec > 0) {
        sampleRate = static_cast<int>(mixFormat->nSamplesPerSec);
    }

    if (mixFormat) {
        CoTaskMemFree(mixFormat);
    }
    if (audioClient) {
        audioClient->Release();
    }
    if (device) {
        device->Release();
    }
    if (enumerator) {
        enumerator->Release();
    }
    if (shouldUninitialize) {
        CoUninitialize();
    }
    return sampleRate;
}

/**
 * 把 HRESULT 格式化为十六进制文本。
 * @param result HRESULT 值。
 * @return 十六进制错误文本。
 */
QString hresultText(HRESULT result)
{
    return QStringLiteral("0x%1").arg(static_cast<quint32>(result),
                                     8,
                                     16,
                                     QLatin1Char('0'));
}

/**
 * 判断 WASAPI 格式是否为 32-bit float。
 * @param format WASAPI 音频格式。
 * @return 是 float 格式时返回 true。
 */
bool isFloatFormat(const WAVEFORMATEX *format)
{
    if (!format) {
        return false;
    }
    if (format->wFormatTag == WAVE_FORMAT_IEEE_FLOAT) {
        return true;
    }
    if (format->wFormatTag == WAVE_FORMAT_EXTENSIBLE
        && format->cbSize >= sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX)) {
        const auto *extensible = reinterpret_cast<const WAVEFORMATEXTENSIBLE *>(format);
        return IsEqualGUID(extensible->SubFormat, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT);
    }
    return false;
}

/**
 * 判断 WASAPI 格式是否为整型 PCM。
 * @param format WASAPI 音频格式。
 * @return 是 PCM 格式时返回 true。
 */
bool isPcmFormat(const WAVEFORMATEX *format)
{
    if (!format) {
        return false;
    }
    if (format->wFormatTag == WAVE_FORMAT_PCM) {
        return true;
    }
    if (format->wFormatTag == WAVE_FORMAT_EXTENSIBLE
        && format->cbSize >= sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX)) {
        const auto *extensible = reinterpret_cast<const WAVEFORMATEXTENSIBLE *>(format);
        return IsEqualGUID(extensible->SubFormat, KSDATAFORMAT_SUBTYPE_PCM);
    }
    return false;
}

/**
 * 判断混音格式是否可转换为编码器需要的 stereo float PCM。
 * @param format WASAPI 音频格式。
 * @param sampleRate 目标采样率。
 * @return 可转换时返回 true。
 */
bool canConvertMixFormat(const WAVEFORMATEX *format, int sampleRate)
{
    if (!format || static_cast<int>(format->nSamplesPerSec) != sampleRate
        || format->nChannels <= 0) {
        return false;
    }
    if (isFloatFormat(format)) {
        return format->wBitsPerSample == 32;
    }
    return isPcmFormat(format)
        && (format->wBitsPerSample == 16
            || format->wBitsPerSample == 24
            || format->wBitsPerSample == 32);
}

/**
 * 填充 48kHz stereo float 的请求格式。
 * @param sampleRate 采样率。
 * @return WASAPI 扩展格式。
 */
WAVEFORMATEXTENSIBLE requestedFloatStereoFormat(int sampleRate)
{
    WAVEFORMATEXTENSIBLE format = {};
    format.Format.wFormatTag = WAVE_FORMAT_EXTENSIBLE;
    format.Format.nChannels = 2;
    format.Format.nSamplesPerSec = static_cast<DWORD>(sampleRate);
    format.Format.wBitsPerSample = 32;
    format.Format.nBlockAlign = format.Format.nChannels * format.Format.wBitsPerSample / 8;
    format.Format.nAvgBytesPerSec = format.Format.nSamplesPerSec * format.Format.nBlockAlign;
    format.Format.cbSize = sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX);
    format.Samples.wValidBitsPerSample = 32;
    format.dwChannelMask = SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT;
    format.SubFormat = KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
    return format;
}

/**
 * 把一个 PCM 样本转换为 float。
 * @param data 样本字节地址。
 * @param bitsPerSample 样本位深。
 * @return float 样本值。
 */
float pcmSampleToFloat(const unsigned char *data, int bitsPerSample)
{
    if (bitsPerSample == 16) {
        int16_t value = 0;
        std::memcpy(&value, data, sizeof(value));
        return static_cast<float>(value) / 32768.0f;
    }
    if (bitsPerSample == 24) {
        int value = static_cast<int>(data[0])
            | (static_cast<int>(data[1]) << 8)
            | (static_cast<int>(data[2]) << 16);
        if (value & 0x800000) {
            value |= ~0xffffff;
        }
        return static_cast<float>(value) / 8388608.0f;
    }
    if (bitsPerSample == 32) {
        int32_t value = 0;
        std::memcpy(&value, data, sizeof(value));
        return static_cast<float>(value) / 2147483648.0f;
    }
    return 0.0f;
}

/**
 * 读取一个源格式声道样本并转换为 float。
 * @param frame 帧起始地址。
 * @param channel 声道下标。
 * @param bytesPerSample 每个样本字节数。
 * @param bitsPerSample 样本位深。
 * @param floatSamples 源格式是否为 float。
 * @return float 样本值。
 */
float sourceChannelSample(const unsigned char *frame,
                          int channel,
                          int bytesPerSample,
                          int bitsPerSample,
                          bool floatSamples)
{
    const unsigned char *sample = frame + channel * bytesPerSample;
    if (floatSamples) {
        float value = 0.0f;
        std::memcpy(&value, sample, sizeof(value));
        return value;
    }
    return pcmSampleToFloat(sample, bitsPerSample);
}

/**
 * 把 WASAPI 数据包转换并追加为 stereo float PCM。
 * @param pendingPcm 输出缓存。
 * @param data WASAPI 数据包地址。
 * @param frames 帧数量。
 * @param flags WASAPI 数据包标记。
 * @param channels 源声道数量。
 * @param bitsPerSample 源样本位深。
 * @param floatSamples 源格式是否为 float。
 * @return 无返回值。
 */
void appendConvertedFrames(QByteArray *pendingPcm,
                           const BYTE *data,
                           UINT32 frames,
                           DWORD flags,
                           int channels,
                           int bitsPerSample,
                           bool floatSamples)
{
    if (!pendingPcm || frames == 0) {
        return;
    }

    const int bytesPerSample = std::max(1, bitsPerSample / 8);
    const int inputFrameBytes = std::max(1, channels) * bytesPerSample;
    const int oldSize = pendingPcm->size();
    pendingPcm->resize(oldSize + static_cast<int>(frames) * 2 * static_cast<int>(sizeof(float)));
    auto *target = reinterpret_cast<float *>(pendingPcm->data() + oldSize);

    const bool silent = (flags & AUDCLNT_BUFFERFLAGS_SILENT) != 0 || !data;
    for (UINT32 frameIndex = 0; frameIndex < frames; ++frameIndex) {
        float left = 0.0f;
        float right = 0.0f;
        if (!silent) {
            const unsigned char *frame = data + frameIndex * inputFrameBytes;
            left = sourceChannelSample(frame, 0, bytesPerSample, bitsPerSample, floatSamples);
            right = channels > 1
                ? sourceChannelSample(frame, 1, bytesPerSample, bitsPerSample, floatSamples)
                : left;
        }
        *target++ = left;
        *target++ = right;
    }
}

#endif

}  // namespace

WasapiAudioCaptureReader::WasapiAudioCaptureReader() = default;

WasapiAudioCaptureReader::~WasapiAudioCaptureReader()
{
    stop();
}

int WasapiAudioCaptureReader::preferredSampleRate() const
{
#ifdef _WIN32
    return defaultRenderMixSampleRate();
#else
    return 48000;
#endif
}

bool WasapiAudioCaptureReader::init(int frameBytes,
                                    int sampleRate,
                                    SampleCallback callback,
                                    QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef _WIN32
    Q_UNUSED(frameBytes)
    Q_UNUSED(sampleRate)
    Q_UNUSED(callback)
    if (error) {
        *error = QStringLiteral("WASAPI recording support is only available on Windows");
    }
    return false;
#else
    if (frameBytes <= 0 || sampleRate <= 0 || !callback) {
        if (error) {
            *error = QStringLiteral("invalid WASAPI capture parameters");
        }
        return false;
    }

    stop();
    m_frameBytes = frameBytes;
    m_sampleRate = sampleRate;
    m_callback = std::move(callback);
    m_sequence = 0;
    resetBuffers();

    const HRESULT comResult = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(comResult) && comResult != RPC_E_CHANGED_MODE) {
        return failWith(error,
                        QStringLiteral("Failed to initialize COM for WASAPI: %1")
                            .arg(hresultText(comResult)));
    }
    m_comInitialized = SUCCEEDED(comResult);

    IMMDeviceEnumerator *enumerator = nullptr;
    HRESULT result = CoCreateInstance(CLSID_MMDeviceEnumerator,
                                      nullptr,
                                      CLSCTX_ALL,
                                      IID_PPV_ARGS(&enumerator));
    if (FAILED(result) || !enumerator) {
        releaseResources();
        return failWith(error,
                        QStringLiteral("Failed to create WASAPI device enumerator: %1")
                            .arg(hresultText(result)));
    }

    result = enumerator->GetDefaultAudioEndpoint(eRender, eConsole, &m_device);
    enumerator->Release();
    if (FAILED(result) || !m_device) {
        releaseResources();
        return failWith(error,
                        QStringLiteral("Failed to open default WASAPI render device: %1")
                            .arg(hresultText(result)));
    }

    result = m_device->Activate(IID_IAudioClient,
                                CLSCTX_ALL,
                                nullptr,
                                reinterpret_cast<void **>(&m_audioClient));
    if (FAILED(result) || !m_audioClient) {
        releaseResources();
        return failWith(error,
                        QStringLiteral("Failed to activate WASAPI audio client: %1")
                            .arg(hresultText(result)));
    }

    WAVEFORMATEX *mixFormat = nullptr;
    result = m_audioClient->GetMixFormat(&mixFormat);
    if (FAILED(result) || !mixFormat) {
        releaseResources();
        return failWith(error,
                        QStringLiteral("Failed to read WASAPI mix format: %1")
                            .arg(hresultText(result)));
    }

    WAVEFORMATEXTENSIBLE requestedFormat = requestedFloatStereoFormat(sampleRate);
    WAVEFORMATEX *closestFormat = nullptr;
    result = m_audioClient->IsFormatSupported(AUDCLNT_SHAREMODE_SHARED,
                                              &requestedFormat.Format,
                                              &closestFormat);
    const WAVEFORMATEX *selectedFormat = nullptr;
    if (result == S_OK) {
        selectedFormat = &requestedFormat.Format;
    } else if (canConvertMixFormat(mixFormat, sampleRate)) {
        selectedFormat = mixFormat;
    } else {
        if (closestFormat) {
            CoTaskMemFree(closestFormat);
        }
        CoTaskMemFree(mixFormat);
        releaseResources();
        return failWith(error,
                        QStringLiteral("Default WASAPI loopback format cannot be converted to %1 Hz stereo float")
                            .arg(sampleRate));
    }

    m_channels = std::max(1, static_cast<int>(selectedFormat->nChannels));
    m_bitsPerSample = std::max(1, static_cast<int>(selectedFormat->wBitsPerSample));
    m_floatSamples = isFloatFormat(selectedFormat);

    constexpr REFERENCE_TIME bufferDuration = 10000000;
    result = m_audioClient->Initialize(AUDCLNT_SHAREMODE_SHARED,
                                       AUDCLNT_STREAMFLAGS_LOOPBACK,
                                       bufferDuration,
                                       0,
                                       selectedFormat,
                                       nullptr);
    if (closestFormat) {
        CoTaskMemFree(closestFormat);
    }
    CoTaskMemFree(mixFormat);
    if (FAILED(result)) {
        releaseResources();
        return failWith(error,
                        QStringLiteral("Failed to initialize WASAPI loopback capture: %1")
                            .arg(hresultText(result)));
    }

    result = m_audioClient->GetService(IID_IAudioCaptureClient,
                                       reinterpret_cast<void **>(&m_captureClient));
    if (FAILED(result) || !m_captureClient) {
        releaseResources();
        return failWith(error,
                        QStringLiteral("Failed to open WASAPI capture client: %1")
                            .arg(hresultText(result)));
    }
    markshot::debugLog("recording",
                       "【录制】【Windows音频】WASAPI loopback initialized sample_rate=%d channels=%d bits=%d float=%d",
                       m_sampleRate,
                       m_channels,
                       m_bitsPerSample,
                       m_floatSamples ? 1 : 0);
    return true;
#endif
}

void WasapiAudioCaptureReader::start()
{
#ifdef _WIN32
    if (m_running || !m_audioClient || !m_captureClient || !m_callback || m_frameBytes <= 0) {
        return;
    }
    m_running = true;
    m_thread = std::thread([this] {
        readLoop();
    });
#endif
}

void WasapiAudioCaptureReader::stop()
{
    if (m_running) {
        m_running = false;
        if (m_thread.joinable()) {
            m_thread.join();
        }
    }
    releaseResources();
    resetBuffers();
}

void WasapiAudioCaptureReader::readLoop()
{
#ifdef _WIN32
    const HRESULT comResult = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    const bool shouldUninitialize = SUCCEEDED(comResult);
    if (FAILED(comResult) && comResult != RPC_E_CHANGED_MODE) {
        markshot::debugLog("recording",
                           "【录制】【Windows音频】WASAPI capture thread COM init failed result=%s",
                           hresultText(comResult).toUtf8().constData());
        m_running = false;
        return;
    }

    const HRESULT startResult = m_audioClient->Start();
    if (FAILED(startResult)) {
        markshot::debugLog("recording",
                           "【录制】【Windows音频】WASAPI loopback start failed result=%s",
                           hresultText(startResult).toUtf8().constData());
        m_running = false;
        if (shouldUninitialize) {
            CoUninitialize();
        }
        return;
    }

    while (m_running) {
        Sleep(5);

        UINT32 packetFrames = 0;
        HRESULT result = m_captureClient->GetNextPacketSize(&packetFrames);
        if (FAILED(result)) {
            markshot::debugLog("recording",
                               "【录制】【Windows音频】WASAPI packet query failed result=%s",
                               hresultText(result).toUtf8().constData());
            break;
        }

        while (m_running && packetFrames > 0) {
            BYTE *data = nullptr;
            UINT32 frames = 0;
            DWORD flags = 0;
            result = m_captureClient->GetBuffer(&data, &frames, &flags, nullptr, nullptr);
            if (FAILED(result)) {
                markshot::debugLog("recording",
                                   "【录制】【Windows音频】WASAPI packet read failed result=%s",
                                   hresultText(result).toUtf8().constData());
                m_running = false;
                break;
            }

            appendConvertedFrames(&m_pendingPcm,
                                  data,
                                  frames,
                                  flags,
                                  m_channels,
                                  m_bitsPerSample,
                                  m_floatSamples);
            m_captureClient->ReleaseBuffer(frames);

            while (m_pendingPcm.size() >= m_frameBytes) {
                AudioCaptureSample sample;
                sample.pcm = m_pendingPcm.left(m_frameBytes);
                sample.sequence = ++m_sequence;
                m_pendingPcm.remove(0, m_frameBytes);
                m_callback(sample);
            }

            result = m_captureClient->GetNextPacketSize(&packetFrames);
            if (FAILED(result)) {
                markshot::debugLog("recording",
                                   "【录制】【Windows音频】WASAPI packet advance failed result=%s",
                                   hresultText(result).toUtf8().constData());
                m_running = false;
                break;
            }
        }
    }

    m_audioClient->Stop();
    if (shouldUninitialize) {
        CoUninitialize();
    }
#endif
}

void WasapiAudioCaptureReader::releaseResources()
{
#ifdef _WIN32
    if (m_captureClient) {
        m_captureClient->Release();
        m_captureClient = nullptr;
    }
    if (m_audioClient) {
        m_audioClient->Release();
        m_audioClient = nullptr;
    }
    if (m_device) {
        m_device->Release();
        m_device = nullptr;
    }
    if (m_comInitialized) {
        CoUninitialize();
        m_comInitialized = false;
    }
#endif
}

void WasapiAudioCaptureReader::resetBuffers()
{
    m_pendingPcm.clear();
}

}  // namespace markshot::recording
