#include "internal.h"

/** 【媒体库】【视频候选】按 name 尝试真实设备与编码器，失败不创建输出文件或保留半个会话 */
static int candidate(ms_encoder *encoder, const char *name, const char *device_path, int width, int height, int fps, int quality) {
    const AVCodec *codec = avcodec_find_encoder_by_name(name);
    if (!codec) return AVERROR_ENCODER_NOT_FOUND;
    AVCodecContext *video = avcodec_alloc_context3(codec);
    if (!video) return AVERROR(ENOMEM);
    AVBufferRef *device = NULL;
    int result = 0;
    video->width = width; video->height = height;
    video->time_base = AV_TIME_BASE_Q;
    video->framerate = (AVRational){fps, 1};
    video->pix_fmt = AV_PIX_FMT_YUV420P;
    video->bit_rate = (int64_t)width * height * fps * (quality == 2 ? 24 : quality == 0 ? 8 : 14) / 100;
    if (video->bit_rate < 1000000) video->bit_rate = 1000000;
    if (video->bit_rate > 80000000) video->bit_rate = 80000000;
    video->gop_size = fps * 2; video->max_b_frames = 0; video->thread_count = 2;
    video->color_primaries = AVCOL_PRI_BT709;
    video->color_trc = AVCOL_TRC_BT709;
    video->colorspace = AVCOL_SPC_BT709;
    video->color_range = AVCOL_RANGE_MPEG;
    video->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    if (!strcmp(name, "h264_vaapi")) {
        result = av_hwdevice_ctx_create(&device, AV_HWDEVICE_TYPE_VAAPI, device_path, NULL, 0);
        if (result < 0 && !device_path) {
            for (int index = 128; index < 144 && result < 0; ++index) {
                char path[64];
                snprintf(path, sizeof(path), "/dev/dri/renderD%d", index);
                result = av_hwdevice_ctx_create(&device, AV_HWDEVICE_TYPE_VAAPI, path, NULL, 0);
            }
        }
        if (result < 0) goto failed;
        video->hw_frames_ctx = av_hwframe_ctx_alloc(device);
        if (!video->hw_frames_ctx) { result = AVERROR(ENOMEM); goto failed; }
        AVHWFramesContext *frames = (AVHWFramesContext *)video->hw_frames_ctx->data;
        frames->format = AV_PIX_FMT_VAAPI; frames->sw_format = AV_PIX_FMT_NV12;
        frames->width = width; frames->height = height; frames->initial_pool_size = 4;
        result = av_hwframe_ctx_init(video->hw_frames_ctx);
        if (result < 0) goto failed;
        video->pix_fmt = AV_PIX_FMT_VAAPI;
    }
    AVDictionary *options = NULL;
    if (!strcmp(name, "h264_nvenc")) {
        av_dict_set(&options, "preset", "p1", 0);
        av_dict_set(&options, "rc-lookahead", "0", 0);
        av_dict_set(&options, "surfaces", "4", 0);
    }
    result = avcodec_open2(video, codec, &options);
    av_dict_free(&options);
    if (result < 0) goto failed;
    encoder->video = video;
    encoder->device = device;
    return 0;
failed:
    avcodec_free_context(&video);
    av_buffer_unref(&device);
    return result;
}

/** 【媒体库】【视频建立】按 mode 尝试硬件或软件候选，返回错误，不把硬件请求静默改为软件 */
int ms_video_open(ms_encoder *encoder, const char *device, int width, int height, int fps, int quality, int mode) {
    int result = AVERROR_ENCODER_NOT_FOUND;
    if (mode != 2) {
        result = candidate(encoder, "h264_vaapi", device, width, height, fps, quality);
        if (result < 0 && !device) result = candidate(encoder, "h264_nvenc", NULL, width, height, fps, quality);
    }
    if (result < 0 && mode != 1)
        result = candidate(encoder, "libopenh264", NULL, width, height, fps, quality);
    if (result < 0) return ms_fail(encoder, "open H.264 encoder", result);
    encoder->video_stream = avformat_new_stream(encoder->output, NULL);
    encoder->pixels = av_frame_alloc();
    encoder->hardware = av_frame_alloc();
    if (!encoder->video_stream || !encoder->pixels || !encoder->hardware)
        return ms_fail(encoder, "allocate video", AVERROR(ENOMEM));
    encoder->video_stream->time_base = encoder->video->time_base;
    result = avcodec_parameters_from_context(encoder->video_stream->codecpar, encoder->video);
    if (result < 0) return ms_fail(encoder, "video parameters", result);
    encoder->pixels->format = encoder->device ? AV_PIX_FMT_NV12 : encoder->video->pix_fmt;
    encoder->pixels->width = width; encoder->pixels->height = height;
    result = av_frame_get_buffer(encoder->pixels, 32);
    if (result < 0) return ms_fail(encoder, "video buffer", result);
    encoder->scale = sws_getContext(width, height, AV_PIX_FMT_RGBA, width, height,
        encoder->pixels->format, SWS_FAST_BILINEAR, NULL, NULL, NULL);
    if (!encoder->scale) return ms_fail(encoder, "pixel conversion", AVERROR(ENOMEM));
    const int *coefficients = sws_getCoefficients(SWS_CS_ITU709);
    sws_setColorspaceDetails(encoder->scale, coefficients, 1, coefficients, 0, 0, 1 << 16, 1 << 16);
    return 0;
}

/** 【媒体库】【视频输入】将 stride 对齐的 RGBA 按 pts_us 编码，硬件兼容路径只上传一次 NV12 */
int ms_encoder_video(ms_encoder *encoder, const uint8_t *rgba, int stride, int64_t pts_us) {
    if (encoder->finalized || pts_us <= encoder->last_video || stride < encoder->video->width * 4)
        return ms_fail(encoder, "invalid video frame", AVERROR(EINVAL));
    int result = av_frame_make_writable(encoder->pixels);
    if (result < 0) return ms_fail(encoder, "writable frame", result);
    const uint8_t *source[4] = {rgba};
    int strides[4] = {stride};
    result = sws_scale(encoder->scale, source, strides, 0, encoder->video->height,
        encoder->pixels->data, encoder->pixels->linesize);
    if (result < 0) return ms_fail(encoder, "convert RGBA", result);
    encoder->pixels->pts = pts_us;
    encoder->pixels->duration = av_rescale_q(1, av_inv_q(encoder->video->framerate), AV_TIME_BASE_Q);
    AVFrame *frame = encoder->pixels;
    if (encoder->device) {
        av_frame_unref(encoder->hardware);
        result = av_hwframe_get_buffer(encoder->video->hw_frames_ctx, encoder->hardware, 0);
        if (result >= 0) result = av_hwframe_transfer_data(encoder->hardware, encoder->pixels, 0);
        if (result < 0) return ms_fail(encoder, "upload NV12", result);
        av_frame_copy_props(encoder->hardware, encoder->pixels);
        frame = encoder->hardware;
    }
    result = ms_write(encoder, encoder->video, encoder->video_stream, frame);
    if (result >= 0) encoder->last_video = pts_us;
    return result;
}
