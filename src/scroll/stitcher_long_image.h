#pragma once

#include "scroll/stitcher_frame_profile.h"

#include <QImage>

namespace markshot::scroll {

// 拼接长图的增量缓冲。内部持有一块预留了增长空间的 ARGB32 图像,追加与
// 前插只拷贝新增行,同步维护与内容对应的行签名,避免每帧对整图的重建与
// 重采样。对外通过 image() 提供紧凑拷贝并按修订号缓存。
class LongImage {
public:
    /// @brief 以种子帧重置缓冲,长图内容变为该帧。
    /// @param seed 种子帧,要求 ARGB32_Premultiplied 格式。
    /// @param seedCols 种子帧的行签名。
    void reset(const QImage &seed, const ColSamples &seedCols);

    /// @brief 清空缓冲,回到未初始化状态。
    void clear();

    /// @brief 判断缓冲是否为空。
    /// @return 未初始化返回 true。
    bool isNull() const;

    /// @brief 长图宽度。
    /// @return 宽度像素数。
    int width() const;

    /// @brief 长图当前高度。
    /// @return 高度像素数。
    int height() const;

    /// @brief 与长图内容同步的行签名。
    /// @return 行签名数组引用。
    const ColSamples &cols() const;

    /// @brief 访问长图某行的像素。
    /// @param y 长图坐标中的行号。
    /// @return 行像素指针;行号越界返回空指针。
    const QRgb *scanLine(int y) const;

    /// @brief 内容修订号,每次修改自增,供外部缓存失效判断。
    /// @return 当前修订号。
    quint64 revision() const;

    /// @brief 长图的紧凑拷贝,按修订号缓存。
    /// @return 高度等于 height() 的图像;未初始化时为空图像。
    QImage image() const;

    /// @brief 裁掉长图尾部若干行后,把帧的底部若干行追加到长图末尾。
    /// @param frame 当前帧。
    /// @param frameCols 当前帧的行签名。
    /// @param rows 追加的行数(取自帧底部)。
    /// @param trimBottom 追加前从长图尾部裁掉的行数。
    void appendBottom(const QImage &frame, const ColSamples &frameCols, int rows, int trimBottom);

    /// @brief 裁掉长图顶部若干行后,把帧的顶部若干行插入长图最前。
    /// @param frame 当前帧。
    /// @param frameCols 当前帧的行签名。
    /// @param rows 插入的行数(取自帧顶部)。
    /// @param trimTop 插入前从长图顶部裁掉的行数。
    void prependTop(const QImage &frame, const ColSamples &frameCols, int rows, int trimTop);

private:
    /// @brief 确保缓冲在顶部与底部各有足够的空闲行,不足时重新分配。
    /// @param growTop 顶部需要的空闲行数。
    /// @param growBottom 底部需要的空闲行数。
    void ensureRoom(int growTop, int growBottom);

    QImage m_buffer;
    int m_top = 0;
    int m_used = 0;
    bool m_reserveTop = false;
    ColSamples m_cols;
    quint64 m_revision = 0;
    mutable QImage m_cache;
    mutable quint64 m_cacheRevision = 0;
    mutable bool m_cacheValid = false;
};

}  // namespace markshot::scroll
