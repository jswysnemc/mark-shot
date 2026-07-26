#include "recording/libav_gif_recording_process.h"

#include "recording/libav_error.h"
#include "recording/recording_frame_converter.h"

#include <QByteArray>

#include <algorithm>

#ifdef HAVE_LIBAV_RECORDING
#include "recording/libav/libav_muxer.h"
#include "recording/libav/libav_video_scaler.h"

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/opt.h>
}
#endif

#ifdef HAVE_LIBAVFILTER
#include "recording/libav/libav_gif_palette_filter.h"
#endif

namespace markshot::recording {
namespace {

#ifdef HAVE_LIBAV_RECORDING
// GIF 以 1/100 秒为帧延迟单位，按该时间基写 pts 可直接表达变帧率
constexpr int kGifTimeBase = 100;

/**
 * 读取 GIF 编码使用的输入像素格式。
 * @return 有调色板滤镜时为 RGB24，否则为固定调色板 RGB8。
 */
AVPixelFormat gifScalerFormat()
{
#ifdef HAVE_LIBAVFILTER
    return AV_PIX_FMT_RGB24;
#else
    return AV_PIX_FMT_RGB8;
#endif
}
#endif

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

}  // namespace

class LibavGifRecordingProcess::Private final {
public:
    /**
     * 启动 GIF 编码。
     * @param outputPath 输出路径。
     * @param frameSize 帧尺寸。
     * @param fps 目标帧率。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(const QString &outputPath, QSize frameSize, int fps, QString *error);

    /**
     * 写入一帧 GIF 画面。
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
     * 把录制样本转换为连续 BGRA 字节。
     * @param sample 录制帧样本。
     * @param error 输出错误信息。
     * @return BGRA 字节视图。
     */
    RecordingBgraFrame bgraBytesForSample(const RecordingFrameSample &sample, QString *error);

#ifdef HAVE_LIBAV_RECORDING
    /**
     * 初始化 GIF 编码器与输出流。
     * @param error 输出错误信息。
     * @return 初始化成功时返回 true。
     */
    bool openEncoder(QString *error);

    /**
     * 把量化后的帧送入编码器并写出 packet。
     * @param frame 输入帧，传空表示冲刷编码器。
     * @param error 输出错误信息。
     * @return 写出成功时返回 true。
     */
    bool encodeFrame(AVFrame *frame, QString *error);

    /**
     * 把已填充的输入帧送去量化并编码。
     * @param error 输出错误信息。
     * @return 处理成功时返回 true。
     */
    bool quantizeAndEncode(QString *error);

    /**
     * 计算帧在 GIF 时间基上的 pts。
     * @param timestampMs 采集时间戳。
     * @return 严格递增的 pts。
     */
    int64_t nextPtsFor(qint64 timestampMs);
#endif

