#include "scroll/stitcher_fixed_regions.h"

#include "scroll/stitcher_frame_profile.h"
#include "scroll/stitcher_internal.h"

#include <algorithm>

namespace markshot::scroll {

namespace {

// 观察生效所需的最小滚动位移;低于该值时两帧几乎重合,逐行相同不能说明固定
constexpr int kMinScrollForDetect = 12;
// 固定带判定的行平均色差上限,取严以避免把近似内容误判为固定带
constexpr float kFixedRowMaxDiff = frame_profile::kLineRowMaxDiff;
// 固定带最大占帧高的比例分母(即至多 1/3)
constexpr int kMaxBandDivisor = 3;
// 上调 committed 值所需的连续确认次数
constexpr int kRaisePersistence = 2;

/// @brief 从帧顶或帧底起统计两帧连续相同的行数。
/// @param prev 上一帧。
/// @param cur 当前帧。
/// @param fromBottom true 时从帧底向上统计,false 时从帧顶向下。
/// @return 连续相同的行数,上限为帧高的三分之一。
int scanBand(const QImage &prev, const QImage &cur, bool fromBottom)
{
    const int h = cur.height();
    const int w = cur.width();
    // 1. 忽略两侧边缘,滚动条随滚动变化会破坏固定行的相同性判定
    const int side = stitcher_internal::sideIgnoreWidth(w);
    const int roiW = w - side * 2;
    if (roiW <= 0 || h <= 0) {
        return 0;
    }

    // 2. 逐行比较同一 y 位置,首个不同的行即固定带边界
    const int limit = h / kMaxBandDivisor;
    int count = 0;
    while (count < limit) {
        const int y = fromBottom ? h - 1 - count : count;
        const QRgb *prevLine = reinterpret_cast<const QRgb *>(prev.constScanLine(y));
        const QRgb *curLine = reinterpret_cast<const QRgb *>(cur.constScanLine(y));
        const float diff = frame_profile::rowMeanAbsDiff(prevLine, curLine, side, roiW);
        if (diff > kFixedRowMaxDiff) {
            break;
        }
        ++count;
    }
    return count;
}

}  // namespace

void FixedRegionDetector::BandState::update(int candidate)
{
    // 1. 首个样本直接采纳
    if (!hasSample) {
        committed = candidate;
        hasSample = true;
        pending = -1;
        pendingCount = 0;
        return;
    }

    // 2. 候选变小立即下调:说明之前把偶然相同的内容行计入了固定带
    if (candidate <= committed) {
        committed = candidate;
        pending = -1;
        pendingCount = 0;
        return;
    }

    // 3. 候选变大需连续确认才上调,期间取最小候选防止单帧巧合抬高估计
    if (pending < 0) {
        pending = candidate;
        pendingCount = 1;
        return;
    }
    pending = std::min(pending, candidate);
    ++pendingCount;
    if (pendingCount >= kRaisePersistence) {
        committed = pending;
        pending = -1;
        pendingCount = 0;
    }
}

void FixedRegionDetector::reset()
{
    m_header = BandState{};
    m_footer = BandState{};
}

FixedRegionDetector::Observation FixedRegionDetector::observe(const QImage &prevFrame,
                                                              const QImage &curFrame,
                                                              int offset)
{
    Observation result;
    // 1. 位移不足或帧不可比时不产生候选,也不污染平滑状态
    if (std::abs(offset) < kMinScrollForDetect
        || prevFrame.isNull() || curFrame.isNull()
        || prevFrame.size() != curFrame.size()) {
        return result;
    }

    // 2. 分别统计顶部与底部的连续相同行数
    result.header = scanBand(prevFrame, curFrame, false);
    result.footer = scanBand(prevFrame, curFrame, true);
    result.valid = true;

    // 3. 更新参与匹配裁剪的平滑估计
    m_header.update(result.header);
    m_footer.update(result.footer);
    return result;
}

int FixedRegionDetector::matchTopIgnore() const
{
    return m_header.committed;
}

int FixedRegionDetector::matchBottomIgnore() const
{
    return m_footer.committed;
}

}  // namespace markshot::scroll
