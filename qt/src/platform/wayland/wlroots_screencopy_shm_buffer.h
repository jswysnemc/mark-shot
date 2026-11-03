#pragma once

#include "recording/recording_frame_payload.h"

#include <QImage>
#include <QString>

#include <cstdint>
#include <memory>
#include <thread>

struct wl_buffer;
struct wl_shm;

namespace markshot::recording {

class WlrootsScreencopyShmBuffer final {
public:
    /**
     * 【录制】【映射寿命】销毁共享内存缓冲，只解除像素映射，永不发送 Wayland 请求。
     * 最后一个帧引用可能在任意线程、Wayland 连接关闭之后释放；协议对象必须由创建线程
     * 提前调用 releaseWaylandBuffer() 或 reset() 释放，残留句柄只会被放弃并记录错误日志。
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
     * 【录制】【映射寿命】释放协议缓冲并解除像素映射，用于重新布局或显式回收。
     * 必须在创建缓冲的采集线程且 Wayland 连接仍有效时调用。
     * @return 无返回值。
     */
    void reset();

    /**
     * 【录制】【映射寿命】在 Wayland 连接关闭前解除协议对象，保留仍被帧引用的像素映射。
     * 只有创建缓冲的采集线程会真正发送 wl_buffer.destroy；其他线程调用时放弃句柄并记录错误日志。
     * 调用后 buffer() 恒为空，后续只允许读取已发布的像素；无参数和返回值。
     */
    void releaseWaylandBuffer();

    /** 返回当前像素映射字节数，不包含协议对象或内核开销，无参数。 */
    qsizetype byteSize() const { return m_size; }

    /** 检查 format、width、height、stride 是否与当前有效缓冲一致，返回是否可直接复用。 */
    bool matches(std::uint32_t format, int width, int height, int stride) const;

private:
    /**
     * 【录制】【映射寿命】放弃无法安全销毁的 wl_buffer 句柄，不访问所属连接。
     * @param reason 放弃原因，写入错误日志。
     * @return 无返回值。
     */
    void abandonWaylandBuffer(const char *reason);

    /**
     * 【录制】【映射寿命】解除像素映射并清空布局参数，不涉及任何 Wayland 对象，可在任意线程调用。
     * @return 无返回值。
     */
    void unmapPixels();

    wl_buffer *m_buffer = nullptr;
    // 【录制】【映射寿命】wl_buffer 所属事件队列的分发线程，只有该线程可以销毁协议对象
    std::thread::id m_ownerThread;
    void *m_data = nullptr;
    qsizetype m_size = 0;
    std::uint32_t m_format = 0;
    int m_width = 0;
    int m_height = 0;
    int m_stride = 0;
};

}  // namespace markshot::recording