    /**
     * 释放全部资源。
     * @return 无返回值。
     */
    void cleanup();

#ifdef HAVE_LIBAV_RECORDING
    LibavMuxer m_muxer;
    LibavVideoScaler m_scaler;
    AVCodecContext *m_codecContext = nullptr;
    AVStream *m_stream = nullptr;
    AVFrame *m_frame = nullptr;
    AVPacket *m_packet = nullptr;
#endif
#ifdef HAVE_LIBAVFILTER
    LibavGifPaletteFilter m_paletteFilter;
#endif
    RecordingFrameConverter m_converter;
    QSize m_frameSize;
    int m_fps = 12;
    int64_t m_lastPts = -1;
    bool m_started = false;
};

bool LibavGifRecordingProcess::Private::start(const QString &outputPath,
                                              QSize frameSize,
                                              int fps,
                                              QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(outputPath)
    Q_UNUSED(frameSize)
    Q_UNUSED(fps)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    if (frameSize.isEmpty()) {
        return failWith(error, QStringLiteral("Cannot start GIF writer with an empty frame size"));
    }
    cleanup();
    m_frameSize = frameSize;
    m_fps = std::max(1, fps);
    m_lastPts = -1;

    if (!m_muxer.open(outputPath, QStringLiteral("gif"), error) || !openEncoder(error)) {
        cleanup();
        return false;
    }

    // 1. 让播放器无限循环播放录制结果
    if (m_muxer.formatContext()->priv_data) {
        av_opt_set(m_muxer.formatContext()->priv_data, "loop", "0", 0);
    }
    if (!m_muxer.writeHeader(error)) {
        cleanup();
        return false;
    }

    // 2. 像素转换目标随是否启用调色板滤镜而变
    if (!m_scaler.prepare(m_frameSize, m_frameSize, gifScalerFormat(), error)) {
        cleanup();
        return false;
    }

#ifdef HAVE_LIBAVFILTER
    if (!m_paletteFilter.open(m_frameSize, kGifTimeBase, error)) {
        cleanup();
        return false;
    }
#endif
    m_started = true;
    return true;
#endif
}

#ifdef HAVE_LIBAV_RECORDING
bool LibavGifRecordingProcess::Private::openEncoder(QString *error)
{
    const AVCodec *codec = avcodec_find_encoder_by_name("gif");
    if (!codec) {
        return failWith(error, QStringLiteral("GIF encoder is not available in FFmpeg libraries"));
    }
    m_stream = avformat_new_stream(m_muxer.formatContext(), codec);
    if (!m_stream) {
        return failWith(error, QStringLiteral("Failed to create GIF video stream"));
    }

    m_codecContext = avcodec_alloc_context3(codec);
    if (!m_codecContext) {
        return failWith(error, QStringLiteral("Failed to allocate GIF codec context"));
    }
    m_codecContext->width = m_frameSize.width();
    m_codecContext->height = m_frameSize.height();
#ifdef HAVE_LIBAVFILTER
    m_codecContext->pix_fmt = AV_PIX_FMT_PAL8;
#else
    m_codecContext->pix_fmt = AV_PIX_FMT_RGB8;
#endif
    m_codecContext->time_base = AVRational{1, kGifTimeBase};
    m_codecContext->framerate = AVRational{m_fps, 1};
    if (m_muxer.needsGlobalHeader()) {
        m_codecContext->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }

    int result = avcodec_open2(m_codecContext, codec, nullptr);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to open GIF encoder: %1").arg(libavErrorText(result)));
    }
    result = avcodec_parameters_from_context(m_stream->codecpar, m_codecContext);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to copy GIF codec parameters: %1")
                            .arg(libavErrorText(result)));
    }
    m_stream->time_base = m_codecContext->time_base;

    m_frame = av_frame_alloc();
    m_packet = av_packet_alloc();
    if (!m_frame || !m_packet) {
        return failWith(error, QStringLiteral("Failed to allocate GIF frame or packet"));
    }
    m_frame->format = gifScalerFormat();
    m_frame->width = m_frameSize.width();
    m_frame->height = m_frameSize.height();
    result = av_frame_get_buffer(m_frame, 32);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to allocate GIF frame buffer: %1")
                            .arg(libavErrorText(result)));
    }
    return true;
}

int64_t LibavGifRecordingProcess::Private::nextPtsFor(qint64 timestampMs)
{
    const int64_t scaled = std::max<qint64>(0, timestampMs) * kGifTimeBase / 1000;
    const int64_t pts = std::max<int64_t>(scaled, m_lastPts + 1);
    m_lastPts = pts;
    return pts;
}

bool LibavGifRecordingProcess::Private::quantizeAndEncode(QString *error)
{
#ifdef HAVE_LIBAVFILTER
    // 逐帧生成局部调色板后再编码
    if (!m_paletteFilter.sendFrame(m_frame, error)) {
        return false;
    }
    for (;;) {
        AVFrame *quantized = nullptr;
        if (!m_paletteFilter.receiveFrame(&quantized, error)) {
            return false;
        }
        if (!quantized) {
            return true;
        }
        if (!encodeFrame(quantized, error)) {
            return false;
        }
    }
#else
    return encodeFrame(m_frame, error);
#endif
}

