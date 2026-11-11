// MIT: Media capability queries for FFmpeg 9
#pragma once
extern "C" {
#include <libavcodec/avcodec.h>
}

/** 【录制组件】【像素格式】按 codec 查询可用像素格式，返回媒体库持有的终止数组 */
inline const AVPixelFormat *mark_shot_pixel_formats(const AVCodec *codec) {
    const void *formats = nullptr;
    avcodec_get_supported_config(nullptr, codec, AV_CODEC_CONFIG_PIX_FORMAT, 0, &formats, nullptr);
    return static_cast<const AVPixelFormat *>(formats);
}

/** 【录制组件】【声道布局】按 codec 查询声道布局，返回媒体库持有的终止数组 */
inline const AVChannelLayout *mark_shot_channel_layouts(const AVCodec *codec) {
    const void *layouts = nullptr;
    avcodec_get_supported_config(nullptr, codec, AV_CODEC_CONFIG_CHANNEL_LAYOUT, 0, &layouts, nullptr);
    return static_cast<const AVChannelLayout *>(layouts);
}

/** 【录制组件】【采样格式】按 codec 查询音频采样格式，返回媒体库持有的终止数组 */
inline const AVSampleFormat *mark_shot_sample_formats(const AVCodec *codec) {
    const void *formats = nullptr;
    avcodec_get_supported_config(nullptr, codec, AV_CODEC_CONFIG_SAMPLE_FORMAT, 0, &formats, nullptr);
    return static_cast<const AVSampleFormat *>(formats);
}
