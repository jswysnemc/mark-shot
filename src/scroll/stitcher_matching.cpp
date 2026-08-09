#include "scroll/stitcher.h"

#include "scroll/stitcher_internal.h"

#include <algorithm>
#include <cmath>
#include <limits>
#include <vector>

namespace markshot::scroll {
using namespace stitcher_internal;
using frame_profile::kLineRowMaxDiff;

namespace {

// 相邻帧匹配中围绕预测偏移做逐像素精搜的窗口半径
constexpr int kOffsetPredictWindow = 128;
// 预测窗口未命中后全范围粗搜的步长
constexpr int kOffsetCoarseStep = 3;

}  // namespace

/// @brief 在上一帧的行签名上搜索当前帧的相对位移。
/// @param frameCols 当前帧的行签名。
/// @return 位移与置信度(平均差,越小越可信)。
std::pair<int, float> Stitcher::findOffsetColSample(const ColSamples &frameCols) const
{
    if (m_lastCols.isEmpty() || frameCols.isEmpty()) {
        return {0, kNoMatchConfidence};
    }

    const int h1 = static_cast<int>(m_lastCols.size());
    const int maxOffset = std::max(0, h1 - m_config.minOverlap);
    const int fixedTop = m_fixedRegions.matchTopIgnore();
    const int fixedBottom = m_fixedRegions.matchBottomIgnore();

    int bestOffset = 0;
    float bestDiff = kNoMatchConfidence;
    int approachCount = 0;
    // 返回 true 表示已经足够好,可以提前结束搜索
    auto probe = [&](int offset) {
        const float diff = frame_profile::frameColDiff(m_lastCols, frameCols, offset,
                                                       m_config.minOverlap, fixedTop, fixedBottom);
        if (diff < bestDiff) {
            bestOffset = offset;
            bestDiff = diff;
        }
        if (bestDiff < m_config.approxDiff) {
            ++approachCount;
            if (approachCount > 10) {
                return true;
            }
            if (diff < m_config.approxDiff / 4.0f) {
                return true;
            }
        }
        return false;
    };

    // 1. 先在预测偏移附近由近及远精搜。人的滚动有惯性,下一次位移通常
    //    落在上一次位移附近,这一步覆盖绝大多数帧
    const int predicted = std::clamp(m_lastOffset, -maxOffset, maxOffset);
    for (int offset : predictOffsetIter(maxOffset, predicted)) {
        if (std::abs(offset - predicted) > kOffsetPredictWindow) {
            break;
        }
        if (probe(offset)) {
            return {bestOffset, bestDiff};
        }
    }
    if (bestDiff < m_config.approxDiff) {
        return {bestOffset, bestDiff};
    }

    // 2. 预测窗口未命中时全范围粗搜,跳过已精搜的窗口
    for (int offset = -maxOffset; offset <= maxOffset; offset += kOffsetCoarseStep) {
        if (std::abs(offset - predicted) <= kOffsetPredictWindow) {
            continue;
        }
        if (probe(offset)) {
            return {bestOffset, bestDiff};
        }
    }

    // 3. 在粗搜最优点附近做一像素精化,补回步长跳过的位置
    const int refineStart = std::max(-maxOffset, bestOffset - kOffsetCoarseStep);
    const int refineEnd = std::min(maxOffset, bestOffset + kOffsetCoarseStep);
    for (int offset = refineStart; offset <= refineEnd; ++offset) {
        if (probe(offset)) {
            break;
        }
    }

    return {bestOffset, bestDiff};
}

std::pair<int, float> Stitcher::findKnownPosition(const ColSamples &frameCols, int predictedPos) const
{
    const int fullH = m_long.height();
    const int frameH = static_cast<int>(frameCols.size());
    const int maxPos = fullH - frameH;
    if (maxPos < 0 || frameH <= 0) {
        return {0, kNoMatchConfidence};
    }

    const int predictedInsidePos = std::clamp(predictedPos, 0, maxPos);
    const float predictedDiff = knownOverlapDiff(frameCols, predictedInsidePos);
    if (predictedDiff <= m_config.acceptDiff) {
        return {predictedInsidePos, predictedDiff};
    }

    int bestPos = predictedInsidePos;
    float bestDiff = kNoMatchConfidence;
    int bestGoodPos = predictedInsidePos;
    float bestGoodDiff = kNoMatchConfidence;
    int bestGoodDistance = std::numeric_limits<int>::max();
    // Prefer an acceptable match nearest to the prediction, but keep the global
    // best diff as a diagnostic fallback when no placement meets acceptDiff.
    auto visit = [&](int pos) {
        pos = std::clamp(pos, 0, maxPos);
        const float diff = knownOverlapDiff(frameCols, pos);
        if (diff < bestDiff) {
            bestDiff = diff;
            bestPos = pos;
        }
        if (diff <= m_config.acceptDiff) {
            const int distance = std::abs(pos - predictedInsidePos);
            if (distance < bestGoodDistance
                || (distance == bestGoodDistance && diff < bestGoodDiff)) {
                bestGoodDistance = distance;
                bestGoodDiff = diff;
                bestGoodPos = pos;
            }
        }
    };

    constexpr int kCoarseStep = 8;
    visit(predictedPos);
    visit(m_anchorPos);
    visit(0);
    visit(maxPos);
    // Coarse pass finds the correct basin, then a one-pixel refinement around
    // the best basin handles slow scrolls and repeated row patterns.
    for (int pos = 0; pos <= maxPos; pos += kCoarseStep) {
        visit(pos);
    }
    visit(maxPos);

    const int refineStart = std::max(0, bestPos - kCoarseStep);
    const int refineEnd = std::min(maxPos, bestPos + kCoarseStep);
    for (int pos = refineStart; pos <= refineEnd; ++pos) {
        visit(pos);
    }

    if (bestGoodDiff <= m_config.acceptDiff) {
        return {bestGoodPos, bestGoodDiff};
    }
    return {bestPos, bestDiff};
}

std::pair<int, float> Stitcher::findEdgePosition(const ColSamples &frameCols, int predictedPos) const
{
    const int fullH = m_long.height();
    const int frameH = static_cast<int>(frameCols.size());
    if (fullH <= 0 || frameH <= 0) {
        return {0, kNoMatchConfidence};
    }

    const int minPos = m_config.minOverlap - frameH;
    const int maxPos = fullH - m_config.minOverlap;
    if (minPos > maxPos) {
        return {0, kNoMatchConfidence};
    }

    int bestPos = std::clamp(predictedPos, minPos, maxPos);
    float bestDiff = kNoMatchConfidence;
    auto visit = [&](int pos) {
        pos = std::clamp(pos, minPos, maxPos);
        // Edge recovery only considers placements that reveal new content; pure
        // inside-known movement is handled by findKnownPosition().
        StitchEdge edge = StitchEdge::None;
        if (overhangAmount(pos, frameH, fullH, &edge) <= 0) {
            return;
        }
        // 长图靠近生长边一侧的旧固定带即将被裁掉,评分时排除,防止固定带
        // 与帧的新内容互比导致真实位置被拒
        const int topExclude = edge == StitchEdge::Start ? m_fixedRegions.matchTopIgnore() : 0;
        const int bottomExclude = edge == StitchEdge::End ? m_fixedRegions.matchBottomIgnore() : 0;
        int overlapLen = 0;
        const float diff = knownOverlapDiff(frameCols, pos, &overlapLen, topExclude, bottomExclude);
        if (overlapLen >= m_config.minOverlap && diff < bestDiff) {
            bestDiff = diff;
            bestPos = pos;
        }
    };

    constexpr int kCoarseStep = 8;
    constexpr int kPredictionWindow = 160;
    auto scanRange = [&](int start, int end) {
        start = std::clamp(start, minPos, maxPos);
        end = std::clamp(end, minPos, maxPos);
        if (start > end) {
            std::swap(start, end);
        }
        for (int pos = start; pos <= end; pos += kCoarseStep) {
            visit(pos);
        }
        visit(end);
    };

    visit(predictedPos);
    visit(m_anchorPos);
    visit(m_anchorPos + m_lastOffset);

    // Scan both true edges plus a window around the prediction. This covers
    // forward scroll, reverse scroll, and recovery after one rejected frame.
    const int endEdgeStart = std::max(minPos, fullH - frameH + 1);
    const int endEdgeEnd = maxPos;
    const int startEdgeStart = minPos;
    const int startEdgeEnd = std::min(maxPos, -1);
    if (endEdgeStart <= endEdgeEnd) {
        scanRange(endEdgeStart, endEdgeEnd);
    }
    if (startEdgeStart <= startEdgeEnd) {
        scanRange(startEdgeStart, startEdgeEnd);
    }
    scanRange(predictedPos - kPredictionWindow, predictedPos + kPredictionWindow);

    if (bestDiff < kNoMatchConfidence) {
        const int refineStart = std::max(minPos, bestPos - kCoarseStep);
        const int refineEnd = std::min(maxPos, bestPos + kCoarseStep);
        for (int pos = refineStart; pos <= refineEnd; ++pos) {
            visit(pos);
        }
    }

    return {bestPos, bestDiff};
}

Stitcher::EdgeLineMatch Stitcher::findLineRunPosition(const QImage &frame, int predictedPos) const
{
    EdgeLineMatch best;
    if (m_long.isNull() || frame.isNull()
        || m_long.width() != frame.width()
        || m_long.height() <= 0
        || frame.height() <= 0) {
        return best;
    }

    const int fullH = m_long.height();
    const int frameH = frame.height();
    const int side = sideIgnoreWidth(frame.width());
    const int roiW = frame.width() - side * 2;
    if (roiW <= 0) {
        return best;
    }

    // 帧行访问器;帧在 pushFrame 入口已归一化为 ARGB32_Premultiplied
    auto frameLine = [&](int y) {
        return reinterpret_cast<const QRgb *>(frame.constScanLine(y));
    };
    auto rowDiff = [&](int fullY, int frameY) {
        return frame_profile::rowMeanAbsDiff(m_long.scanLine(fullY), frameLine(frameY), side, roiW);
    };
    // 候选优先级:更多匹配行,其次更低平均差,最后与预测位置更近
    auto lineMatchBetter = [&](const EdgeLineMatch &candidate, const EdgeLineMatch &current) {
        if (candidate.matchedRows != current.matchedRows) {
            return candidate.matchedRows > current.matchedRows;
        }
        if (std::abs(candidate.diff - current.diff) >= 0.001f) {
            return candidate.diff < current.diff;
        }
        return std::abs(candidate.position - predictedPos) < std::abs(current.position - predictedPos);
    };

    const int maxTrim = std::min({fullH - 1, frameH / 3, fullH / 3});
    const int minRows = m_config.minOverlap;
    const int matchLimit = std::max(minRows, frameH / 2);
    const int trimDetectFloor = std::max(8, m_config.minAppend / 2);

    // 组装某一侧的裁剪候选:历史最佳、检测器估计、以及边缘逐行探测结果
    auto collectTrims = [&](int remembered, int detector, int detected) {
        std::vector<int> trims{0};
        if (remembered > 0) {
            trims.push_back(std::clamp(remembered, 0, maxTrim));
        }
        if (detector > 0) {
            trims.push_back(std::clamp(detector, 0, maxTrim));
        }
        if (detected >= trimDetectFloor) {
            trims.push_back(std::clamp(detected, 0, maxTrim));
        }
        std::sort(trims.begin(), trims.end());
        trims.erase(std::unique(trims.begin(), trims.end()), trims.end());
        return trims;
    };

    // 1. End 侧:探测长图尾部与帧尾部重复的固定 footer 行数
    int detectedBottom = 0;
    while (detectedBottom < maxTrim
           && rowDiff(fullH - 1 - detectedBottom, frameH - 1 - detectedBottom) <= kLineRowMaxDiff) {
        ++detectedBottom;
    }

    // 2. End 侧:在候选裁剪下自帧底向上找逐像素相同的行运行
    for (int trim : collectTrims(m_bestBottomTrim, m_fixedRegions.matchBottomIgnore(), detectedBottom)) {
        const int edgeY = fullH - trim - 1;
        if (edgeY < minRows - 1) {
            continue;
        }
        for (int currentY = frameH - 1; currentY >= 0 && best.matchedRows < matchLimit; --currentY) {
            int rows = 0;
            float diffSum = 0.0f;
            // Walk upward while exact rows keep matching. This fallback is
            // stricter than the column sampler and is used only near an edge.
            while (rows < matchLimit && edgeY - rows >= 0 && currentY - rows >= 0) {
                const float diff = rowDiff(edgeY - rows, currentY - rows);
                if (diff > kLineRowMaxDiff) {
                    break;
                }
                diffSum += diff;
                ++rows;
            }
            if (rows < minRows) {
                continue;
            }

            StitchEdge edge = StitchEdge::None;
            const int pos = edgeY - currentY;
            const int appendRows = overhangAmount(pos, frameH, fullH - trim, &edge);
            if (edge != StitchEdge::End || appendRows - trim < m_config.minAppend) {
                continue;
            }

            EdgeLineMatch candidate{pos, diffSum / static_cast<float>(rows), trim, rows, StitchEdge::End};
            if (lineMatchBetter(candidate, best)) {
                best = candidate;
            }
        }
    }

    // 3. Start 侧:探测长图顶部与帧顶部重复的固定 header 行数
    int detectedTop = 0;
    while (detectedTop < maxTrim && rowDiff(detectedTop, detectedTop) <= kLineRowMaxDiff) {
        ++detectedTop;
    }

    // 4. Start 侧:反向滚动时自帧顶向下找行运行,镜像 End 侧逻辑
    for (int trim : collectTrims(m_bestTopTrim, m_fixedRegions.matchTopIgnore(), detectedTop)) {
        const int edgeY = trim;
        if (fullH - edgeY < minRows) {
            continue;
        }
        for (int currentY = 0; currentY < frameH && best.matchedRows < matchLimit; ++currentY) {
            int rows = 0;
            float diffSum = 0.0f;
            while (rows < matchLimit && edgeY + rows < fullH && currentY + rows < frameH) {
                const float diff = rowDiff(edgeY + rows, currentY + rows);
                if (diff > kLineRowMaxDiff) {
                    break;
                }
                diffSum += diff;
                ++rows;
            }
            if (rows < minRows) {
                continue;
            }

            // 帧顶相对长图的位置;净增行数为 -pos,前插切片为帧顶 currentY 行
            const int pos = edgeY - currentY;
            if (pos >= 0 || -pos < m_config.minAppend || currentY > frameH) {
                continue;
            }
            StitchEdge edge = StitchEdge::None;
            overhangAmount(pos, frameH, fullH, &edge);
            if (edge != StitchEdge::Start) {
                continue;
            }

            EdgeLineMatch candidate{pos, diffSum / static_cast<float>(rows), trim, rows, StitchEdge::Start};
            if (lineMatchBetter(candidate, best)) {
                best = candidate;
            }
        }
    }

    return best;
}

}  // namespace markshot::scroll
