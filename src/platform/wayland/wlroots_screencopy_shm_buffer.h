#pragma once

#include "recording/recording_frame_payload.h"

#include <QImage>
#include <QString>

#include <cstdint>

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
     * 释放当前缓冲资源。
     * @return 无返回值。
     */
    void reset();

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
