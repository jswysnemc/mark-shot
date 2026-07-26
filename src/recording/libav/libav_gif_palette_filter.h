#pragma once

#include <QSize>
#include <QString>

#ifdef HAVE_LIBAVFILTER

extern "C" {
#include <libavfilter/avfilter.h>
#include <libavutil/frame.h>
}

namespace markshot::recording {

/**
 * 【录制】【GIF调色板】按帧生成局部调色板并做量化的滤镜链。
 *
 * GIF 每帧可携带独立调色板，逐帧 palettegen 让画面色彩明显优于固定调色板，
 * 且无需缓存整段录制即可单遍完成量化。
 */
class LibavGifPaletteFilter final {
public:
    LibavGifPaletteFilter() = default;
    ~LibavGifPaletteFilter();
    LibavGifPaletteFilter(const LibavGifPaletteFilter &) = delete;
    LibavGifPaletteFilter &operator=(const LibavGifPaletteFilter &) = delete;

    /**
     * 构建滤镜链。
     * @param size 帧尺寸。
     * @param timeBase 输入帧时间基分母，输入 pts 以该时间基计。
     * @param error 输出错误信息。
     * @return 构建成功时返回 true。
     */
    bool open(QSize size, int timeBase, QString *error);

    /**
     * 送入一帧 RGB24。
     * @param input RGB24 输入帧，传空表示送入结束标记。
     * @param error 输出错误信息。
     * @return 送入成功时返回 true。
     */
    bool sendFrame(AVFrame *input, QString *error);

    /**
     * 取出一帧量化后的 PAL8 结果。
     * @param output 输出量化帧，暂无可用输出时置空。
     * @param error 输出错误信息。
     * @return 取出过程没有出错时返回 true。
     */
    bool receiveFrame(AVFrame **output, QString *error);

    /**
     * 判断滤镜链是否已构建。
     * @return 已构建时返回 true。
     */
    bool isOpen() const
    {
        return m_graph != nullptr;
    }

    /**
     * 释放滤镜链资源。
     * @return 无返回值。
     */
    void close();

private:
    AVFilterGraph *m_graph = nullptr;
    AVFilterContext *m_source = nullptr;
    AVFilterContext *m_sink = nullptr;
    AVFrame *m_output = nullptr;
};

}  // namespace markshot::recording

#endif  // HAVE_LIBAVFILTER
