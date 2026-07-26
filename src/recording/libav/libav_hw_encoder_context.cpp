#include "recording/libav/libav_hw_encoder_context.h"

#ifdef HAVE_LIBAV_RECORDING

#include "recording/libav_error.h"
#include "recording/recording_encoder_probe.h"

#include <QByteArray>
#include <QtGlobal>

extern "C" {
#include <libavcodec/avcodec.h>
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
 * 【录制】【硬件编码】读取硬件设备类型对应的显存帧格式。
 * @param type 硬件设备类型。
 * @return 显存帧像素格式。
 */
AVPixelFormat hardwarePixelFormat(AVHWDeviceType type)
{
    switch (type) {
    case AV_HWDEVICE_TYPE_VAAPI:
        return AV_PIX_FMT_VAAPI;
    case AV_HWDEVICE_TYPE_QSV:
        return AV_PIX_FMT_QSV;
    default:
        break;
    }
    return AV_PIX_FMT_NONE;
}

}  // namespace

LibavHwEncoderContext::~LibavHwEncoderContext()
{
    reset();
}

bool LibavHwEncoderContext::requiresHardwareFrames(const QString &encoderId)
{
    return deviceTypeForEncoder(encoderId) != AV_HWDEVICE_TYPE_NONE;
}

AVHWDeviceType LibavHwEncoderContext::deviceTypeForEncoder(const QString &encoderId)
{
    const QString id = encoderId.trimmed().toLower();
    if (id.endsWith(QStringLiteral("_vaapi"))) {
        return AV_HWDEVICE_TYPE_VAAPI;
    }
    if (id.endsWith(QStringLiteral("_qsv"))) {
        return AV_HWDEVICE_TYPE_QSV;
    }
    // nvenc、amf、mediafoundation 均可直接接收系统内存帧
    return AV_HWDEVICE_TYPE_NONE;
}

QStringList LibavHwEncoderContext::deviceNodeCandidates(const QString &encoderId)
{
    if (deviceTypeForEncoder(encoderId) != AV_HWDEVICE_TYPE_VAAPI) {
        // QSV 自行选择子设备，交给 FFmpeg 默认逻辑
        return {QString()};
    }

    // 1. 显式指定的节点优先，便于排查多显卡机器的设备选择
    const QString configured = QString::fromLocal8Bit(
        qgetenv("MARK_SHOT_RECORDING_VAAPI_DEVICE").trimmed());
    if (!configured.isEmpty()) {
        return {configured};
    }

    // 2. 混合显卡机器上第一个渲染节点未必支持编码，逐个尝试
    const QStringList nodes = recordingRenderNodePaths();
    return nodes.isEmpty() ? QStringList{QString()} : nodes;
}

