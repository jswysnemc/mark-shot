#include "recording/libav_gif_recording_process.h"

#include "recording/libav_error.h"
#include "recording/recording_frame_converter.h"

#include <QByteArray>
#include <QFile>

#include <algorithm>

#ifdef HAVE_LIBAV_RECORDING
extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/opt.h>
#include <libswscale/swscale.h>
}
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

}  // namespace

class LibavGifRecordingProcess::Private final {
public:
    bool start(const QString &outputPath, QSize frameSize, int fps, QString *error);
    bool writeFrame(const RecordingFrameSample &sample, QString *error);
    bool finish(QString *error);
    void cancel();

private:
    RecordingBgraFrame bgraBytesForSample(const RecordingFrameSample &sample, QString *error);
    bool fillFrame(RecordingBgraFrame bytes, QString *error);
#ifdef HAVE_LIBAV_RECORDING
    bool encodeFrame(AVFrame *frame, QString *error);
#endif
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
    QByteArray m_outputPathBytes;
    QSize m_frameSize;
    int m_fps = 12;
    int64_t m_nextPts = 0;
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
    m_nextPts = 0;

    m_outputPathBytes = QFile::encodeName(outputPath);
    int result = avformat_alloc_output_context2(&m_formatContext,
                                                nullptr,
                                                "gif",
                                                m_outputPathBytes.constData());
    if (result < 0 || !m_formatContext) {
        return failWith(error,
                        QStringLiteral("Failed to allocate GIF output context: %1")
                            .arg(libavErrorText(result)));
    }

    const AVCodec *codec = avcodec_find_encoder_by_name("gif");
    if (!codec) {
        return failWith(error, QStringLiteral("GIF encoder is not available in FFmpeg libraries"));
    }
    m_stream = avformat_new_stream(m_formatContext, codec);
    if (!m_stream) {
        return failWith(error, QStringLiteral("Failed to create GIF video stream"));
    }

    m_codecContext = avcodec_alloc_context3(codec);
    if (!m_codecContext) {
        return failWith(error, QStringLiteral("Failed to allocate GIF codec context"));
    }
    m_codecContext->width = m_frameSize.width();
    m_codecContext->height = m_frameSize.height();
    m_codecContext->pix_fmt = AV_PIX_FMT_RGB8;
    m_codecContext->time_base = AVRational{1, m_fps};
    m_codecContext->framerate = AVRational{m_fps, 1};
    if (m_formatContext->oformat->flags & AVFMT_GLOBALHEADER) {
        m_codecContext->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }

    result = avcodec_open2(m_codecContext, codec, nullptr);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to open GIF encoder: %1")
                            .arg(libavErrorText(result)));
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
    m_frame->format = m_codecContext->pix_fmt;
    m_frame->width = m_codecContext->width;
    m_frame->height = m_codecContext->height;
    result = av_frame_get_buffer(m_frame, 32);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to allocate GIF frame buffer: %1")
                            .arg(libavErrorText(result)));
    }

    m_swsContext = sws_getContext(m_frameSize.width(),
                                  m_frameSize.height(),
                                  AV_PIX_FMT_BGRA,
                                  m_frameSize.width(),
                                  m_frameSize.height(),
                                  AV_PIX_FMT_RGB8,
                                  SWS_FAST_BILINEAR,
                                  nullptr,
                                  nullptr,
                                  nullptr);
    if (!m_swsContext) {
        return failWith(error, QStringLiteral("Failed to create GIF scale context"));
    }

    result = avio_open(&m_formatContext->pb, m_outputPathBytes.constData(), AVIO_FLAG_WRITE);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to open GIF output file: %1")
                            .arg(libavErrorText(result)));
    }
    result = avformat_write_header(m_formatContext, nullptr);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to write GIF header: %1")
                            .arg(libavErrorText(result)));
    }
    m_started = true;
    return true;
#endif
}

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
    if (!fillFrame(bytes, error)) {
        return false;
    }
    m_frame->pts = m_nextPts++;
    return encodeFrame(m_frame, error);
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
    if (!encodeFrame(nullptr, error)) {
        cleanup();
        return false;
    }
    const int result = av_write_trailer(m_formatContext);
    cleanup();
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to write GIF trailer: %1")
                            .arg(libavErrorText(result)));
    }
    return true;
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
        const int rowBytes = m_frameSize.width() * 4;
        if (sample.bgra.stride == rowBytes) {
            return {sample.bgra.bytes.constData(), sample.bgra.bytes.size()};
        }
    }
    if (!sample.image.isNull()) {
        return m_converter.convertToBgra(sample.image, m_frameSize, error);
    }
    failWith(error, QStringLiteral("Cannot write an empty GIF frame"));
    return {};
}

bool LibavGifRecordingProcess::Private::fillFrame(RecordingBgraFrame bytes, QString *error)
{
#ifndef HAVE_LIBAV_RECORDING
    Q_UNUSED(bytes)
    return failWith(error, QStringLiteral("FFmpeg libraries are not linked"));
#else
    const int result = av_frame_make_writable(m_frame);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to make GIF frame writable: %1")
                            .arg(libavErrorText(result)));
    }
    const uint8_t *sourceData[] = {reinterpret_cast<const uint8_t *>(bytes.data)};
    const int sourceLineSize[] = {m_frameSize.width() * 4};
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

#ifdef HAVE_LIBAV_RECORDING
bool LibavGifRecordingProcess::Private::encodeFrame(AVFrame *frame, QString *error)
{
    int result = avcodec_send_frame(m_codecContext, frame);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to send GIF frame: %1")
                            .arg(libavErrorText(result)));
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
        result = av_interleaved_write_frame(m_formatContext, m_packet);
        av_packet_unref(m_packet);
        if (result < 0) {
            return failWith(error,
                            QStringLiteral("Failed to write GIF packet: %1")
                                .arg(libavErrorText(result)));
        }
    }
    return true;
}
#endif

void LibavGifRecordingProcess::Private::cleanup()
{
#ifdef HAVE_LIBAV_RECORDING
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
