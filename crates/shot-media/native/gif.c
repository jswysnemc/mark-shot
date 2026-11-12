#include "internal.h"
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersrc.h>
#include <libavfilter/buffersink.h>

typedef struct {
    AVCodecContext *decoder, *encoder;
    AVFilterGraph *graph;
    AVFilterContext *source, *sink;
    AVStream *stream;
    AVFormatContext *output;
    AVPacket *input_packet, *output_packet;
    AVFrame *decoded, *filtered;
} gif_state;

/** 【媒体导出】【GIF编码】按 state 送入 frame 并写出全部压缩包，空帧排空编码器 */
static int encode(gif_state *state, AVFrame *frame) {
    int result = avcodec_send_frame(state->encoder, frame);
    if (result < 0) return result;
    while ((result = avcodec_receive_packet(state->encoder, state->output_packet)) >= 0) {
        av_packet_rescale_ts(state->output_packet, state->encoder->time_base, state->stream->time_base);
        state->output_packet->stream_index = state->stream->index;
        result = av_interleaved_write_frame(state->output, state->output_packet);
        av_packet_unref(state->output_packet);
        if (result < 0) return result;
    }
    return result == AVERROR(EAGAIN) || result == AVERROR_EOF ? 0 : result;
}

/** 【媒体导出】【量化输出】排空 state 图像过滤器，输出帧后立即归还缓冲，返回错误 */
static int filtered(gif_state *state) {
    int result;
    while ((result = av_buffersink_get_frame(state->sink, state->filtered)) >= 0) {
        result = encode(state, state->filtered);
        av_frame_unref(state->filtered);
        if (result < 0) return result;
    }
    return result == AVERROR(EAGAIN) || result == AVERROR_EOF ? 0 : result;
}

/** 【媒体导出】【视频解码】按 state 读取解码帧并送入逐帧调色板，返回错误 */
static int decoded(gif_state *state) {
    int result;
    while ((result = avcodec_receive_frame(state->decoder, state->decoded)) >= 0) {
        state->decoded->pts = state->decoded->best_effort_timestamp;
        result = av_buffersrc_add_frame_flags(state->source, state->decoded, AV_BUFFERSRC_FLAG_KEEP_REF);
        av_frame_unref(state->decoded);
        if (result < 0 || (result = filtered(state)) < 0) return result;
    }
    return result == AVERROR(EAGAIN) || result == AVERROR_EOF ? 0 : result;
}

/** 【媒体导出】【GIF会话】将 input 视频按 fps 流式量化写入 output，cancel 提供有界取消检查 */
int ms_gif(AVFormatContext *input, AVFormatContext *output, int fps, ms_cancel cancel, void *context) {
    gif_state state = {.output = output};
    int index = av_find_best_stream(input, AVMEDIA_TYPE_VIDEO, -1, -1, NULL, 0);
    int result = index;
    if (index < 0) goto end;
    AVStream *video = input->streams[index];
    const AVCodec *decoder = avcodec_find_decoder(video->codecpar->codec_id);
    const AVCodec *encoder = avcodec_find_encoder(AV_CODEC_ID_GIF);
    if (!decoder || !encoder) { result = AVERROR_DECODER_NOT_FOUND; goto end; }
    state.decoder = avcodec_alloc_context3(decoder);
    state.encoder = avcodec_alloc_context3(encoder);
    state.graph = avfilter_graph_alloc();
    state.input_packet = av_packet_alloc(); state.output_packet = av_packet_alloc();
    state.decoded = av_frame_alloc(); state.filtered = av_frame_alloc();
    state.stream = avformat_new_stream(output, NULL);
    if (!state.decoder || !state.encoder || !state.graph || !state.input_packet ||
        !state.output_packet || !state.decoded || !state.filtered || !state.stream) { result = AVERROR(ENOMEM); goto end; }
    result = avcodec_parameters_to_context(state.decoder, video->codecpar);
    if (result < 0) goto end;
    state.decoder->thread_count = 2;
    if ((result = avcodec_open2(state.decoder, decoder, NULL)) < 0) goto end;
    char arguments[256];
    snprintf(arguments, sizeof(arguments), "video_size=%dx%d:pix_fmt=%d:time_base=%d/%d:pixel_aspect=1/1",
        state.decoder->width, state.decoder->height, state.decoder->pix_fmt, video->time_base.num, video->time_base.den);
    state.graph->nb_threads = 2;
    result = avfilter_graph_create_filter(&state.source, avfilter_get_by_name("buffer"), "in", arguments, NULL, state.graph);
    if (result < 0) goto end;
    result = avfilter_graph_create_filter(&state.sink, avfilter_get_by_name("buffersink"), "out", NULL, NULL, state.graph);
    if (result < 0) goto end;
    AVFilterInOut *inputs = avfilter_inout_alloc(), *outputs = avfilter_inout_alloc();
    if (!inputs || !outputs) { avfilter_inout_free(&inputs); avfilter_inout_free(&outputs); result = AVERROR(ENOMEM); goto end; }
    inputs->name = av_strdup("out"); inputs->filter_ctx = state.sink;
    outputs->name = av_strdup("in"); outputs->filter_ctx = state.source;
    // 1. 【媒体导出】【有界调色板】每帧生成调色板，不把整段视频保留到全局颜色统计结束
    snprintf(arguments, sizeof(arguments), "fps=%d,split[a][b];[a]palettegen=stats_mode=single[p];[b][p]paletteuse=new=1:dither=sierra2_4a", fps);
    result = avfilter_graph_parse_ptr(state.graph, arguments, &inputs, &outputs, NULL);
    avfilter_inout_free(&inputs); avfilter_inout_free(&outputs);
    if (result < 0 || (result = avfilter_graph_config(state.graph, NULL)) < 0) goto end;
    state.encoder->width = state.decoder->width; state.encoder->height = state.decoder->height;
    state.encoder->pix_fmt = AV_PIX_FMT_PAL8;
    state.encoder->time_base = av_buffersink_get_time_base(state.sink);
    if ((result = avcodec_open2(state.encoder, encoder, NULL)) < 0) goto end;
    state.stream->time_base = state.encoder->time_base;
    if ((result = avcodec_parameters_from_context(state.stream->codecpar, state.encoder)) < 0) goto end;
    if ((result = avformat_write_header(output, NULL)) < 0) goto end;
    while ((result = av_read_frame(input, state.input_packet)) >= 0) {
        if (cancel && cancel(context)) { result = AVERROR_EXIT; break; }
        if (state.input_packet->stream_index == index) {
            result = avcodec_send_packet(state.decoder, state.input_packet);
            if (result >= 0) result = decoded(&state);
        }
        av_packet_unref(state.input_packet);
        if (result < 0) break;
    }
    if (result == AVERROR_EOF) {
        result = avcodec_send_packet(state.decoder, NULL);
        if (result >= 0) result = decoded(&state);
        if (result >= 0) result = av_buffersrc_add_frame_flags(state.source, NULL, 0);
        if (result >= 0) result = filtered(&state);
        if (result >= 0) result = encode(&state, NULL);
        if (result >= 0) result = av_write_trailer(output);
    }
end:
    avcodec_free_context(&state.decoder); avcodec_free_context(&state.encoder);
    avfilter_graph_free(&state.graph);
    av_frame_free(&state.decoded); av_frame_free(&state.filtered);
    av_packet_free(&state.input_packet); av_packet_free(&state.output_packet);
    return result;
}