bool LibavGifRecordingProcess::Private::encodeFrame(AVFrame *frame, QString *error)
{
    int result = avcodec_send_frame(m_codecContext, frame);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to send GIF frame: %1").arg(libavErrorText(result)));
    }
    while (result >= 0) {
        result = avcodec_receive_packet(m_codecContext, m_packet);
        if (result == AVERROR(EAGAIN) || result == AVERROR_EOF) {
            return true;
        }
        if (result < 0) {
            return failWith(error,
                            QStringLiteral("Failed to receive GIF packet: %1")
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
}
#endif

bool LibavGifRecordingProcess::Private::writeFrame(const RecordingFrameSample &sample,
                                                   QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(sample)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    if (!m_started) {
        return failWith(error, QStringLiteral("GIF writer is not started"));
    }

    const RecordingBgraFrame bytes = bgraBytesForSample(sample, error);
    if (!bytes.data || bytes.size <= 0) {
        return false;
    }
    const int result = av_frame_make_writable(m_frame);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to make GIF frame writable: %1")
                            .arg(libavErrorText(result)));
    }

    const int stride = bytes.stride > 0 ? bytes.stride : m_frameSize.width() * 4;
    if (!m_scaler.convert(bytes.data, stride, m_frameSize.height(), bytes.yInverted, m_frame, error)) {
        return false;
    }
    m_frame->pts = nextPtsFor(sample.timestampMs);
    return quantizeAndEncode(error);
#endif
}

bool LibavGifRecordingProcess::Private::finish(QString *error)
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

#ifdef HAVE_LIBAVFILTER
    // 1. 冲刷调色板滤镜链中滞留的帧
    if (!m_paletteFilter.sendFrame(nullptr, error)) {
        cleanup();
        return false;
    }
    for (;;) {
        AVFrame *quantized = nullptr;
        if (!m_paletteFilter.receiveFrame(&quantized, error)) {
            cleanup();
            return false;
        }
        if (!quantized) {
            break;
        }
        if (!encodeFrame(quantized, error)) {
            cleanup();
            return false;
        }
    }
#endif

    // 2. 冲刷编码器并写出容器尾
    if (!encodeFrame(nullptr, error)) {
        cleanup();
        return false;
    }
    const bool ok = m_muxer.writeTrailer(error);
    cleanup();
    return ok;
#endif
}

void LibavGifRecordingProcess::Private::cancel()
{
    cleanup();
}

RecordingBgraFrame LibavGifRecordingProcess::Private::bgraBytesForSample(
    const RecordingFrameSample &sample,
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
    failWith(error, QStringLiteral("Cannot write an empty GIF frame"));
    return {};
}

void LibavGifRecordingProcess::Private::cleanup()
{
#ifdef HAVE_LIBAVFILTER
    m_paletteFilter.close();
#endif
#ifdef HAVE_LIBAV_RECORDING
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
    m_muxer.close();
    m_stream = nullptr;
#endif
    m_lastPts = -1;
    m_started = false;
}

LibavGifRecordingProcess::LibavGifRecordingProcess()
    : d(new Private)
{
}

LibavGifRecordingProcess::~LibavGifRecordingProcess()
{
    cancel();
    delete d;
}

bool LibavGifRecordingProcess::start(const QString &outputPath,
                                     QSize frameSize,
                                     int fps,
                                     QString *error)
{
    return d->start(outputPath, frameSize, fps, error);
}

bool LibavGifRecordingProcess::writeFrame(const RecordingFrameSample &sample, QString *error)
{
    return d->writeFrame(sample, error);
}

bool LibavGifRecordingProcess::finish(QString *error)
{
    return d->finish(error);
}

void LibavGifRecordingProcess::cancel()
{
    d->cancel();
}

}  // namespace markshot::recording
