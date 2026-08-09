#include "scroll/stitcher_frame_profile.h"

#include "scroll/stitcher.h"
#include "scroll/stitcher_internal.h"

#include <algorithm>
#include <cmath>

namespace markshot::scroll::frame_profile {

// 顶部按比例忽略的高度占比
constexpr float kColTopIgnoreRatio = 0.10f;
// 底部按比例忽略的高度占比
constexpr float kColBottomIgnoreRatio = 0.08f;
// 按比例忽略时的最小像素数
constexpr int kColMinIgnorePx = 16;
// 行像素比较最多抽取的采样列数
constexpr int kLineMaxSampleCols = 256;

/// @brief 按高度与占比计算边缘忽略行数。
/// @param height 帧高。
/// @param ratio 忽略占比。
/// @return 忽略的行数;帧太矮时为 0。
static int scaledIgnore(int height, float ratio)
{
    if (height < 80) {
        return 0;
    }
    return std::min(height / 4, std::max(kColMinIgnorePx, static_cast<int>(height * ratio)));
}

int contentTopIgnore(int height)
{
    return scaledIgnore(height, kColTopIgnoreRatio);
}

int contentBottomIgnore(int height)
{
    return scaledIgnore(height, kColBottomIgnoreRatio);
}

bool isContentRow(int y, int height)
{
    return y >= contentTopIgnore(height) && y < height - contentBottomIgnore(height);
}

bool shouldCropContentRows(int overlapLen, int frameHeight, int minOverlap)
{
    return overlapLen >= minOverlap + contentTopIgnore(frameHeight) + contentBottomIgnore(frameHeight);
}

int requiredComparedRows(int minOverlap, bool cropped)
{
    return cropped ? std::max(24, minOverlap / 2) : minOverlap;
}

ColSamples computeColSamples(const QImage &frame)
{
    const int w = frame.width();
    const int h = frame.height();
    if (w <= 0 || h <= 0) {
        return {};
    }

    // 1. 帧在进入拼接管线前已统一为 ARGB32_Premultiplied,直接按 QRgb 读取
    //    即可;仅在遇到其它格式时才做一次转换,避免每帧的整图拷贝
    QImage converted;
    const QImage *source = &frame;
    if (frame.format() != QImage::Format_ARGB32_Premultiplied
        && frame.format() != QImage::Format_RGB32
        && frame.format() != QImage::Format_ARGB32) {
        converted = frame.convertToFormat(QImage::Format_RGB32);
        source = &converted;
    }

    // 2. 三个横向采样带覆盖左/中/右区域,保留页面结构信息的同时控制成本
    const std::array<std::pair<int, int>, 3> bands = {
        stitcher_internal::bandRange(w, 0.08f, 0.32f),
        stitcher_internal::bandRange(w, 0.34f, 0.66f),
        stitcher_internal::bandRange(w, 0.68f, 0.92f),
    };

    ColSamples result(h);
    for (int g = 0; g < 3; ++g) {
        const int start = bands[g].first;
        const int end = bands[g].second;
        const int sampleCount = std::max(1, std::min(kColMaxBandSamples, end - start + 1));
        const float step = sampleCount > 1
            ? static_cast<float>(end - start) / static_cast<float>(sampleCount - 1)
            : 0.0f;
        // 3. 每行对采样列求亮度均值,形成该行在此带上的签名
        for (int y = 0; y < h; ++y) {
            const QRgb *line = reinterpret_cast<const QRgb *>(source->constScanLine(y));
            float sum = 0.0f;
            for (int i = 0; i < sampleCount; ++i) {
                const int x = std::clamp(static_cast<int>(std::lround(start + i * step)), 0, w - 1);
                const QRgb px = line[x];
                sum += 0.299f * qRed(px) + 0.587f * qGreen(px) + 0.114f * qBlue(px);
            }
            result[y][g] = sum / static_cast<float>(sampleCount);
        }
    }
    return result;
}

float rowMeanAbsDiff(const QRgb *a, const QRgb *b, int startX, int width)
{
    if (!a || !b || width <= 0) {
        return kNoMatchConfidence;
    }

    const int step = std::max(1, width / kLineMaxSampleCols);
    float sum = 0.0f;
    int count = 0;
    for (int x = startX; x < startX + width; x += step) {
        const QRgb ap = a[x];
        const QRgb bp = b[x];
        sum += std::abs(qRed(ap) - qRed(bp));
        sum += std::abs(qGreen(ap) - qGreen(bp));
        sum += std::abs(qBlue(ap) - qBlue(bp));
        count += 3;
    }
    return count > 0 ? sum / static_cast<float>(count) : kNoMatchConfidence;
}

/// @brief 判断帧内某行是否允许参与比较。
/// @param y 行号。
/// @param height 帧高。
/// @param fixedTop 顶部固定带高度。
/// @param fixedBottom 底部固定带高度。
/// @param softCrop 是否附加按比例的边缘裁剪。
/// @return 允许比较返回 true。
static bool usableRow(int y, int height, int fixedTop, int fixedBottom, bool softCrop)
{
    if (y < fixedTop || y >= height - fixedBottom) {
        return false;
    }
    return !softCrop || isContentRow(y, height);
}

float frameColDiff(const ColSamples &cols1,
                   const ColSamples &cols2,
                   int offset,
                   int minOverlap,
                   int fixedTop,
                   int fixedBottom)
{
    const int h1 = static_cast<int>(cols1.size());
    const int h2 = static_cast<int>(cols2.size());
    if (h1 == 0 || h2 == 0) {
        return kNoMatchConfidence;
    }

    // 1. 计算几何重叠长度,不足最小重叠直接放弃
    const int overlapLen = offset >= 0
        ? std::min(h1 - offset, h2)
        : std::min(h1, h2 + offset);
    if (overlapLen < minOverlap) {
        return kNoMatchConfidence;
    }

    // 2. 固定带行永远剔除;按比例裁剪仅在重叠足够长时附加
    const bool softCrop = shouldCropContentRows(overlapLen, std::min(h1, h2), minOverlap);
    const int start1 = offset >= 0 ? offset : 0;
    const int start2 = offset >= 0 ? 0 : -offset;
    float sum = 0.0f;
    int count = 0;
    int rows = 0;
    for (int i = 0; i < overlapLen; ++i) {
        const int y1 = start1 + i;
        const int y2 = start2 + i;
        if (!usableRow(y1, h1, fixedTop, fixedBottom, softCrop)
            || !usableRow(y2, h2, fixedTop, fixedBottom, softCrop)) {
            continue;
        }
        ++rows;
        for (int g = 0; g < 3; ++g) {
            sum += std::abs(cols1[y1][g] - cols2[y2][g]);
            ++count;
        }
    }

    // 3. 有效行不足以支撑判定时按无匹配处理
    if (count == 0 || rows < requiredComparedRows(minOverlap, softCrop || fixedTop + fixedBottom > 0)) {
        return kNoMatchConfidence;
    }
    return sum / static_cast<float>(count);
}

float overlapColDiff(const ColSamples &fullCols,
                     const ColSamples &frameCols,
                     int framePos,
                     int minOverlap,
                     int fixedTop,
                     int fixedBottom,
                     int *overlapLen,
                     int fullTopExclude,
                     int fullBottomExclude)
{
    const int fullH = static_cast<int>(fullCols.size());
    const int frameH = static_cast<int>(frameCols.size());
    // 长图两端即将被裁掉的旧固定带不参与比较,避免固定带与新内容互比
    fullTopExclude = std::clamp(fullTopExclude, 0, fullH);
    fullBottomExclude = std::clamp(fullBottomExclude, 0, fullH - fullTopExclude);
    const int fullStart = std::max(fullTopExclude, framePos);
    const int fullEnd = std::min(fullH - fullBottomExclude, framePos + frameH);
    const int len = fullEnd - fullStart;
    if (overlapLen) {
        *overlapLen = std::max(0, len);
    }
    if (len < minOverlap) {
        return kNoMatchConfidence;
    }

    // 1. 只在帧与长图的重叠区内比较;长图内容无固定带,窗口只作用于帧行
    const int frameStart = fullStart - framePos;
    const bool softCrop = shouldCropContentRows(len, frameH, minOverlap);
    float sum = 0.0f;
    int count = 0;
    int rows = 0;
    for (int row = 0; row < len; ++row) {
        const int frameY = frameStart + row;
        if (!usableRow(frameY, frameH, fixedTop, fixedBottom, softCrop)) {
            continue;
        }
        const std::array<float, 3> &fullPx = fullCols[fullStart + row];
        const std::array<float, 3> &framePx = frameCols[frameY];
        ++rows;
        for (int g = 0; g < 3; ++g) {
            sum += std::abs(fullPx[g] - framePx[g]);
            ++count;
        }
    }

    // 2. 有效行不足时拒绝,防止在稀疏比较上得出高置信结论
    if (count == 0 || rows < requiredComparedRows(minOverlap, softCrop || fixedTop + fixedBottom > 0)) {
        return kNoMatchConfidence;
    }
    return sum / static_cast<float>(count);
}

}  // namespace markshot::scroll::frame_profile
