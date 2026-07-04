#include "recording/libav_audio_encoder.h"

#include "recording/libav_error.h"

#ifdef HAVE_LIBAV_RECORDING
extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/channel_layout.h>
#include <libavutil/opt.h>
#include <libswresample/swresample.h>
}
#endif

#include <cstring>

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

#ifdef HAVE_LIBAV_RECORDING
/**
 * 【录制】【音频编码】选择 AAC 编码器支持的采样格式。
 * @param codec 编码器。
 * @param codecContext 编码上下文。
 * @return 采样格式。
 */
AVSampleFormat preferredAudioSampleFormat(const AVCodec *codec, const AVCodecContext *codecContext)
{
#if defined(LIBAVCODEC_VERSION_MAJOR) && LIBAVCODEC_VERSION_MAJOR >= 62
    const void *configs = nullptr;
    int configCount = 0;
    if (avcodec_get_supported_config(codecContext,
                                     codec,
                                     AV_CODEC_CONFIG_SAMPLE_FORMAT,
                                     0,
                                     &configs,
                                     &configCount) == 0
        && configs && configCount > 0) {
        return static_cast<const AVSampleFormat *>(configs)[0];
    }
    return AV_SAMPLE_FMT_FLTP;
#else
    return codec->sample_fmts ? codec->sample_fmts[0] : AV_SAMPLE_FMT_FLTP;
#endif
}
#endif

}  // namespace

class LibavAudioEncoder::Private final {
public:
    bool open(AVFormatContext *formatContext, int sampleRate, QString *error);
    int frameBytes() const;
    bool encode(const AudioCaptureSample &sample, std::mutex &writeMutex, QString *error);
    bool flush(std::mutex &writeMutex, QString *error);
    void close();

private:
#ifdef HAVE_LIBAV_RECORDING
    /**
     * 分配可复用的音频输入帧、输出帧和 packet。
     * @param frameSamples 每个音频帧的采样数。
     * @param error 输出错误信息。
     * @return 分配成功时返回 true。
     */
    bool allocateReusableFrames(int frameSamples, QString *error);

    bool encodeFrame(AVFrame *frame, std::mutex &writeMutex, QString *error);
#endif

#ifdef HAVE_LIBAV_RECORDING
    AVFormatContext *m_formatContext = nullptr;
    AVCodecContext *m_codecContext = nullptr;
    AVStream *m_stream = nullptr;
    SwrContext *m_swrContext = nullptr;
    AVFrame *m_inputFrame = nullptr;
    AVFrame *m_outputFrame = nullptr;
    AVPacket *m_packet = nullptr;
    int64_t m_nextPts = 0;
#endif
    int m_sampleRate = 48000;
    int m_frameBytes = 0;
};

