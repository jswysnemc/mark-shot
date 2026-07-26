#pragma once

#include "recording/recording_quality_options.h"
#include "recording/recording_video_encoder_options.h"

#include <QSize>
#include <QString>

#ifdef HAVE_LIBAV_RECORDING

extern "C" {
#include <libavcodec/avcodec.h>
}

namespace markshot::recording {

class LibavHwEncoderContext;

struct LibavVideoEncoderRequest {
    RecordingVideoEncoderOptions encoder;
    RecordingQualityProfile quality;
    QSize encodedSize;
    int fps = 30;
    bool globalHeader = false;
};

struct LibavVideoEncoderResult {
    AVCodecContext *codecContext = nullptr;
    const AVCodec *codec = nullptr;
    AVPixelFormat scalerFormat = AV_PIX_FMT_YUV420P;
    bool hardwareFrames = false;
};

/**
 * 【录制】【库内编码】为编码器选择系统内存输入像素格式。
 * @param codec 目标编码器。
 * @return 优先 yuv420p、其次 nv12，再退到首个非硬件格式。
 */
AVPixelFormat chooseEncoderPixelFormat(const AVCodec *codec);

/**
 * 【录制】【库内编码】估算需要显式码率的编码器目标码率。
 * @param size 编码尺寸。
 * @param fps 目标帧率。
 * @param factor 质量档位对应的码率倍率。
 * @return 码率（bit/s）。
 */
qint64 estimatedVideoBitRate(QSize size, int fps, double factor = 1.0);

/**
 * 【录制】【库内编码】创建并打开视频编码器。
 * @param request 编码请求参数。
 * @param hardware 硬件帧上下文，编码器需要显存帧时由本函数初始化。
 * @param result 输出编码器上下文与像素格式。
 * @param error 输出错误信息。
 * @return 打开成功时返回 true，失败时不会残留已分配的上下文。
 */
bool openVideoEncoder(const LibavVideoEncoderRequest &request,
                      LibavHwEncoderContext *hardware,
                      LibavVideoEncoderResult *result,
                      QString *error);

}  // namespace markshot::recording

#endif  // HAVE_LIBAV_RECORDING
