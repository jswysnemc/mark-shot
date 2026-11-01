#pragma once

#ifdef HAVE_LIBAV_RECORDING

#include <QtGlobal>

extern "C" {
#include <libavutil/frame.h>
}

namespace markshot::recording {

/** 【录制】【GIF复用】仅持有上一采集帧的量化结果，不保留原始像素或整段录制。 */
class LibavGifFrameCache final {
public:
    LibavGifFrameCache() = default;
    ~LibavGifFrameCache();
    LibavGifFrameCache(const LibavGifFrameCache &) = delete;
    LibavGifFrameCache &operator=(const LibavGifFrameCache &) = delete;

    /**
     * 【录制】【GIF复用】共享量化帧引用，分配失败或序号无效时禁用复用。
     * @param frame 编码前的 PAL8 量化帧，像素必须由 AVBufferRef 管理。
     * @param sequence 正的采集序号；非正值表示没有稳定帧标识。
     * @return 无返回值，缓存失败不影响常规编码。
     */
    void remember(const AVFrame *frame, qint64 sequence);

    /**
     * 【录制】【GIF复用】读取同一采集帧的缓存，调用方只可更新时间属性。
     * @param sequence 当前样本的采集序号。
     * @return 匹配时返回借用帧，否则返回空；有效期至下次 remember 或 clear。
     */
    AVFrame *find(qint64 sequence) const;

    /** 【录制】【GIF复用】释放引用并清空序号，无参数和返回值。 */
    void clear();

private:
    AVFrame *m_frame = nullptr;
    qint64 m_sequence = 0;
};

}  // namespace markshot::recording

#endif
