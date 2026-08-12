#include "recording/libav_recording_process.h"

#include "recording/audio/audio_capture_reader.h"
#include "recording/audio/audio_capture_reader_factory.h"
#include "recording/libav_audio_encoder.h"
#include "recording/libav_error.h"
#include "recording/recording_frame_converter.h"

#include <QByteArray>

#include <algorithm>
#include <atomic>
#include <memory>

#ifdef HAVE_LIBAV_RECORDING
#include "recording/libav/libav_hw_encoder_context.h"
#include "recording/libav/libav_muxer.h"
#include "recording/libav/libav_video_encoder_setup.h"
#include "recording/libav/libav_video_scaler.h"

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
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
 * 把尺寸裁剪到 yuv420p 可接受的偶数宽高。
 * @param size 输入帧尺寸。
 * @return 编码尺寸，奇数边按裁剪丢弃最后一行或一列。
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
     * 复用上一帧已转换数据补写一帧，跳过像素转换。
     * @param error 输出错误信息。
     * @return 写入成功时返回 true。
     */
    bool writeRepeatFrame(QString *error);

    /**
     * 冲刷编码器并关闭输出文件。
     * @param error 输出错误信息。
     * @return 完成成功时返回 true。
     */
    bool finish(QString *error);

    /**
     * 设置暂停状态，暂停时停止音频采集。
     * @param paused 暂停时为 true。
     * @return 无返回值。
     */
    void setPaused(bool paused);

    /**
     * 取消编码并释放资源。
     * @return 无返回值。
     */
    void cancel();

private:
    /**
     * 初始化视频编码器、编码帧与像素转换器。
     * @param encoder 编码器候选。
     * @param fps 目标帧率。
     * @param error 输出错误信息。
     * @return 初始化成功时返回 true。
     */
    bool openVideo(const RecordingVideoEncoderOptions &encoder, int fps, QString *error);

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
    LibavMuxer m_muxer;
    LibavHwEncoderContext m_hardware;
    LibavVideoScaler m_scaler;
    AVCodecContext *m_codecContext = nullptr;
    AVStream *m_stream = nullptr;
    AVFrame *m_frame = nullptr;
    AVPacket *m_packet = nullptr;
    bool m_hardwareFrames = false;
#endif
    RecordingFrameConverter m_converter;
    LibavAudioEncoder m_audioEncoder;
    std::unique_ptr<AudioCaptureReader> m_audioReader;
    RecordingQualityProfile m_quality;
    QSize m_frameSize;
    QSize m_encodedSize;
    std::atomic<bool> m_audioFailed{false};
    int m_fps = 30;
    int64_t m_nextPts = 0;
    bool m_started = false;
    bool m_enableAudio = false;
    QString m_audioDevice;
    bool m_audioCaptureStarted = false;
    bool m_audioResumePending = false;
    bool m_paused = false;
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
    if (frameSize.isEmpty()) {
        return failWith(error, QStringLiteral("Cannot start libav writer with an empty frame size"));
    }

    cleanup();
    m_frameSize = frameSize;
    m_encodedSize = evenEncodedSize(frameSize);
    m_fps = std::max(1, fps);
    m_nextPts = 0;
    m_enableAudio = options.includeAudio;
    m_audioDevice = options.audioDevice;
    m_audioFailed = false;
    m_audioCaptureStarted = false;
    m_audioResumePending = false;
    m_paused = false;

    m_quality = recordingQualityProfile(options.quality, m_fps);

    // 1. 先打开容器，编码器需要据此判断是否输出全局头
    if (!m_muxer.open(options.outputPath, recordingContainerMuxerName(options.container), error)
        || !openVideo(encoder, m_fps, error)) {
        cleanup();
        return false;
    }
    if (m_enableAudio && !openAudio(error)) {
        cleanup();
        return false;
    }

    if (!m_muxer.writeHeader(error)) {
        cleanup();
        return false;
    }
    m_started = true;
    return true;
#endif
}

bool LibavRecordingProcessPrivate::openVideo(const RecordingVideoEncoderOptions &encoder,
                                             int fps,
                                             QString *error)
{
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(encoder)
    Q_UNUSED(fps)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    // 1. 打开编码器，需要显存帧的编码器在此完成帧池创建与绑定
    LibavVideoEncoderRequest request;
    request.encoder = encoder;
    request.quality = m_quality;
    request.encodedSize = m_encodedSize;
    request.fps = fps;
    request.globalHeader = m_muxer.needsGlobalHeader();

    LibavVideoEncoderResult result;
    if (!openVideoEncoder(request, &m_hardware, &result, error)) {
        return false;
    }
    m_codecContext = result.codecContext;
    m_hardwareFrames = result.hardwareFrames;

    // 2. 建立输出流并复制编码参数
    m_stream = avformat_new_stream(m_muxer.formatContext(), result.codec);
    if (!m_stream) {
        return failWith(error, QStringLiteral("Failed to create libav video stream"));
    }
    const int parametersResult = avcodec_parameters_from_context(m_stream->codecpar, m_codecContext);
    if (parametersResult < 0) {
        return failWith(error,
                        QStringLiteral("Failed to copy libav codec parameters: %1")
                            .arg(libavErrorText(parametersResult)));
    }
    m_stream->time_base = m_codecContext->time_base;

    // 3. 分配系统内存编码帧，硬件编码时它承担上传前的暂存
    m_frame = av_frame_alloc();
    m_packet = av_packet_alloc();
    if (!m_frame || !m_packet) {
        return failWith(error, QStringLiteral("Failed to allocate libav frame or packet"));
    }
    m_frame->format = result.scalerFormat;
    m_frame->width = m_encodedSize.width();
    m_frame->height = m_encodedSize.height();
    const int bufferResult = av_frame_get_buffer(m_frame, 32);
    if (bufferResult < 0) {
        return failWith(error,
                        QStringLiteral("Failed to allocate libav frame buffer: %1")
                            .arg(libavErrorText(bufferResult)));
    }

    // 4. 源与目标尺寸一致，像素转换可按行切片并行
    return m_scaler.prepare(m_encodedSize, m_encodedSize, result.scalerFormat, error);
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

    // 硬件编码需要把系统内存帧上传到显存后再送入编码器
    AVFrame *encodeInput = m_frame;
    if (m_hardwareFrames) {
        encodeInput = m_hardware.upload(m_frame, error);
        if (!encodeInput) {
            return false;
        }
    }
    return encodeFrame(encodeInput, error);
#endif
}

