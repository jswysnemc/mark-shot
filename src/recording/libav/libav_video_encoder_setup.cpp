#include "recording/libav/libav_video_encoder_setup.h"

#ifdef HAVE_LIBAV_RECORDING

#include "debug_log.h"
#include "recording/libav/libav_hw_encoder_context.h"
#include "recording/libav_error.h"

#include <QByteArray>
#include <QStringList>

#include <algorithm>

extern "C" {
#include <libavutil/opt.h>
#include <libavutil/pixdesc.h>
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
 * 【录制】【库内编码】读取编码器支持的像素格式列表。
 * @param codec 目标编码器。
 * @return 像素格式数组，编码器未声明时返回空指针。
 */
const enum AVPixelFormat *encoderPixelFormats(const AVCodec *codec)
{
#if LIBAVCODEC_VERSION_INT >= AV_VERSION_INT(61, 13, 100)
    const enum AVPixelFormat *formats = nullptr;
    int count = 0;
    if (avcodec_get_supported_config(nullptr,
                                     codec,
                                     AV_CODEC_CONFIG_PIX_FORMAT,
                                     0,
                                     reinterpret_cast<const void **>(&formats),
                                     &count) < 0) {
        return nullptr;
    }
    return formats;
#else
    return codec->pix_fmts;
#endif
}

/**
 * 【录制】【库内编码】按编码器族写入编码参数。
 * @param codecContext 编码器上下文。
 * @param encoderId 编码器名称。
 * @param quality 质量档位参数。
 * @param options 输出编码器私有参数字典。
 * @return 无返回值。
 */
void applyEncoderTuning(AVCodecContext *codecContext,
                        const QString &encoderId,
                        const RecordingQualityProfile &quality,
                        AVDictionary **options)
{
    const QByteArray constantQuality = QByteArray::number(quality.constantQuality);
    const QByteArray softwarePreset = quality.softwarePreset.toUtf8();
    const QByteArray hardwarePreset = quality.hardwarePreset.toUtf8();

    if (encoderId == QStringLiteral("libx264")) {
        av_dict_set(options, "preset", softwarePreset.constData(), 0);
        av_dict_set(options, "crf", constantQuality.constData(), 0);
        return;
    }
    if (encoderId.endsWith(QStringLiteral("_nvenc"))) {
        // p1 最快、p7 画质最好，按质量档位在两端之间取值
        av_dict_set(options, "preset", hardwarePreset.constData(), 0);
        av_dict_set(options, "tune", "ull", 0);
        av_dict_set(options, "rc", "vbr", 0);
        av_dict_set(options, "cq", constantQuality.constData(), 0);
        return;
    }
    if (encoderId.endsWith(QStringLiteral("_vaapi"))) {
        // VAAPI 用固定量化参数，避免码率控制在静止画面上浪费带宽
        av_dict_set(options, "rc_mode", "CQP", 0);
        av_dict_set(options, "qp", constantQuality.constData(), 0);
        codecContext->bit_rate = 0;
        return;
    }
    if (encoderId.endsWith(QStringLiteral("_qsv"))) {
        av_dict_set(options, "preset", softwarePreset.constData(), 0);
        av_dict_set(options, "global_quality", constantQuality.constData(), 0);
        return;
    }
    if (encoderId.endsWith(QStringLiteral("_amf"))) {
        av_dict_set(options, "usage", "ultralowlatency", 0);
        av_dict_set(options, "quality", "balanced", 0);
        return;
    }
}

/**
 * 【录制】【库内编码】判断编码器是否需要显式码率。
 * @param encoderId 编码器名称。
 * @param hardware 是否为硬件编码。
 * @return 需要显式码率时返回 true。
 */
bool needsExplicitBitRate(const QString &encoderId, bool hardware)
{
    if (encoderId.endsWith(QStringLiteral("_vaapi"))) {
        return false;
    }
    return hardware || encoderId == QStringLiteral("mpeg4");
}

}  // namespace

AVPixelFormat chooseEncoderPixelFormat(const AVCodec *codec)
{
    const enum AVPixelFormat *formats = encoderPixelFormats(codec);
    if (!formats) {
        return AV_PIX_FMT_YUV420P;
    }

    AVPixelFormat firstSoftware = AV_PIX_FMT_NONE;
    bool hasYuv420p = false;
    bool hasNv12 = false;
    for (const enum AVPixelFormat *format = formats; *format != AV_PIX_FMT_NONE; ++format) {
        const AVPixFmtDescriptor *descriptor = av_pix_fmt_desc_get(*format);
        if (descriptor && (descriptor->flags & AV_PIX_FMT_FLAG_HWACCEL)) {
            continue;
        }
        if (*format == AV_PIX_FMT_YUV420P) {
            hasYuv420p = true;
        }
        if (*format == AV_PIX_FMT_NV12) {
            hasNv12 = true;
        }
        if (firstSoftware == AV_PIX_FMT_NONE) {
            firstSoftware = *format;
        }
    }
    if (hasYuv420p) {
        return AV_PIX_FMT_YUV420P;
    }
    if (hasNv12) {
        return AV_PIX_FMT_NV12;
    }
    return firstSoftware != AV_PIX_FMT_NONE ? firstSoftware : AV_PIX_FMT_YUV420P;
}

qint64 estimatedVideoBitRate(QSize size, int fps, double factor)
{
    const qint64 pixelRate = static_cast<qint64>(size.width()) * size.height() * std::max(1, fps);
    const double scaled = static_cast<double>(pixelRate / 10) * std::max(0.1, factor);
    return std::max<qint64>(1000000, static_cast<qint64>(scaled));
}

