#include "recording/libav/libav_gif_palette_filter.h"

#ifdef HAVE_LIBAVFILTER

#include "recording/libav_error.h"

#include <QByteArray>

extern "C" {
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavutil/opt.h>
}

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
 * 【录制】【GIF调色板】读取滤镜链描述。
 * @return 逐帧调色板的滤镜链描述。
 */
QByteArray paletteFilterDescription()
{
    // stats_mode=single 让每帧独立统计调色板，new=1 让量化按帧切换调色板
    // bayer 抖动在屏幕内容上比误差扩散更稳定，也不会放大帧间噪点
    return QByteArrayLiteral(
        "[in]split[a][b];"
        "[a]palettegen=stats_mode=single[p];"
        "[b][p]paletteuse=new=1:dither=bayer:bayer_scale=4[out]");
}

}  // namespace

LibavGifPaletteFilter::~LibavGifPaletteFilter()
{
    close();
}

bool LibavGifPaletteFilter::open(QSize size, int timeBase, QString *error)
{
    if (error) {
        error->clear();
    }
    close();
    if (size.isEmpty() || timeBase <= 0) {
        return failWith(error, QStringLiteral("Cannot build GIF palette filter with invalid parameters"));
    }

    m_graph = avfilter_graph_alloc();
    if (!m_graph) {
        return failWith(error, QStringLiteral("Failed to allocate GIF filter graph"));
    }

    // 1. 创建输入与输出端点
    const QByteArray sourceArgs = QStringLiteral("video_size=%1x%2:pix_fmt=%3:time_base=1/%4:pixel_aspect=1/1")
                                      .arg(size.width())
                                      .arg(size.height())
                                      .arg(static_cast<int>(AV_PIX_FMT_RGB24))
                                      .arg(timeBase)
                                      .toUtf8();
    int result = avfilter_graph_create_filter(&m_source,
                                              avfilter_get_by_name("buffer"),
                                              "in",
                                              sourceArgs.constData(),
                                              nullptr,
                                              m_graph);
    if (result < 0) {
        close();
        return failWith(error,
                        QStringLiteral("Failed to create GIF filter source: %1")
                            .arg(libavErrorText(result)));
    }

    result = avfilter_graph_create_filter(&m_sink,
                                          avfilter_get_by_name("buffersink"),
                                          "out",
                                          nullptr,
                                          nullptr,
                                          m_graph);
    if (result < 0) {
        close();
        return failWith(error,
                        QStringLiteral("Failed to create GIF filter sink: %1")
                            .arg(libavErrorText(result)));
    }

    // 2. 解析并连接调色板滤镜链，paletteuse 的输出天然是 PAL8
    AVFilterInOut *outputs = avfilter_inout_alloc();
    AVFilterInOut *inputs = avfilter_inout_alloc();
    if (!outputs || !inputs) {
        avfilter_inout_free(&outputs);
        avfilter_inout_free(&inputs);
        close();
        return failWith(error, QStringLiteral("Failed to allocate GIF filter endpoints"));
    }
    outputs->name = av_strdup("in");
    outputs->filter_ctx = m_source;
    outputs->pad_idx = 0;
    outputs->next = nullptr;
    inputs->name = av_strdup("out");
    inputs->filter_ctx = m_sink;
    inputs->pad_idx = 0;
    inputs->next = nullptr;

    const QByteArray description = paletteFilterDescription();
    result = avfilter_graph_parse_ptr(m_graph, description.constData(), &inputs, &outputs, nullptr);
    avfilter_inout_free(&outputs);
    avfilter_inout_free(&inputs);
    if (result < 0) {
        close();
        return failWith(error,
                        QStringLiteral("Failed to parse GIF palette filter: %1")
                            .arg(libavErrorText(result)));
    }

    result = avfilter_graph_config(m_graph, nullptr);
    if (result < 0) {
        close();
        return failWith(error,
                        QStringLiteral("Failed to configure GIF palette filter: %1")
                            .arg(libavErrorText(result)));
    }

    // 3. 输出格式必须与 GIF 编码器输入一致，不一致时直接失败并回退
    if (av_buffersink_get_format(m_sink) != AV_PIX_FMT_PAL8) {
        close();
        return failWith(error, QStringLiteral("GIF palette filter did not produce a PAL8 output"));
    }

    m_output = av_frame_alloc();
    if (!m_output) {
        close();
        return failWith(error, QStringLiteral("Failed to allocate GIF filter output frame"));
    }
    return true;
}

bool LibavGifPaletteFilter::sendFrame(AVFrame *input, QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_graph || !m_source) {
        return failWith(error, QStringLiteral("GIF palette filter is not open"));
    }

    // 传空帧表示输入结束，滤镜链会据此冲刷剩余输出
    const int result = input
        ? av_buffersrc_add_frame_flags(m_source, input, AV_BUFFERSRC_FLAG_KEEP_REF)
        : av_buffersrc_add_frame(m_source, nullptr);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to feed GIF palette filter: %1")
                            .arg(libavErrorText(result)));
    }
    return true;
}

bool LibavGifPaletteFilter::receiveFrame(AVFrame **output, QString *error)
{
    if (error) {
        error->clear();
    }
    if (output) {
        *output = nullptr;
    }
    if (!m_graph || !m_sink || !m_output) {
        return failWith(error, QStringLiteral("GIF palette filter is not open"));
    }

    av_frame_unref(m_output);
    const int result = av_buffersink_get_frame(m_sink, m_output);
    if (result == AVERROR(EAGAIN) || result == AVERROR_EOF) {
        return true;
    }
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to read GIF palette filter output: %1")
                            .arg(libavErrorText(result)));
    }
    if (output) {
        *output = m_output;
    }
    return true;
}

void LibavGifPaletteFilter::close()
{
    if (m_output) {
        av_frame_free(&m_output);
    }
    if (m_graph) {
        avfilter_graph_free(&m_graph);
        m_graph = nullptr;
    }
    m_source = nullptr;
    m_sink = nullptr;
}

}  // namespace markshot::recording

#endif  // HAVE_LIBAVFILTER
