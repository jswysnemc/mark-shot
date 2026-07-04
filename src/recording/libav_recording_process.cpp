#include "recording/libav_recording_process.h"

#include "recording/audio/pulse_audio_capture_reader.h"
#include "recording/libav_audio_encoder.h"
#include "recording/libav_error.h"
#include "recording/recording_frame_converter.h"

#include <QByteArray>
#include <QFile>

#include <algorithm>
#include <atomic>
#include <mutex>

#ifdef HAVE_LIBAV_RECORDING
extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/imgutils.h>
#include <libavutil/opt.h>
#include <libswscale/swscale.h>
}
#else
struct AVFrame;
#endif

namespace markshot::recording {
namespace {

/**
 * 向调用方写入错误文本。
 * @param error 输出错误信息。
 * @param text 错误文本。
 * @return 固定返回 false，便于调用处直接返回。
 */
bool failWith(QString *error, const QString &text)
{
    if (error) {
        *error = text;
    }
    return false;
}

/**
 * 把尺寸压到 yuv420p 可接受的偶数宽高。
 * @param size 输入帧尺寸。
 * @return 编码尺寸。
 */
QSize evenEncodedSize(QSize size)
{
    return {std::max(2, size.width() & ~1), std::max(2, size.height() & ~1)};
}

}  // namespace

class LibavRecordingProcessPrivate final {
public:
    /**
     * 启动库内 FFmpeg 编码器。
     * @param options 录制配置。
     * @param encoder 编码器候选。
     * @param frameSize 输入帧尺寸。
     * @param fps 目标帧率。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(const RecordingOptions &options,
               const RecordingVideoEncoderOptions &encoder,
               QSize frameSize,
               int fps,
               QString *error);

    /**
     * 写入一帧录制样本。
     * @param sample 录制帧样本。
     * @param error 输出错误信息。
     * @return 写入成功时返回 true。
     */
    bool writeFrame(const RecordingFrameSample &sample, QString *error);

    /**
     * 冲刷编码器并关闭输出文件。
     * @param error 输出错误信息。
     * @return 完成成功时返回 true。
     */
    bool finish(QString *error);

    /**
     * 取消编码并释放资源。
     * @return 无返回值。
     */
    void cancel();

private:
    /**
     * 初始化输出容器。
     * @param outputPath 输出路径。
     * @param error 输出错误信息。
     * @return 初始化成功时返回 true。
     */
    bool openOutput(const QString &outputPath, QString *error);

    /**
     * 初始化视频编码器。
     * @param fps 目标帧率。
     * @param error 输出错误信息。
     * @return 初始化成功时返回 true。
     */
    bool openEncoder(int fps, QString *error);

    /**
     * 初始化音频编码器和采集器。
     * @param error 输出错误信息。
     * @return 初始化成功时返回 true。
     */
    bool openAudio(QString *error);

    /**
     * 首帧视频到达时启动音频采集。
     * @return 无返回值。
     */
    void startAudioCaptureIfNeeded();

    /**
     * 编码音频采集线程送来的 PCM 样本。
     * @param sample 音频样本。
     * @return 无返回值。
     */
    void encodeAudioSample(const AudioCaptureSample &sample);

    /**
     * 把录制样本转换为连续 BGRA 字节。
     * @param sample 录制帧样本。
     * @param error 输出错误信息。
     * @return BGRA 字节视图。
     */
    RecordingBgraFrame bgraBytesForSample(const RecordingFrameSample &sample, QString *error);

    /**
     * 把 BGRA 输入帧转换为编码帧。
     * @param bytes BGRA 字节视图。
     * @param error 输出错误信息。
     * @return 转换成功时返回 true。
     */
    bool fillVideoFrame(RecordingBgraFrame bytes, QString *error);

    /**
     * 把帧送入编码器并写出已生成的 packet。
     * @param frame 输入帧，传空表示冲刷编码器。
     * @param error 输出错误信息。
     * @return 写出成功时返回 true。
     */
    bool encodeFrame(AVFrame *frame, QString *error);

