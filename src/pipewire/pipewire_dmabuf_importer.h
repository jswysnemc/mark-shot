#pragma once

#include <memory>

#include <QByteArray>
#include <QImage>
#include <QRect>
#include <QString>

#ifdef HAVE_PIPEWIRE
#include <pipewire/pipewire.h>
#include <spa/param/video/raw.h>

namespace markshot {

class PipeWireDmaBufImporter final {
public:
    /**
     * 创建 PipeWire DMA-BUF 导入器。
     */
    PipeWireDmaBufImporter();

    /**
     * 释放 PipeWire DMA-BUF 导入器。
     */
    ~PipeWireDmaBufImporter();

    PipeWireDmaBufImporter(const PipeWireDmaBufImporter &) = delete;
    PipeWireDmaBufImporter &operator=(const PipeWireDmaBufImporter &) = delete;

    /**
     * 将 PipeWire DMA-BUF buffer 导入为 CPU 可读图像。
     * @param buffer PipeWire 提供的 SPA buffer。
     * @param videoInfo 已协商的视频格式信息。
     * @param error 输出错误信息。
     * @return 导入成功时返回图像，失败时返回空图像。
     */
    QImage importBuffer(const spa_buffer *buffer,
                        const spa_video_info_raw &videoInfo,
                        QString *error);

    /**
     * 将 PipeWire DMA-BUF buffer 裁剪直读为 BGRA 字节，跳过 QImage 转换。
     * @param buffer PipeWire 提供的 SPA buffer。
     * @param videoInfo 已协商的视频格式信息。
     * @param crop 帧内裁剪区域。
     * @param bgra 输出缓冲，成功时按 crop 尺寸填充自底向上的 BGRA 数据。
     * @param error 输出错误信息。
     * @return 读取成功时返回 true。
     */
    bool importBufferToBgra(const spa_buffer *buffer,
                            const spa_video_info_raw &videoInfo,
                            const QRect &crop,
                            QByteArray *bgra,
                            QString *error);

private:
    class Impl;
    std::unique_ptr<Impl> m_impl;
};

}  // namespace markshot

#endif
