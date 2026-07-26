#pragma once

#include <QSize>
#include <QString>

#ifdef HAVE_LIBAV_RECORDING

#include <memory>
#include <vector>

extern "C" {
#include <libavutil/frame.h>
#include <libavutil/pixfmt.h>
#include <libswscale/swscale.h>
}

namespace markshot::recording {

class LibavScalerThreadPool;

/**
 * 【录制】【像素转换】把采集到的 BGRA 帧转换为编码器输入格式。
 *
 * 转换按行切片后并行执行，每个切片持有独立的 SwsContext，
 * 避免 BGRA 到 YUV 的转换成为写出线程的串行瓶颈。
 */
class LibavVideoScaler final {
public:
    LibavVideoScaler();
    ~LibavVideoScaler();
    LibavVideoScaler(const LibavVideoScaler &) = delete;
    LibavVideoScaler &operator=(const LibavVideoScaler &) = delete;

    /**
     * 准备转换上下文。
     * @param sourceSize 参与转换的源区域尺寸，超出部分按裁剪丢弃。
     * @param targetSize 编码帧尺寸。
     * @param targetFormat 编码帧像素格式。
     * @param error 输出错误信息。
     * @return 准备成功时返回 true。
     */
    bool prepare(QSize sourceSize, QSize targetSize, AVPixelFormat targetFormat, QString *error);

    /**
     * 转换一帧 BGRA 数据到编码帧。
     * @param source BGRA 首字节地址。
     * @param stride 源行跨度字节数。
     * @param sourceDataHeight 源缓冲实际行数，自底向上的帧据此定位首行。
     * @param yInverted 源数据自底向上存储时为 true。
     * @param destination 目标编码帧。
     * @param error 输出错误信息。
     * @return 转换成功时返回 true。
     */
    bool convert(const char *source,
                 int stride,
                 int sourceDataHeight,
                 bool yInverted,
                 AVFrame *destination,
                 QString *error);

    /**
     * 释放全部转换上下文。
     * @return 无返回值。
     */
    void reset();

    /**
     * 读取当前并行切片数量。
     * @return 切片数量。
     */
    int sliceCount() const
    {
        return static_cast<int>(m_slices.size());
    }

private:
    struct SliceContext {
        SwsContext *context = nullptr;
        int startY = 0;
        int height = 0;
    };

    /**
     * 按目标高度切分并创建每个切片的转换上下文。
     * @param error 输出错误信息。
     * @return 创建成功时返回 true。
     */
    bool buildSlices(QString *error);

    /**
     * 执行单个切片的像素转换。
     * @param slice 切片描述。
     * @param source BGRA 首字节地址。
     * @param stride 源行跨度，自底向上时为负值。
     * @param destination 目标编码帧。
     * @return 无返回值。
     */
    void convertSlice(const SliceContext &slice,
                      const char *source,
                      int stride,
                      AVFrame *destination) const;

    std::vector<SliceContext> m_slices;
    std::unique_ptr<LibavScalerThreadPool> m_pool;
    QSize m_sourceSize;
    QSize m_targetSize;
    AVPixelFormat m_targetFormat = AV_PIX_FMT_YUV420P;
};

}  // namespace markshot::recording

#endif  // HAVE_LIBAV_RECORDING