    /**
     * 释放所有 FFmpeg 资源。
     * @return 无返回值。
     */
    void cleanup();

#ifdef HAVE_LIBAV_RECORDING
    AVFormatContext *m_formatContext = nullptr;
    AVCodecContext *m_codecContext = nullptr;
    AVStream *m_stream = nullptr;
    AVFrame *m_frame = nullptr;
    AVPacket *m_packet = nullptr;
    SwsContext *m_swsContext = nullptr;
#endif
    RecordingFrameConverter m_converter;
    LibavAudioEncoder m_audioEncoder;
    PulseAudioCaptureReader m_audioReader;
    QByteArray m_outputPathBytes;
    QSize m_frameSize;
    QSize m_encodedSize;
    std::mutex m_writeMutex;
    std::atomic<bool> m_audioFailed{false};
    int m_fps = 30;
    int64_t m_nextPts = 0;
    bool m_started = false;
    bool m_enableAudio = false;
    bool m_audioCaptureStarted = false;
};

bool LibavRecordingProcessPrivate::start(const RecordingOptions &options,
                                         const RecordingVideoEncoderOptions &encoder,
                                         QSize frameSize,
                                         int fps,
                                         QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(options)
    Q_UNUSED(encoder)
    Q_UNUSED(frameSize)
    Q_UNUSED(fps)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    if (encoder.hardware || encoder.id != QStringLiteral("libx264")) {
        return failWith(error, QStringLiteral("libav writer supports only libx264 software encoding"));
    }
    if (frameSize.isEmpty()) {
        return failWith(error, QStringLiteral("Cannot start libav writer with an empty frame size"));
    }

    cleanup();
    m_frameSize = frameSize;
    m_encodedSize = evenEncodedSize(frameSize);
    m_fps = std::max(1, fps);
    m_nextPts = 0;
    m_enableAudio = options.includeAudio;
    m_audioFailed = false;
    m_audioCaptureStarted = false;

    if (!openOutput(options.outputPath, error) || !openEncoder(m_fps, error)) {
        cleanup();
        return false;
    }
    if (m_enableAudio && !openAudio(error)) {
        cleanup();
        return false;
    }

    int result = avformat_write_header(m_formatContext, nullptr);
    if (result < 0) {
        cleanup();
        return failWith(error,
                        QStringLiteral("Failed to write libav output header: %1")
                            .arg(libavErrorText(result)));
    }
    m_started = true;
    return true;
#endif
}

bool LibavRecordingProcessPrivate::writeFrame(const RecordingFrameSample &sample, QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(sample)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    if (!m_started) {
        return failWith(error, QStringLiteral("libav writer is not started"));
    }
    if (m_audioFailed) {
        return failWith(error, QStringLiteral("libav audio capture or encoding failed"));
    }
    startAudioCaptureIfNeeded();

    const RecordingBgraFrame bytes = bgraBytesForSample(sample, error);
    if (!bytes.data || bytes.size <= 0) {
        return false;
    }
    if (!fillVideoFrame(bytes, error)) {
        return false;
    }
    m_frame->pts = m_nextPts++;
    return encodeFrame(m_frame, error);
#endif
}

bool LibavRecordingProcessPrivate::finish(QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_LIBAV_RECORDING
    return true;
#else
    if (!m_started) {
        cleanup();
        return true;
    }
    if (m_enableAudio) {
        m_audioReader.stop();
        m_audioCaptureStarted = false;
    }
    if (!encodeFrame(nullptr, error)) {
        cleanup();
        return false;
    }
    if (m_enableAudio && !m_audioEncoder.flush(m_writeMutex, error)) {
        cleanup();
        return false;
    }
    const int result = av_write_trailer(m_formatContext);
    cleanup();
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to write libav output trailer: %1")
                            .arg(libavErrorText(result)));
    }
    return true;
#endif
}

void LibavRecordingProcessPrivate::cancel()
{
    cleanup();
}

bool LibavRecordingProcessPrivate::openAudio(QString *error)
{
#ifndef HAVE_LIBAV_RECORDING
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    constexpr int kAudioSampleRate = 48000;
    if (!m_audioEncoder.open(m_formatContext, kAudioSampleRate, error)) {
        return false;
    }
    if (!m_audioReader.init(m_audioEncoder.frameBytes(),
                            kAudioSampleRate,
                            [this](const AudioCaptureSample &sample) {
                                encodeAudioSample(sample);
                            },
                            error)) {
        return false;
    }
    return true;
#endif
}

void LibavRecordingProcessPrivate::startAudioCaptureIfNeeded()
{
    if (!m_enableAudio || m_audioCaptureStarted) {
        return;
    }
    m_audioCaptureStarted = true;
    m_audioReader.start();
}