namespace {

/**
 * 【录制】【库内编码】在指定硬件设备节点上尝试打开一次编码器。
 * @param request 编码请求参数。
 * @param codec 已查找到的编码器。
 * @param hardwareFrames 编码器是否需要显存帧。
 * @param hardware 硬件帧上下文。
 * @param deviceNode 本次尝试使用的硬件设备节点。
 * @param result 输出编码器上下文与像素格式。
 * @param error 输出错误信息。
 * @return 打开成功时返回 true。
 */
bool tryOpenVideoEncoder(const LibavVideoEncoderRequest &request,
                         const AVCodec *codec,
                         bool hardwareFrames,
                         LibavHwEncoderContext *hardware,
                         const QString &deviceNode,
                         LibavVideoEncoderResult *result,
                         QString *error)
{
    AVCodecContext *codecContext = avcodec_alloc_context3(codec);
    if (!codecContext) {
        return failWith(error, QStringLiteral("Failed to allocate libav codec context"));
    }

    const int fps = std::max(1, request.fps);
    codecContext->width = request.encodedSize.width();
    codecContext->height = request.encodedSize.height();
    codecContext->time_base = AVRational{1, fps};
    codecContext->framerate = AVRational{fps, 1};
    codecContext->gop_size = fps;
    codecContext->max_b_frames = 0;
    // 0 表示自动按 CPU 核数分配编码线程，默认值 1 会让 libx264 单线程运行
    codecContext->thread_count = 0;
    if (request.globalHeader) {
        codecContext->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }

    // 1. 需要显存帧的编码器先建立帧池再绑定，系统内存编码器直接选像素格式
    AVPixelFormat scalerFormat = AV_PIX_FMT_YUV420P;
    if (hardwareFrames) {
        if (!hardware) {
            avcodec_free_context(&codecContext);
            return failWith(error,
                            QStringLiteral("encoder %1 requires a hardware frame pool")
                                .arg(request.encoder.id));
        }
        scalerFormat = AV_PIX_FMT_NV12;
        if (!hardware->create(request.encoder.id, request.encodedSize, scalerFormat, deviceNode, error)
            || !hardware->attach(codecContext, error)) {
            avcodec_free_context(&codecContext);
            hardware->reset();
            return false;
        }
    } else {
        scalerFormat = chooseEncoderPixelFormat(codec);
        codecContext->pix_fmt = scalerFormat;
    }

    if (needsExplicitBitRate(request.encoder.id, request.encoder.hardware)) {
        codecContext->bit_rate = estimatedVideoBitRate(request.encodedSize,
                                                       fps,
                                                       request.quality.bitRateFactor);
    }

    // 2. 写入编码器私有参数并打开
    AVDictionary *codecOptions = nullptr;
    applyEncoderTuning(codecContext, request.encoder.id, request.quality, &codecOptions);
    const int openResult = avcodec_open2(codecContext, codec, &codecOptions);
    av_dict_free(&codecOptions);
    if (openResult < 0) {
        avcodec_free_context(&codecContext);
        if (hardware) {
            hardware->reset();
        }
        return failWith(error,
                        QStringLiteral("Failed to open libav video encoder %1%2: %3")
                            .arg(request.encoder.id,
                                 deviceNode.trimmed().isEmpty()
                                     ? QString()
                                     : QStringLiteral(" on %1").arg(deviceNode),
                                 libavErrorText(openResult)));
    }

    result->codecContext = codecContext;
    result->codec = codec;
    result->scalerFormat = scalerFormat;
    result->hardwareFrames = hardwareFrames;
    return true;
}

}  // namespace

bool openVideoEncoder(const LibavVideoEncoderRequest &request,
                      LibavHwEncoderContext *hardware,
                      LibavVideoEncoderResult *result,
                      QString *error)
{
    if (error) {
        error->clear();
    }
    if (!result) {
        return failWith(error, QStringLiteral("libav encoder result sink is missing"));
    }
    *result = {};

    // 1. 查找编码器实现
    const QByteArray encoderName = request.encoder.id.toUtf8();
    const AVCodec *codec = avcodec_find_encoder_by_name(encoderName.constData());
    if (!codec) {
        return failWith(error,
                        QStringLiteral("encoder %1 is not available in FFmpeg libraries")
                            .arg(request.encoder.id));
    }

    // 2. 混合显卡机器上首个渲染节点未必支持编码，逐个设备节点尝试打开
    const bool hardwareFrames = LibavHwEncoderContext::requiresHardwareFrames(request.encoder.id);
    const QStringList deviceNodes = hardwareFrames
        ? LibavHwEncoderContext::deviceNodeCandidates(request.encoder.id)
        : QStringList{QString()};

    QString lastError;
    for (const QString &deviceNode : deviceNodes) {
        QString attemptError;
        if (tryOpenVideoEncoder(request, codec, hardwareFrames, hardware, deviceNode, result, &attemptError)) {
            if (hardwareFrames) {
                markshot::debugLog("recording",
                                   "【录制】【硬件编码】encoder=%s device=%s",
                                   request.encoder.id.toUtf8().constData(),
                                   deviceNode.trimmed().isEmpty()
                                       ? "default"
                                       : deviceNode.toUtf8().constData());
            }
            return true;
        }
        markshot::debugLog("recording",
                           "【录制】【硬件编码尝试失败】encoder=%s device=%s error=%s",
                           request.encoder.id.toUtf8().constData(),
                           deviceNode.trimmed().isEmpty() ? "default" : deviceNode.toUtf8().constData(),
                           attemptError.toUtf8().constData());
        lastError = attemptError;
    }

    return failWith(error,
                    lastError.isEmpty()
                        ? QStringLiteral("Failed to open libav video encoder %1").arg(request.encoder.id)
                        : lastError);
}

}  // namespace markshot::recording

#endif  // HAVE_LIBAV_RECORDING
