#include "internal.h"

/** 【媒体导出】【中断检查】按 context 包装的用户回调返回是否请求取消 */
typedef struct { ms_cancel cancel; void *context; } interruption;
static int interrupted(void *context) {
    interruption *state = context;
    return state->cancel && state->cancel(state->context);
}

/** 【媒体导出】【重封装】将 input 音视频压缩包写入 output，cancel 检查取消，不重新编码 */
int ms_remux(AVFormatContext *input, AVFormatContext *output, ms_cancel cancel, void *context) {
    int *mapping = av_malloc_array(input->nb_streams, sizeof(int));
    AVPacket *packet = av_packet_alloc();
    int result = 0;
    if (!mapping || !packet) { result = AVERROR(ENOMEM); goto end; }
    for (unsigned i = 0; i < input->nb_streams; ++i) {
        AVStream *source = input->streams[i];
        mapping[i] = -1;
        if (source->codecpar->codec_type != AVMEDIA_TYPE_VIDEO && source->codecpar->codec_type != AVMEDIA_TYPE_AUDIO) continue;
        AVStream *target = avformat_new_stream(output, NULL);
        if (!target) { result = AVERROR(ENOMEM); goto end; }
        mapping[i] = target->index;
        result = avcodec_parameters_copy(target->codecpar, source->codecpar);
        if (result < 0) goto end;
        target->codecpar->codec_tag = 0;
        target->time_base = source->time_base;
    }
    result = avformat_write_header(output, NULL);
    if (result < 0) goto end;
    while ((result = av_read_frame(input, packet)) >= 0) {
        if (cancel && cancel(context)) { result = AVERROR_EXIT; break; }
        int index = mapping[packet->stream_index];
        if (index >= 0) {
            av_packet_rescale_ts(packet, input->streams[packet->stream_index]->time_base, output->streams[index]->time_base);
            packet->stream_index = index;
            packet->pos = -1;
            result = av_interleaved_write_frame(output, packet);
        }
        av_packet_unref(packet);
        if (result < 0) break;
    }
    if (result == AVERROR_EOF) result = av_write_trailer(output);
end:
    av_free(mapping);
    av_packet_free(&packet);
    return result;
}

/** 【媒体导出】【文件转换】按 gif_fps 选择 GIF 或无损重封装，支持取消，错误写入调用方缓冲 */
int ms_export(const char *source, const char *destination, int gif_fps,
    ms_cancel cancel, void *context, char *error, size_t capacity) {
    AVFormatContext *input = avformat_alloc_context(), *output = NULL;
    interruption state = {cancel, context};
    int result = AVERROR(ENOMEM);
    if (!input) goto end;
    input->interrupt_callback = (AVIOInterruptCB){interrupted, &state};
    result = avformat_open_input(&input, source, NULL, NULL);
    if (result < 0) goto end;
    result = avformat_find_stream_info(input, NULL);
    if (result < 0) goto end;
    result = avformat_alloc_output_context2(&output, NULL, gif_fps > 0 ? "gif" : "matroska", destination);
    if (result < 0) goto end;
    output->interrupt_callback = (AVIOInterruptCB){interrupted, &state};
    result = avio_open2(&output->pb, destination, AVIO_FLAG_WRITE, &output->interrupt_callback, NULL);
    if (result < 0) goto end;
    result = gif_fps > 0 ? ms_gif(input, output, gif_fps, cancel, context) : ms_remux(input, output, cancel, context);
    avio_flush(output->pb);
    if (result >= 0 && output->pb->error < 0) result = output->pb->error;
end:
    if (result < 0) av_strerror(result, error, capacity);
    avformat_close_input(&input);
    if (output) {
        if (output->pb) avio_closep(&output->pb);
        avformat_free_context(output);
    }
    return result;
}