void LibavRecordingProcessPrivate::encodeAudioSample(const AudioCaptureSample &sample)
{
    QString error;
    if (!m_audioEncoder.encode(sample, m_writeMutex, &error)) {
        m_audioFailed = true;
    }
}

bool LibavRecordingProcessPrivate::openOutput(const QString &outputPath, QString *error)
{
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(outputPath)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    m_outputPathBytes = QFile::encodeName(outputPath);
    int result = avformat_alloc_output_context2(&m_formatContext,
                                                nullptr,
                                                nullptr,
                                                m_outputPathBytes.constData());
    if (result < 0 || !m_formatContext) {
        return failWith(error,
                        QStringLiteral("Failed to allocate libav output context: %1")
                            .arg(libavErrorText(result)));
    }
    if (!(m_formatContext->oformat->flags & AVFMT_NOFILE)) {
        result = avio_open(&m_formatContext->pb, m_outputPathBytes.constData(), AVIO_FLAG_WRITE);
        if (result < 0) {
            return failWith(error,
                            QStringLiteral("Failed to open libav output file: %1")
                                .arg(libavErrorText(result)));
        }
    }
    return true;
#endif
}

bool LibavRecordingProcessPrivate::openEncoder(int fps, QString *error)
{
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(fps)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    const AVCodec *codec = avcodec_find_encoder_by_name("libx264");
    if (!codec) {
        return failWith(error, QStringLiteral("libx264 encoder is not available in FFmpeg libraries"));
    }

    m_stream = avformat_new_stream(m_formatContext, codec);
    if (!m_stream) {
        return failWith(error, QStringLiteral("Failed to create libav video stream"));
    }

    m_codecContext = avcodec_alloc_context3(codec);
    if (!m_codecContext) {
        return failWith(error, QStringLiteral("Failed to allocate libav codec context"));
    }
    m_codecContext->width = m_encodedSize.width();
    m_codecContext->height = m_encodedSize.height();
    m_codecContext->pix_fmt = AV_PIX_FMT_YUV420P;
    m_codecContext->time_base = AVRational{1, fps};
    m_codecContext->framerate = AVRational{fps, 1};
    m_codecContext->gop_size = fps;
    m_codecContext->max_b_frames = 0;
    if (m_formatContext->oformat->flags & AVFMT_GLOBALHEADER) {
        m_codecContext->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }

    AVDictionary *codecOptions = nullptr;
    av_dict_set(&codecOptions, "preset", fps >= 48 ? "ultrafast" : "veryfast", 0);
    av_dict_set(&codecOptions, "tune", "zerolatency", 0);
    av_dict_set(&codecOptions, "crf", "23", 0);
    int result = avcodec_open2(m_codecContext, codec, &codecOptions);
    av_dict_free(&codecOptions);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to open libav video encoder: %1")
                            .arg(libavErrorText(result)));
    }

    result = avcodec_parameters_from_context(m_stream->codecpar, m_codecContext);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to copy libav codec parameters: %1")
                            .arg(libavErrorText(result)));
    }
    m_stream->time_base = m_codecContext->time_base;

    m_frame = av_frame_alloc();
    m_packet = av_packet_alloc();
    if (!m_frame || !m_packet) {
        return failWith(error, QStringLiteral("Failed to allocate libav frame or packet"));
    }
    m_frame->format = m_codecContext->pix_fmt;
    m_frame->width = m_codecContext->width;
    m_frame->height = m_codecContext->height;
    result = av_frame_get_buffer(m_frame, 32);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to allocate libav frame buffer: %1")
                            .arg(libavErrorText(result)));
    }

    m_swsContext = sws_getContext(m_frameSize.width(),
                                  m_frameSize.height(),
                                  AV_PIX_FMT_BGRA,
                                  m_encodedSize.width(),
                                  m_encodedSize.height(),
                                  AV_PIX_FMT_YUV420P,
                                  SWS_FAST_BILINEAR,
                                  nullptr,
                                  nullptr,
                                  nullptr);
    if (!m_swsContext) {
        return failWith(error, QStringLiteral("Failed to create libav scale context"));
    }
    return true;
#endif
}

