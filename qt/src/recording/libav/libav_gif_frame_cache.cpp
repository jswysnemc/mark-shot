#include "recording/libav/libav_gif_frame_cache.h"

#ifdef HAVE_LIBAV_RECORDING

namespace markshot::recording {

LibavGifFrameCache::~LibavGifFrameCache()
{
    clear();
}

void LibavGifFrameCache::remember(const AVFrame *frame, qint64 sequence)
{
    clear();
    if (!frame || frame->format != AV_PIX_FMT_PAL8 || !frame->buf[0] || sequence <= 0) {
        return;
    }
    // 1. 【录制】【GIF复用】仅共享 FFmpeg 缓冲；克隆失败时保留常规量化路径
    m_frame = av_frame_clone(frame);
    if (m_frame) {
        m_sequence = sequence;
    }
}

AVFrame *LibavGifFrameCache::find(qint64 sequence) const
{
    return sequence > 0 && sequence == m_sequence ? m_frame : nullptr;
}

void LibavGifFrameCache::clear()
{
    av_frame_free(&m_frame);
    m_sequence = 0;
}

}  // namespace markshot::recording

#endif
