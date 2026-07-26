#pragma once

#include <QSize>
#include <QString>
#include <QStringList>

#ifdef HAVE_LIBAV_RECORDING

extern "C" {
#include <libavutil/buffer.h>
#include <libavutil/frame.h>
#include <libavutil/hwcontext.h>
#include <libavutil/pixfmt.h>
}

struct AVCodecContext;

namespace markshot::recording {

/**
 * 【录制】【硬件编码】管理需要显存帧的硬件编码上下文。
 *
 * VAAPI 与 QSV 编码器不接受系统内存帧，必须先创建硬件设备与帧池，
 * 再把转换好的 NV12 帧上传到显存后送入编码器。
 */
class LibavHwEncoderContext final {
public:
    LibavHwEncoderContext() = default;
    ~LibavHwEncoderContext();
    LibavHwEncoderContext(const LibavHwEncoderContext &) = delete;
    LibavHwEncoderContext &operator=(const LibavHwEncoderContext &) = delete;

    /**
     * 判断编码器是否要求显存帧输入。
     * @param encoderId FFmpeg 编码器名称。
     * @return 需要显存帧时返回 true。
     */
    static bool requiresHardwareFrames(const QString &encoderId);

    /**
     * 读取编码器对应的硬件设备类型。
     * @param encoderId FFmpeg 编码器名称。
     * @return 硬件设备类型，无对应类型时返回 AV_HWDEVICE_TYPE_NONE。
     */
    static AVHWDeviceType deviceTypeForEncoder(const QString &encoderId);

    /**
     * 列出该编码器可尝试的硬件设备节点。
     * @param encoderId FFmpeg 编码器名称。
     * @return 设备节点列表，元素为空串表示交由 FFmpeg 自选设备。
     */
    static QStringList deviceNodeCandidates(const QString &encoderId);

    /**
     * 创建硬件设备与帧池。
     * @param encoderId FFmpeg 编码器名称。
     * @param size 编码帧尺寸。
     * @param softwareFormat 上传前的系统内存像素格式。
     * @param deviceNode 指定的设备节点，传空时由 FFmpeg 自选。
     * @param error 输出错误信息。
     * @return 创建成功时返回 true。
     */
    bool create(const QString &encoderId,
                QSize size,
                AVPixelFormat softwareFormat,
                const QString &deviceNode,
                QString *error);

    /**
     * 把帧池绑定到编码器上下文，必须在 avcodec_open2 之前调用。
     * @param codecContext 编码器上下文。
     * @param error 输出错误信息。
     * @return 绑定成功时返回 true。
     */
    bool attach(AVCodecContext *codecContext, QString *error);

    /**
     * 把系统内存帧上传到显存帧。
     * @param softwareFrame 已完成像素转换的系统内存帧。
     * @param error 输出错误信息。
     * @return 上传成功时返回显存帧，失败时返回空。
     */
    AVFrame *upload(const AVFrame *softwareFrame, QString *error);

    /**
     * 读取上一次上传得到的显存帧，用于补帧复用。
     * @return 显存帧，尚未上传时返回空。
     */
    AVFrame *lastUploadedFrame() const
    {
        return m_uploaded ? m_hardwareFrame : nullptr;
    }

    /**
     * 判断硬件上下文是否可用。
     * @return 已创建帧池时返回 true。
     */
    bool isActive() const
    {
        return m_framesContext != nullptr;
    }

    /**
     * 读取显存帧像素格式。
     * @return 显存帧像素格式。
     */
    AVPixelFormat hardwareFormat() const
    {
        return m_hardwareFormat;
    }

    /**
     * 释放硬件设备与帧池。
     * @return 无返回值。
     */
    void reset();

private:
    AVBufferRef *m_deviceContext = nullptr;
    AVBufferRef *m_framesContext = nullptr;
    AVFrame *m_hardwareFrame = nullptr;
    AVPixelFormat m_hardwareFormat = AV_PIX_FMT_NONE;
    bool m_uploaded = false;
};

}  // namespace markshot::recording

#endif  // HAVE_LIBAV_RECORDING
