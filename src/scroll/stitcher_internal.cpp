#include "scroll/stitcher_internal.h"

#include "debug_log.h"

#include <algorithm>
#include <cmath>
#include <cstdarg>
#include <vector>

namespace markshot::scroll::stitcher_internal {

// 分块转置的块边长,按缓存行友好取值
constexpr int kTransposeBlock = 32;

std::pair<int, int> bandRange(int width, float startRatio, float endRatio)
{
    const int start = std::clamp(static_cast<int>(std::lround(width * startRatio)), 0, width - 1);
    const int end = std::clamp(static_cast<int>(std::lround(width * endRatio)), start, width - 1);
    return {start, end};
}

int overhangAmount(int pos, int frameHeight, int fullHeight, StitchEdge *edge)
{
    const int overTop = std::max(0, -pos);
    const int overBottom = std::max(0, pos + frameHeight - fullHeight);
    if (edge) {
        *edge = overBottom >= overTop ? StitchEdge::End : StitchEdge::Start;
    }
    return std::max(overTop, overBottom);
}

int sideIgnoreWidth(int width)
{
    if (width <= 0) {
        return 0;
    }
    const int wide = std::min(std::max(50, width / 20), width / 3);
    return std::min(wide, std::max(0, (width - 1) / 2));
}

// Search order centred on the predicted offset, expanding outward:
// [p, p+1, p-1, p+2, p-2, ...], clamped to [-max, +max]. Signed so reverse
// scrolling (negative offsets) is searched too: the previous offset carries its
// sign as the prediction centre, giving reverse momentum just like forward.
/// @brief 生成以预测值为中心向两侧展开的搜索偏移序列。
/// @param max 最大搜索偏移,范围为 [-max, max]。
/// @param predict 预测的中心偏移。
/// @return 按与预测值距离升序排列的偏移序列。
std::vector<int> predictOffsetIter(int max, int predict)
{
    const int m = std::max(0, max);
    const int p = std::clamp(predict, -m, m);
    std::vector<int> result;
    result.reserve(static_cast<std::size_t>(m) * 2 + 1);
    result.push_back(p);
    for (int delta = 1; delta <= m; ++delta) {
        if (p + delta <= m) {
            result.push_back(p + delta);
        }
        if (p - delta >= -m) {
            result.push_back(p - delta);
        }
    }
    return result;
}

// Swaps the X and Y axes of an image: (x, y) -> (y, x). Self-inverse. Used to
// run the entire vertical stitching pipeline on horizontally-scrolled frames by
// transposing on the way in and transposing the accumulated result on the way
// out. Walks the source in cache-friendly blocks.
QImage transposeImage(const QImage &src)
{
    if (src.isNull()) {
        return src;
    }
    const QImage rgb = src.format() == QImage::Format_ARGB32_Premultiplied
        ? src
        : src.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const int w = rgb.width();
    const int h = rgb.height();
    QImage dst(h, w, QImage::Format_ARGB32_Premultiplied);

    // 1. 预先缓存目标行指针,避免内层循环反复调用 scanLine
    std::vector<QRgb *> dstLines(static_cast<std::size_t>(w));
    for (int x = 0; x < w; ++x) {
        dstLines[static_cast<std::size_t>(x)] = reinterpret_cast<QRgb *>(dst.scanLine(x));
    }

    // 2. 按块遍历源图,块内行列互换,提升两侧访问的缓存局部性
    for (int by = 0; by < h; by += kTransposeBlock) {
        const int yEnd = std::min(by + kTransposeBlock, h);
        for (int bx = 0; bx < w; bx += kTransposeBlock) {
            const int xEnd = std::min(bx + kTransposeBlock, w);
            for (int y = by; y < yEnd; ++y) {
                const QRgb *srcLine = reinterpret_cast<const QRgb *>(rgb.constScanLine(y));
                for (int x = bx; x < xEnd; ++x) {
                    dstLines[static_cast<std::size_t>(x)][y] = srcLine[x];
                }
            }
        }
    }
    return dst;
}

QImage normalizePixelImage(QImage image)
{
    if (!image.isNull()) {
        if (image.format() != QImage::Format_ARGB32_Premultiplied) {
            image = image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        }
        image.setDevicePixelRatio(1.0);
    }
    return image;
}

const char *algorithmDebugName()
{
    return "col-sample";
}

const char *axisDebugName(ScrollAxis axis)
{
    return axis == ScrollAxis::Horizontal ? "horizontal" : "vertical";
}

const char *edgeDebugName(StitchEdge edge)
{
    switch (edge) {
    case StitchEdge::Start:
        return "start";
    case StitchEdge::End:
        return "end";
    case StitchEdge::None:
        return "none";
    }
    return "unknown";
}

void logStitchDebug(const char *format, ...)
{
    if (!markshot::debugEnabled()) {
        return;
    }
    va_list args;
    va_start(args, format);
    markshot::debugLogV("stitch", format, args);
    va_end(args);
}

}  // namespace markshot::scroll::stitcher_internal