bool LibavAudioEncoder::Private::open(AVFormatContext *formatContext,
                                      int sampleRate,
                                      QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(formatContext)
    Q_UNUSED(sampleRate)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    if (!formatContext || sampleRate <= 0) {
        return failWith(error, QStringLiteral("invalid libav audio encoder parameters"));
    }

    const AVCodec *codec = avcodec_find_encoder_by_name("aac");
    if (!codec) {
        return failWith(error, QStringLiteral("AAC encoder is not available in FFmpeg libraries"));
    }

    m_formatContext = formatContext;
    m_sampleRate = sampleRate;
    m_stream = avformat_new_stream(m_formatContext, codec);
    if (!m_stream) {
        return failWith(error, QStringLiteral("Failed to create libav audio stream"));
    }

    m_codecContext = avcodec_alloc_context3(codec);
    if (!m_codecContext) {
        return failWith(error, QStringLiteral("Failed to allocate libav audio codec context"));
    }
    m_codecContext->sample_rate = m_sampleRate;
    m_codecContext->sample_fmt = preferredAudioSampleFormat(codec, m_codecContext);
    m_codecContext->time_base = AVRational{1, m_sampleRate};
    av_channel_layout_from_mask(&m_codecContext->ch_layout, AV_CH_LAYOUT_STEREO);
    if (m_formatContext->oformat->flags & AVFMT_GLOBALHEADER) {
        m_codecContext->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }

    int result = avcodec_open2(m_codecContext, codec, nullptr);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to open libav audio encoder: %1")
                            .arg(libavErrorText(result)));
    }

    result = avcodec_parameters_from_context(m_stream->codecpar, m_codecContext);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to copy libav audio parameters: %1")
                            .arg(libavErrorText(result)));
    }
    m_stream->time_base = m_codecContext->time_base;

    m_swrContext = swr_alloc();
    if (!m_swrContext) {
        return failWith(error, QStringLiteral("Failed to allocate libav audio resampler"));
    }
    AVChannelLayout inputLayout = AV_CHANNEL_LAYOUT_STEREO;
    av_opt_set_chlayout(m_swrContext, "in_chlayout", &inputLayout, 0);
    av_opt_set_chlayout(m_swrContext, "out_chlayout", &m_codecContext->ch_layout, 0);
    av_opt_set_int(m_swrContext, "in_sample_rate", m_sampleRate, 0);
    av_opt_set_int(m_swrContext, "out_sample_rate", m_codecContext->sample_rate, 0);
    av_opt_set_sample_fmt(m_swrContext, "in_sample_fmt", AV_SAMPLE_FMT_FLT, 0);
    av_opt_set_sample_fmt(m_swrContext, "out_sample_fmt", m_codecContext->sample_fmt, 0);
    result = swr_init(m_swrContext);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to initialize libav audio resampler: %1")
                            .arg(libavErrorText(result)));
    }

    const int frameSamples = m_codecContext->frame_size > 0 ? m_codecContext->frame_size : 1024;
    m_frameBytes = frameSamples * 2 * static_cast<int>(sizeof(float));
    if (!allocateReusableFrames(frameSamples, error)) {
        return false;
    }
    m_nextPts = 0;
    return true;
#endif
}

int LibavAudioEncoder::Private::frameBytes() const
{
    return m_frameBytes;
}

bool LibavAudioEncoder::Private::encode(const AudioCaptureSample &sample,
                                        std::mutex &writeMutex,
                                        QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(sample)
    Q_UNUSED(writeMutex)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    if (!m_codecContext || !m_swrContext || !m_inputFrame || !m_outputFrame || !m_packet
        || sample.pcm.size() < m_frameBytes) {
        return failWith(error, QStringLiteral("invalid libav audio sample"));
    }

    int result = av_frame_make_writable(m_inputFrame);
    if (result >= 0) {
        result = av_frame_make_writable(m_outputFrame);
    }
    if (result >= 0) {
        std::memcpy(m_inputFrame->data[0], sample.pcm.constData(), static_cast<size_t>(m_frameBytes));
    }
    if (result >= 0) {
        m_outputFrame->pts = m_nextPts;
        result = swr_convert_frame(m_swrContext, m_outputFrame, m_inputFrame);
    }

    bool ok = false;
    if (result >= 0) {
        m_nextPts += m_outputFrame->nb_samples;
        ok = encodeFrame(m_outputFrame, writeMutex, error);
    } else {
        failWith(error,
                 QStringLiteral("Failed to convert libav audio frame: %1")
                     .arg(libavErrorText(result)));
    }
    return ok;
#endif
}

bool LibavAudioEncoder::Private::flush(std::mutex &writeMutex, QString *error)
{
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(writeMutex)
    if (error) {
        error->clear();
    }
    return true;
#else
    if (!m_codecContext) {
        return true;
    }
    return encodeFrame(nullptr, writeMutex, error);
#endif
}

void LibavAudioEncoder::Private::close()
{
#ifdef HAVE_LIBAV_RECORDING
    if (m_packet) {
        av_packet_free(&m_packet);
    }
    if (m_inputFrame) {
        av_frame_free(&m_inputFrame);
    }
    if (m_outputFrame) {
        av_frame_free(&m_outputFrame);
    }
    if (m_swrContext) {
        swr_free(&m_swrContext);
    }
    if (m_codecContext) {
        avcodec_free_context(&m_codecContext);
    }
    m_stream = nullptr;
    m_formatContext = nullptr;
    m_nextPts = 0;
#endif
    m_frameBytes = 0;
}

