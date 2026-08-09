#include "scroll/stitcher.h"

#include "scroll/stitcher_internal.h"

namespace markshot::scroll {
using namespace stitcher_internal;

StitchConfig defaultConfig()
{
    return StitchConfig{100, 9.0f, 15, 1.0f};
}

Stitcher::Stitcher(StitchConfig config) : m_config(config) {}

/// @brief 用首帧初始化长图与全部匹配状态。
/// @param frame 归一化后的种子帧。
/// @param frameCols 种子帧的行签名。
/// @return FirstFrame 状态的拼接结果。
StitchResult Stitcher::initializeSeed(const QImage &frame, const ColSamples &frameCols)
{
    m_long.reset(frame, frameCols);
    m_fixedRegions.reset();
    m_anchorPos = 0;
    m_bestBottomTrim = 0;
    m_bestTopTrim = 0;
    m_pendingEdge = StitchEdge::None;
    m_growthEdge = StitchEdge::None;
    m_stats.frameCount = 1;
    m_stats.totalHeight = frame.height();
    m_stats.lastAppend = frame.height();
    rememberFrame(frame, frameCols);
    logStitchDebug("first-frame alg=%s axis=%s frame=%dx%d full=%dx%d",
                   algorithmDebugName(), axisDebugName(m_axis),
                   frame.width(), frame.height(), m_long.width(), m_long.height());
    return StitchResult{StitchStatus::FirstFrame, frame.height(), StitchEdge::None, 0, frame.height()};
}

/// @brief 记录当前帧作为下一次相邻匹配的基准。
/// @param frame 归一化后的帧。
/// @param frameCols 该帧的行签名,复用调用方已算好的结果。
void Stitcher::rememberFrame(const QImage &frame, const ColSamples &frameCols)
{
    m_lastFrame = frame;
    m_lastCols = frameCols;
}

float Stitcher::knownOverlapDiff(const ColSamples &frameCols, int framePos, int *overlapLen,
                                 int fullTopExclude, int fullBottomExclude) const
{
    return frame_profile::overlapColDiff(m_long.cols(), frameCols, framePos,
                                         m_config.minOverlap,
                                         m_fixedRegions.matchTopIgnore(),
                                         m_fixedRegions.matchBottomIgnore(),
                                         overlapLen, fullTopExclude, fullBottomExclude);
}

QImage Stitcher::fullImage() const
{
    if (m_long.isNull()) {
        return QImage();
    }
    // 长图与横向转置结果都按修订号缓存,预览重复读取时不再逐次重建
    if (!m_fullCacheValid || m_fullCacheRevision != m_long.revision()) {
        const QImage compact = m_long.image();
        m_fullCache = m_axis == ScrollAxis::Horizontal ? transposeImage(compact) : compact;
        m_fullCache.setDevicePixelRatio(1.0);
        m_fullCacheRevision = m_long.revision();
        m_fullCacheValid = true;
    }
    return m_fullCache;
}

StitchStats Stitcher::stats() const
{
    return m_stats;
}

ScrollAxis Stitcher::axis() const
{
    return m_axis;
}

bool Stitcher::axisLocked() const
{
    return m_axisLocked;
}

void Stitcher::setAxis(ScrollAxis axis)
{
    if (m_axis == axis) {
        return;
    }
    // The axis locks only once the long image has actually grown in a direction
    // (the first directional stitch). Before that, the user may still flip it:
    // capturing the seed frame alone does not commit an orientation. A duplicate
    // (un-scrolled) frame is dropped before pushFrame, so the seed sits idle and
    // switchable until the user scrolls.
    if (m_axisLocked) {
        return;
    }
    m_axis = axis;
    m_pendingEdge = StitchEdge::None;
    m_growthEdge = StitchEdge::None;
    m_fixedRegions.reset();
    m_fullCacheValid = false;

    // The seed state is stored in the pipeline's (vertical) space, which is the
    // transpose of the captured frame when horizontal. Flipping the axis flips
    // that mapping, so re-transpose whatever seed we hold to match the new space.
    if (!m_long.isNull()) {
        const QImage seed = transposeImage(m_long.image());
        m_long.reset(seed, frame_profile::computeColSamples(seed));
        m_bestBottomTrim = 0;
        m_bestTopTrim = 0;
        m_stats.totalHeight = m_long.height();
        m_stats.lastAppend = m_long.height();
    }
    if (!m_lastFrame.isNull()) {
        m_lastFrame = transposeImage(m_lastFrame);
        m_lastCols = frame_profile::computeColSamples(m_lastFrame);
    }
}

}  // namespace markshot::scroll