RecordingBgraFrame LibavRecordingProcessPrivate::bgraBytesForSample(const RecordingFrameSample &sample,
                                                                    QString *error)
{
    if (sample.bgra.isValid() && sample.bgra.size == m_frameSize) {
        return {sample.bgra.constData(),
                sample.bgra.byteSize(),
                sample.bgra.stride,
                sample.bgra.yInverted};
    }
    if (!sample.image.isNull()) {
        return m_converter.convertToBgra(sample.image, m_frameSize, error);
    }
    failWith(error, QStringLiteral("Cannot write an empty recording frame"));
    return {};
}

bool LibavRecordingProcessPrivate::fillVideoFrame(RecordingBgraFrame bytes, QString *error)
{
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(bytes)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    const int result = av_frame_make_writable(m_frame);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to make libav frame writable: %1")
                            .arg(libavErrorText(result)));
    }
    const char *source = bytes.data;
    int sourceStride = bytes.stride > 0 ? bytes.stride : m_frameSize.width() * 4;
    if (bytes.yInverted) {
        source += static_cast<qsizetype>(sourceStride) * (m_frameSize.height() - 1);
        sourceStride = -sourceStride;
    }
    const uint8_t *sourceData[] = {reinterpret_cast<const uint8_t *>(source)};
    const int sourceLineSize[] = {sourceStride};
    sws_scale(m_swsContext,
              sourceData,
              sourceLineSize,
              0,
              m_frameSize.height(),
              m_frame->data,
              m_frame->linesize);
    return true;
#endif
}

bool LibavRecordingProcessPrivate::encodeFrame(AVFrame *frame, QString *error)
{
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(frame)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    int result = avcodec_send_frame(m_codecContext, frame);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to send frame to libav encoder: %1")
                            .arg(libavErrorText(result)));
    }

    while (result >= 0) {
        result = avcodec_receive_packet(m_codecContext, m_packet);
        if (result == AVERROR(EAGAIN) || result == AVERROR_EOF) {
            return true;
        }
        if (result < 0) {
            return failWith(error,
                            QStringLiteral("Failed to receive packet from libav encoder: %1")
                                .arg(libavErrorText(result)));
        }
        av_packet_rescale_ts(m_packet, m_codecContext->time_base, m_stream->time_base);
        m_packet->stream_index = m_stream->index;
        {
            std::lock_guard<std::mutex> lock(m_writeMutex);
            result = av_interleaved_write_frame(m_formatContext, m_packet);
        }
        av_packet_unref(m_packet);
        if (result < 0) {
            return failWith(error,
                            QStringLiteral("Failed to write libav packet: %1")
                                .arg(libavErrorText(result)));
        }
    }
    return true;
#endif
}

void LibavRecordingProcessPrivate::cleanup()
{
#ifdef HAVE_LIBAV_RECORDING
    m_audioReader.stop();
    m_audioCaptureStarted = false;
    m_audioEncoder.close();
    if (m_codecContext && m_started) {
        avcodec_flush_buffers(m_codecContext);
    }
    if (m_frame) {
        av_frame_free(&m_frame);
    }
    if (m_packet) {
        av_packet_free(&m_packet);
    }
    if (m_swsContext) {
        sws_freeContext(m_swsContext);
        m_swsContext = nullptr;
    }
    if (m_codecContext) {
        avcodec_free_context(&m_codecContext);
    }
    if (m_formatContext) {
        if (m_formatContext->pb) {
            avio_closep(&m_formatContext->pb);
        }
        avformat_free_context(m_formatContext);
        m_formatContext = nullptr;
    }
    m_stream = nullptr;
#endif
    m_started = false;
}

LibavRecordingProcess::LibavRecordingProcess()
    : m_impl(std::make_unique<LibavRecordingProcessPrivate>())
{
}

LibavRecordingProcess::~LibavRecordingProcess() = default;

bool LibavRecordingProcess::start(const RecordingOptions &options,
                                  const RecordingVideoEncoderOptions &encoder,
                                  QSize frameSize,
                                  int fps,
                                  QString *error)
{
    return m_impl->start(options, encoder, frameSize, fps, error);
}

bool LibavRecordingProcess::writeFrame(const RecordingFrameSample &sample, QString *error)
{
    return m_impl->writeFrame(sample, error);
}

bool LibavRecordingProcess::finish(QString *error)
{
    return m_impl->finish(error);
}

void LibavRecordingProcess::cancel()
{
    m_impl->cancel();
}

}  // namespace markshot::recording