#ifdef HAVE_LIBAV_RECORDING
bool LibavAudioEncoder::Private::allocateReusableFrames(int frameSamples, QString *error)
{
    m_inputFrame = av_frame_alloc();
    m_outputFrame = av_frame_alloc();
    m_packet = av_packet_alloc();
    if (!m_inputFrame || !m_outputFrame || !m_packet) {
        return failWith(error, QStringLiteral("Failed to allocate reusable libav audio buffers"));
    }

    // 1. 【录制】【音频编码】输入帧固定接收 PulseAudio 采集的 float32 stereo PCM
    m_inputFrame->format = AV_SAMPLE_FMT_FLT;
    m_inputFrame->sample_rate = m_sampleRate;
    m_inputFrame->nb_samples = frameSamples;
    av_channel_layout_from_mask(&m_inputFrame->ch_layout, AV_CH_LAYOUT_STEREO);
    int result = av_frame_get_buffer(m_inputFrame, 0);

    // 2. 【录制】【音频编码】输出帧使用 AAC 编码器实际支持的采样格式
    m_outputFrame->format = m_codecContext->sample_fmt;
    m_outputFrame->sample_rate = m_codecContext->sample_rate;
    m_outputFrame->nb_samples = frameSamples;
    av_channel_layout_copy(&m_outputFrame->ch_layout, &m_codecContext->ch_layout);
    if (result >= 0) {
        result = av_frame_get_buffer(m_outputFrame, 0);
    }
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to allocate reusable libav audio frame data: %1")
                            .arg(libavErrorText(result)));
    }
    return true;
}

bool LibavAudioEncoder::Private::encodeFrame(AVFrame *frame,
                                             std::mutex &writeMutex,
                                             QString *error)
{
    int result = avcodec_send_frame(m_codecContext, frame);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to send frame to libav audio encoder: %1")
                            .arg(libavErrorText(result)));
    }

    while (result >= 0) {
        result = avcodec_receive_packet(m_codecContext, m_packet);
        if (result == AVERROR(EAGAIN) || result == AVERROR_EOF) {
            return true;
        }
        if (result < 0) {
            return failWith(error,
                            QStringLiteral("Failed to receive packet from libav audio encoder: %1")
                                .arg(libavErrorText(result)));
        }
        av_packet_rescale_ts(m_packet, m_codecContext->time_base, m_stream->time_base);
        m_packet->stream_index = m_stream->index;
        {
            std::lock_guard<std::mutex> lock(writeMutex);
            result = av_interleaved_write_frame(m_formatContext, m_packet);
        }
        av_packet_unref(m_packet);
        if (result < 0) {
            return failWith(error,
                            QStringLiteral("Failed to write libav audio packet: %1")
                                .arg(libavErrorText(result)));
        }
    }
    return true;
}
#endif

LibavAudioEncoder::LibavAudioEncoder()
    : d(new Private)
{
}

LibavAudioEncoder::~LibavAudioEncoder()
{
    close();
    delete d;
}

bool LibavAudioEncoder::open(AVFormatContext *formatContext, int sampleRate, QString *error)
{
    return d->open(formatContext, sampleRate, error);
}

int LibavAudioEncoder::frameBytes() const
{
    return d->frameBytes();
}

bool LibavAudioEncoder::encode(const AudioCaptureSample &sample,
                               std::mutex &writeMutex,
                               QString *error)
{
    return d->encode(sample, writeMutex, error);
}

bool LibavAudioEncoder::flush(std::mutex &writeMutex, QString *error)
{
    return d->flush(writeMutex, error);
}

void LibavAudioEncoder::close()
{
    d->close();
}

}  // namespace markshot::recording
