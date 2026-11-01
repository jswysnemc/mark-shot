#pragma once

#include "recording/recording_frame_payload.h"

#include <QImage>
#include <QString>

#include <cstdint>
#include <memory>

struct wl_buffer;
struct wl_shm;

namespace markshot::recording {

class WlrootsScreencopyShmBuffer final {
public:
    /**
     * 销毁共享内存缓冲。
     */
    ~WlrootsScreencopyShmBuffer();

    /**
     * 确保缓冲参数满足当前帧要求。
     * @param shm Wayland 共享内存接口。
     * @param format wl_shm 像素格式。
     * @param width 帧宽度。
     * @param height 帧高度。
     * @param stride 行跨度。
     * @param error 输出错误信息。
     * @return 缓冲可用时返回 true。
     */
    bool ensure(wl_shm *shm,
                std::uint32_t format,
                int width,
                int height,
                int stride,
                QString *error);

    /**
     * 返回 Wayland buffer。
     * @return Wayland buffer 指针。
     */
    wl_buffer *buffer() const;

    /**
     * 把缓冲内容复制为 QImage。
     * @param yInvert 内容是否上下翻转。
     * @param error 输出错误信息。
     * @return 复制后的图像。
     */
    QImage toImage(bool yInvert, QString *error) const;

    /**
     * 把缓冲内容复制为连续 raw BGRA 帧。
     * @param yInvert 内容是否上下翻转。
     * @param error 输出错误信息。
     * @return 连续 raw BGRA 帧。
     */
    RecordingRawBgraFrame copyBgraFrame(bool yInvert, QString *error) const;

    /**
     * 把缓冲内容包装为共享内存 raw BGRA 视图。
     * @param owner 维持缓冲生命周期的共享所有者。
     * @param yInvert 内容是否上下翻转。
     * @param error 输出错误信息。
     * @return raw BGRA 视图。
     */
    RecordingRawBgraFrame mappedBgraFrame(std::shared_ptr<const void> owner,
                                          bool yInvert,
                                          QString *error) const;

    /**
     * 释放当前缓冲资源。
     * @return 无返回值。
     */
    void reset();

    /**
     * 【录制】【映射寿命】在 Wayland 连接关闭前解除协议对象，保留仍被帧引用的像素映射。
     * 必须在采集线程调用，后续只允许读取已发布的像素；无参数和返回值。
     */
    void releaseWaylandBuffer();

    /** 返回当前像素映射字节数，不包含协议对象或内核开销，无参数。 */
    qsizetype byteSize() const { return m_size; }

    /** 检查 format、width、height、stride 是否与当前有效缓冲一致，返回是否可直接复用。 */
    bool matches(std::uint32_t format, int width, int height, int stride) const;

private:
    wl_buffer *m_buffer = nullptr;
    void *m_data = nullptr;
    qsizetype m_size = 0;
    std::uint32_t m_format = 0;
    int m_width = 0;
    int m_height = 0;
    int m_stride = 0;
};

}  // namespace markshot::recording