bool LibavHwEncoderContext::create(const QString &encoderId,
                                   QSize size,
                                   AVPixelFormat softwareFormat,
                                   const QString &deviceNode,
                                   QString *error)
{
    if (error) {
        error->clear();
    }
    reset();

    const AVHWDeviceType type = deviceTypeForEncoder(encoderId);
    m_hardwareFormat = hardwarePixelFormat(type);
    if (type == AV_HWDEVICE_TYPE_NONE || m_hardwareFormat == AV_PIX_FMT_NONE) {
        return failWith(error,
                        QStringLiteral("encoder %1 does not use a hardware frame pool").arg(encoderId));
    }
    if (size.isEmpty()) {
        return failWith(error, QStringLiteral("Cannot create a hardware frame pool with an empty size"));
    }

    // 1. 创建硬件设备上下文，节点为空时交由 FFmpeg 自选
    const QByteArray deviceNodeBytes = deviceNode.trimmed().toLocal8Bit();
    int result = av_hwdevice_ctx_create(&m_deviceContext,
                                        type,
                                        deviceNodeBytes.isEmpty() ? nullptr : deviceNodeBytes.constData(),
                                        nullptr,
                                        0);
    if (result < 0 || !m_deviceContext) {
        reset();
        return failWith(error,
                        QStringLiteral("Failed to create %1 hardware device %2: %3")
                            .arg(QString::fromUtf8(av_hwdevice_get_type_name(type)),
                                 deviceNode.trimmed().isEmpty() ? QStringLiteral("(default)") : deviceNode,
                                 libavErrorText(result)));
    }

    // 2. 按编码尺寸创建显存帧池
    m_framesContext = av_hwframe_ctx_alloc(m_deviceContext);
    if (!m_framesContext) {
        reset();
        return failWith(error, QStringLiteral("Failed to allocate hardware frame context"));
    }
    auto *frames = reinterpret_cast<AVHWFramesContext *>(m_framesContext->data);
    frames->format = m_hardwareFormat;
    frames->sw_format = softwareFormat;
    frames->width = size.width();
    frames->height = size.height();
    frames->initial_pool_size = 24;
    result = av_hwframe_ctx_init(m_framesContext);
    if (result < 0) {
        reset();
        return failWith(error,
                        QStringLiteral("Failed to initialize hardware frame pool: %1")
                            .arg(libavErrorText(result)));
    }

    // 3. 预分配上传用的帧对象
    m_hardwareFrame = av_frame_alloc();
    if (!m_hardwareFrame) {
        reset();
        return failWith(error, QStringLiteral("Failed to allocate hardware frame"));
    }
    return true;
}

bool LibavHwEncoderContext::attach(AVCodecContext *codecContext, QString *error)
{
    if (error) {
        error->clear();
    }
    if (!codecContext || !m_framesContext) {
        return failWith(error, QStringLiteral("hardware frame pool is not ready"));
    }

    const auto *frames = reinterpret_cast<const AVHWFramesContext *>(m_framesContext->data);
    codecContext->pix_fmt = m_hardwareFormat;
    codecContext->sw_pix_fmt = frames->sw_format;
    codecContext->hw_frames_ctx = av_buffer_ref(m_framesContext);
    if (!codecContext->hw_frames_ctx) {
        return failWith(error, QStringLiteral("Failed to reference hardware frame pool"));
    }
    return true;
}

AVFrame *LibavHwEncoderContext::upload(const AVFrame *softwareFrame, QString *error)
{
    if (error) {
        error->clear();
    }
    if (!softwareFrame || !m_framesContext || !m_hardwareFrame) {
        failWith(error, QStringLiteral("hardware frame pool is not ready"));
        return nullptr;
    }

    // 1. 归还上一帧引用，编码器仍持有的帧会由引用计数保护
    av_frame_unref(m_hardwareFrame);
    m_uploaded = false;

    // 2. 从帧池取一块显存并上传像素
    int result = av_hwframe_get_buffer(m_framesContext, m_hardwareFrame, 0);
    if (result < 0) {
        failWith(error,
                 QStringLiteral("Failed to acquire a hardware frame: %1").arg(libavErrorText(result)));
        return nullptr;
    }
    result = av_hwframe_transfer_data(m_hardwareFrame, softwareFrame, 0);
    if (result < 0) {
        failWith(error,
                 QStringLiteral("Failed to upload frame to hardware: %1").arg(libavErrorText(result)));
        return nullptr;
    }
    m_hardwareFrame->pts = softwareFrame->pts;
    m_uploaded = true;
    return m_hardwareFrame;
}

void LibavHwEncoderContext::reset()
{
    if (m_hardwareFrame) {
        av_frame_free(&m_hardwareFrame);
    }
    if (m_framesContext) {
        av_buffer_unref(&m_framesContext);
    }
    if (m_deviceContext) {
        av_buffer_unref(&m_deviceContext);
    }
    m_hardwareFormat = AV_PIX_FMT_NONE;
    m_uploaded = false;
}

}  // namespace markshot::recording

#endif  // HAVE_LIBAV_RECORDING