bool LibavRecordingProcessPrivate::writeRepeatFrame(QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_LIBAV_RECORDING
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    if (!m_started) {
        return failWith(error, QStringLiteral("libav writer is not started"));
    }
    if (m_nextPts <= 0) {
        return failWith(error, QStringLiteral("libav writer has no previous frame to repeat"));
    }

    // 补帧复用上一帧已转换像素，只推进 pts，避免重复执行像素转换与显存上传
    AVFrame *repeated = m_hardwareFrames ? m_hardware.lastUploadedFrame() : m_frame;
    if (!repeated) {
        return failWith(error, QStringLiteral("libav writer has no previous frame to repeat"));
    }
    repeated->pts = m_nextPts++;
    return encodeFrame(repeated, error);
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
        if (m_audioReader) {
            m_audioReader->stop();
        }
        m_audioCaptureStarted = false;
    }
    if (!encodeFrame(nullptr, error)) {
        cleanup();
        return false;
    }
    if (m_enableAudio && !m_audioEncoder.flush(m_muxer.writeMutex(), error)) {
        cleanup();
        return false;
    }
    const bool ok = m_muxer.writeTrailer(error);
    cleanup();
    return ok;
#endif
}

void LibavRecordingProcessPrivate::setPaused(bool paused)
{
    if (m_paused == paused) {
        return;
    }
    m_paused = paused;
    if (!m_enableAudio || !m_audioReader) {
        return;
    }

    // 1. 暂停时停止音频采集，音频编码器的样本计数随之停住，恢复后不会出现错位
    if (paused) {
        if (m_audioCaptureStarted) {
            m_audioReader->stop();
            m_audioCaptureStarted = false;
            m_audioResumePending = true;
        }
        return;
    }

    // 2. 只恢复此前确实在采集的音频，尚未收到首帧时保持等待
    if (m_audioResumePending) {
        m_audioResumePending = false;
        m_audioCaptureStarted = true;
        m_audioReader->start();
    }
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
    m_audioReader = createPlatformAudioCaptureReader();
    if (!m_audioReader) {
        return failWith(error, recordingAudioUnavailableText());
    }
    const int audioSampleRate = m_audioReader->preferredSampleRate();
    if (!m_audioEncoder.open(m_muxer.formatContext(), audioSampleRate, error)) {
        return false;
    }
    if (!m_audioReader->init(m_audioEncoder.frameBytes(),
                             audioSampleRate,
                             m_audioDevice,
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
    if (m_audioReader) {
        m_audioReader->start();
    }
}

void LibavRecordingProcessPrivate::encodeAudioSample(const AudioCaptureSample &sample)
{
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(sample)
#else
    QString error;
    if (!m_audioEncoder.encode(sample, m_muxer.writeMutex(), &error)) {
        m_audioFailed = true;
    }
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
    const int stride = bytes.stride > 0 ? bytes.stride : m_frameSize.width() * 4;
    return m_scaler.convert(bytes.data,
                            stride,
                            m_frameSize.height(),
                            bytes.yInverted,
                            m_frame,
                            error);
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
        const bool written = m_muxer.writePacket(m_packet, error);
        av_packet_unref(m_packet);
        if (!written) {
            return false;
        }
    }
    return true;
#endif
}

void LibavRecordingProcessPrivate::cleanup()
{
#ifdef HAVE_LIBAV_RECORDING
    if (m_audioReader) {
        m_audioReader->stop();
        m_audioReader.reset();
    }
    m_audioCaptureStarted = false;
    m_audioEncoder.close();
    if (m_codecContext && m_started) {
        avcodec_flush_buffers(m_codecContext);
    }
    m_scaler.reset();
    if (m_frame) {
        av_frame_free(&m_frame);
    }
    if (m_packet) {
        av_packet_free(&m_packet);
    }
    if (m_codecContext) {
        avcodec_free_context(&m_codecContext);
    }
    m_hardware.reset();
    m_hardwareFrames = false;
    m_muxer.close();
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

bool LibavRecordingProcess::writeRepeatFrame(QString *error)
{
    return m_impl->writeRepeatFrame(error);
}

bool LibavRecordingProcess::finish(QString *error)
{
    return m_impl->finish(error);
}

void LibavRecordingProcess::setPaused(bool paused)
{
    m_impl->setPaused(paused);
}

void LibavRecordingProcess::cancel()
{
    m_impl->cancel();
}

}  // namespace markshot::recording
