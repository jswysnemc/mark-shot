#include "internal.h"

/** 【媒体库】【协议版本】无参数，返回稳定接口版本，不暴露 FFmpeg 结构体 */
uint32_t ms_media_abi(void) { return 2; }

/** 【媒体库】【错误记录】按阶段和错误码更新 encoder，返回原始负错误码 */
int ms_fail(ms_encoder *encoder, const char *stage, int code) {
    char detail[AV_ERROR_MAX_STRING_SIZE];
    av_strerror(code, detail, sizeof(detail));
    snprintf(encoder->error, sizeof(encoder->error), "%s: %s (%d)", stage, detail, code);
    encoder->failed = 1;
    return code;
}

/** 【媒体库】【错误读取】按 encoder 返回对象持有的错误，不转移内存所有权 */
const char *ms_encoder_error(ms_encoder *encoder) { return encoder->error; }

/** 【媒体库】【编码身份】按 encoder 返回实际选择的编码器名称 */
const char *ms_encoder_name(ms_encoder *encoder) { return encoder->video->codec->name; }

/** 【媒体库】【数据封装】将 frame 送入 codec 并排空已完成包到 stream，空帧表示排空编码器 */
int ms_write(ms_encoder *encoder, AVCodecContext *codec, AVStream *stream, AVFrame *frame) {
    int result = avcodec_send_frame(codec, frame);
    if (result < 0) return ms_fail(encoder, "send frame", result);
    while ((result = avcodec_receive_packet(codec, encoder->packet)) >= 0) {
        if (codec->codec_type == AVMEDIA_TYPE_VIDEO && encoder->packet->duration <= 0)
            encoder->packet->duration = av_rescale_q(1, av_inv_q(codec->framerate), codec->time_base);
        av_packet_rescale_ts(encoder->packet, codec->time_base, stream->time_base);
        encoder->packet->stream_index = stream->index;
        result = av_interleaved_write_frame(encoder->output, encoder->packet);
        av_packet_unref(encoder->packet);
        if (result < 0) return ms_fail(encoder, "write packet", result);
    }
    return result == AVERROR(EAGAIN) || result == AVERROR_EOF ? 0 : ms_fail(encoder, "receive packet", result);
}

/** 【媒体库】【会话建立】按参数创建 H.264/AAC 分片 MP4，失败填写 error 并回收全部资源 */
ms_encoder *ms_encoder_open(const char *path, const char *device, int width, int height, int fps,
    int quality, int mode, int audio, char *error, size_t capacity) {
    ms_encoder *encoder = av_mallocz(sizeof(*encoder));
    if (!encoder) { snprintf(error, capacity, "allocate encoder"); return NULL; }
    encoder->last_video = -1;
    encoder->audio_pts = AV_NOPTS_VALUE;
    int result = avformat_alloc_output_context2(&encoder->output, NULL, "mp4", path);
    if (result < 0) { ms_fail(encoder, "MP4 context", result); goto failure; }
    if (width < 2 || height < 2 || (width & 1) || (height & 1) || fps < 1 || fps > 60 ||
        (int64_t)width * height > 33554432 || quality < 0 || quality > 2 || mode < 0 || mode > 2) {
        ms_fail(encoder, "invalid configuration", AVERROR(EINVAL)); goto failure;
    }
    if (ms_video_open(encoder, device, width, height, fps, quality, mode) < 0) goto failure;
    if (audio && ms_audio_open(encoder) < 0) goto failure;
    encoder->packet = av_packet_alloc();
    if (!encoder->packet) { ms_fail(encoder, "allocate packet", AVERROR(ENOMEM)); goto failure; }
    result = avio_open(&encoder->output->pb, path, AVIO_FLAG_WRITE);
    if (result < 0) { ms_fail(encoder, "open output", result); goto failure; }
    AVDictionary *options = NULL;
    av_dict_set(&options, "movflags", "frag_keyframe+empty_moov+default_base_moof", 0);
    av_dict_set(&options, "frag_duration", "1000000", 0);
    result = avformat_write_header(encoder->output, &options);
    av_dict_free(&options);
    if (result < 0) { ms_fail(encoder, "write header", result); goto failure; }
    encoder->header_written = 1;
    encoder->failed = 0;
    encoder->error[0] = 0;
    return encoder;
failure:
    snprintf(error, capacity, "%s", encoder->error);
    ms_encoder_destroy(encoder);
    return NULL;
}

/** 【媒体库】【完整收尾】排空音视频并写容器尾部，返回首个错误，重复调用不重复编码 */
int ms_encoder_finish(ms_encoder *encoder) {
    if (encoder->finalized) return encoder->failed ? AVERROR(EIO) : 0;
    encoder->finalized = 1;
    int first = encoder->audio ? ms_audio_flush(encoder) : 0;
    int result = ms_write(encoder, encoder->video, encoder->video_stream, NULL);
    if (first >= 0) first = result;
    if (encoder->audio) {
        result = ms_write(encoder, encoder->audio, encoder->audio_stream, NULL);
        if (first >= 0) first = result;
    }
    result = av_write_trailer(encoder->output);
    if (result < 0 && first >= 0) first = ms_fail(encoder, "write trailer", result);
    avio_flush(encoder->output->pb);
    if (encoder->output->pb->error < 0 && first >= 0)
        first = ms_fail(encoder, "flush output", encoder->output->pb->error);
    return first;
}

/** 【媒体库】【资源释放】按 encoder 回收自己持有的内存和设备，异常退出尝试容器收尾 */
void ms_encoder_destroy(ms_encoder *encoder) {
    if (!encoder) return;
    if (encoder->header_written && !encoder->finalized) ms_encoder_finish(encoder);
    avcodec_free_context(&encoder->video);
    avcodec_free_context(&encoder->audio);
    av_frame_free(&encoder->pixels);
    av_frame_free(&encoder->hardware);
    av_frame_free(&encoder->sound);
    av_packet_free(&encoder->packet);
    av_buffer_unref(&encoder->device);
    sws_freeContext(encoder->scale);
    swr_free(&encoder->resample);
    av_audio_fifo_free(encoder->fifo);
    if (encoder->output) {
        if (encoder->output->pb) avio_closep(&encoder->output->pb);
        avformat_free_context(encoder->output);
    }
    av_free(encoder);
}
