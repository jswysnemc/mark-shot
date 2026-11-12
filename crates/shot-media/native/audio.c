#include "internal.h"

/** 【媒体库】【音频建立】为 encoder 创建 48kHz 双声道 AAC 与有界转换缓冲，返回媒体错误 */
int ms_audio_open(ms_encoder *encoder) {
    const AVCodec *codec = avcodec_find_encoder(AV_CODEC_ID_AAC);
    if (!codec) return ms_fail(encoder, "AAC unavailable", AVERROR_ENCODER_NOT_FOUND);
    encoder->audio = avcodec_alloc_context3(codec);
    encoder->sound = av_frame_alloc();
    encoder->audio_stream = avformat_new_stream(encoder->output, NULL);
    if (!encoder->audio || !encoder->sound || !encoder->audio_stream)
        return ms_fail(encoder, "audio allocation", AVERROR(ENOMEM));
    AVCodecContext *audio = encoder->audio;
    audio->sample_rate = 48000; audio->sample_fmt = AV_SAMPLE_FMT_FLTP;
    audio->time_base = (AVRational){1, 48000}; audio->bit_rate = 192000;
    audio->ch_layout = (AVChannelLayout)AV_CHANNEL_LAYOUT_STEREO;
    audio->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    int result = avcodec_open2(audio, codec, NULL);
    if (result < 0) return ms_fail(encoder, "open AAC", result);
    encoder->audio_stream->time_base = audio->time_base;
    result = avcodec_parameters_from_context(encoder->audio_stream->codecpar, audio);
    if (result < 0) return ms_fail(encoder, "audio parameters", result);
    encoder->sound->sample_rate = audio->sample_rate;
    encoder->sound->format = audio->sample_fmt;
    av_channel_layout_copy(&encoder->sound->ch_layout, &audio->ch_layout);
    encoder->sound->nb_samples = audio->frame_size;
    result = av_frame_get_buffer(encoder->sound, 0);
    if (result < 0) return ms_fail(encoder, "audio frame", result);
    encoder->fifo = av_audio_fifo_alloc(audio->sample_fmt, 2, audio->frame_size * 2);
    if (!encoder->fifo) return ms_fail(encoder, "audio FIFO", AVERROR(ENOMEM));
    result = swr_alloc_set_opts2(&encoder->resample, &audio->ch_layout, audio->sample_fmt, 48000,
        &audio->ch_layout, AV_SAMPLE_FMT_FLT, 48000, 0, NULL);
    if (result >= 0) result = swr_init(encoder->resample);
    return result < 0 ? ms_fail(encoder, "audio conversion", result) : 0;
}

/** 【媒体库】【音频排队】将已转换 sound 的 count 样本排入 FIFO 并编码整帧，返回错误 */
static int queue(ms_encoder *encoder, int count) {
    if (av_audio_fifo_write(encoder->fifo, (void **)encoder->sound->data, count) < count)
        return ms_fail(encoder, "audio queue", AVERROR(ENOMEM));
    int size = encoder->audio->frame_size;
    while (av_audio_fifo_size(encoder->fifo) >= size) {
        int result = av_frame_make_writable(encoder->sound);
        if (result < 0) return ms_fail(encoder, "audio writable", result);
        av_audio_fifo_read(encoder->fifo, (void **)encoder->sound->data, size);
        encoder->sound->nb_samples = size;
        encoder->sound->pts = encoder->audio_pts;
        encoder->audio_pts += size;
        result = ms_write(encoder, encoder->audio, encoder->audio_stream, encoder->sound);
        if (result < 0) return result;
    }
    return 0;
}

/** 【媒体库】【PCM输入】按 samples、frames 和 pts_us 交付交错浮点双声道音频，输入不超过一秒 */
int ms_encoder_audio(ms_encoder *encoder, const float *samples, int frames, int64_t pts_us) {
    if (!encoder->audio || encoder->finalized || frames < 0 || frames > 48000 || pts_us < 0)
        return ms_fail(encoder, "invalid audio", AVERROR(EINVAL));
    int64_t position = av_rescale_q(pts_us, AV_TIME_BASE_Q, encoder->audio->time_base);
    if (encoder->audio_pts == AV_NOPTS_VALUE) encoder->audio_pts = 0;
    int64_t delta = position - encoder->audio_pts - av_audio_fifo_size(encoder->fifo);
    if (delta > 480000) return ms_fail(encoder, "audio capture stalled", AVERROR(ETIMEDOUT));
    // 1. 【媒体库】【时间对齐】小于两毫秒的量化误差保留连续样本，较大间断补静音，不压缩时间轴
    while (delta > 96) {
        int count = FFMIN(delta, encoder->audio->frame_size);
        int result = av_frame_make_writable(encoder->sound);
        if (result < 0) return ms_fail(encoder, "audio gap", result);
        av_samples_set_silence(encoder->sound->data, 0, count, 2, encoder->audio->sample_fmt);
        if ((result = queue(encoder, count)) < 0) return result;
        delta -= count;
    }
    // 2. 【媒体库】【重叠裁除】设备回退或边界重叠只丢弃重复区间，后续时间戳继续校正累计漂移
    if (delta < -96) {
        int skip = FFMIN(-delta, frames);
        samples += skip * 2;
        frames -= skip;
    }
    while (frames > 0) {
        int count = FFMIN(frames, encoder->audio->frame_size);
        int result = av_frame_make_writable(encoder->sound);
        if (result < 0) return ms_fail(encoder, "audio writable", result);
        const uint8_t *input[1] = {(const uint8_t *)samples};
        result = swr_convert(encoder->resample, encoder->sound->data, count, input, count);
        if (result < 0) return ms_fail(encoder, "PCM conversion", result);
        if ((result = queue(encoder, result)) < 0) return result;
        samples += count * 2; frames -= count;
    }
    return 0;
}

/** 【媒体库】【音频尾帧】把 FIFO 最后不足整帧的部分补静音并编码，返回错误 */
int ms_audio_flush(ms_encoder *encoder) {
    int count = av_audio_fifo_size(encoder->fifo);
    if (!count) return 0;
    int result = av_frame_make_writable(encoder->sound);
    if (result < 0) return ms_fail(encoder, "audio tail", result);
    int remaining = encoder->audio->frame_size - count;
    av_samples_set_silence(encoder->sound->data, 0, remaining, 2, encoder->audio->sample_fmt);
    return queue(encoder, remaining);
}
