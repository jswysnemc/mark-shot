#include "scroll/stitcher.h"

#include "scroll/stitcher_internal.h"

#include <algorithm>
#include <cmath>

namespace markshot::scroll {
using namespace stitcher_internal;

// The stitcher keeps one normalized vertical pipeline regardless of UI axis:
// every placement is an absolute top position in the long image, and only the
// final output is transposed back for horizontal sessions.

StitchResult Stitcher::pushFrame(const QImage &rawFrame)
{
    if (rawFrame.isNull() || rawFrame.width() <= 0 || rawFrame.height() <= 0) {
        logStitchDebug("drop invalid-frame raw_w=%d raw_h=%d",
                       rawFrame.width(), rawFrame.height());
        return StitchResult{StitchStatus::NoMatch, 0};
    }

    // Horizontal capture runs the whole vertical pipeline on a transposed frame;
    // everything below works in the transposed (vertical) space, and fullImage()
    // transposes the accumulated result back for output.
    const QImage frame = normalizePixelImage(
        m_axis == ScrollAxis::Horizontal ? transposeImage(rawFrame) : rawFrame);

    // 行签名整帧只计算一次,相邻匹配、全图搜索与长图增量追加全部复用
    const ColSamples frameCols = frame_profile::computeColSamples(frame);

    if (m_long.isNull()) {
        return initializeSeed(frame, frameCols);
    }

    // Stitching assumes equal-width frames; drop a frame whose width drifted.
    if (frame.width() != m_long.width()) {
        logStitchDebug("drop width-mismatch alg=%s axis=%s frame_w=%d full_w=%d frame_h=%d full_h=%d",
                       algorithmDebugName(), axisDebugName(m_axis),
                       frame.width(), m_long.width(), frame.height(), m_long.height());
        return StitchResult{StitchStatus::NoMatch, 0};
    }

    const std::pair<int, float> match = findOffsetColSample(frameCols);
    const int offset = match.first;
    const float confidence = match.second;

    const int fh = frame.height();
    const int H = m_long.height();
    const int predictedPos = m_anchorPos + offset;
    auto overhang = [&](int pos, int trimBottom, StitchEdge *edge) {
        return overhangAmount(pos, fh, H - trimBottom, edge);
    };

    // 固定区域观察每帧至多执行一次:位置可信后用相邻两帧识别固定带,
    // 结果既供本帧裁剪,也通过检测器状态影响后续帧的匹配窗口
    FixedRegionDetector::Observation fixedObs;
    bool fixedObserved = false;
    auto observeFixedRegions = [&](int trustedOffset) {
        if (!fixedObserved) {
            fixedObs = m_fixedRegions.observe(m_lastFrame, frame, trustedOffset);
            fixedObserved = true;
        }
    };

    // NoProgress paths update the anchor when the frame is recognized inside the
    // known image; pending paths leave the anchor unchanged until enough new
    // content accumulates to cross minAppend.
    auto adoptKnownFrame = [&](int pos, int appliedOffset) {
        observeFixedRegions(appliedOffset);
        m_anchorPos = pos;
        rememberFrame(frame, frameCols);
        m_lastOffset = appliedOffset;
        m_pendingEdge = StitchEdge::None;
        return StitchResult{StitchStatus::NoProgress, 0, StitchEdge::None, pos, fh};
    };

    auto holdPendingFrame = [&](int pos, StitchEdge pendingEdge) {
        m_pendingEdge = pendingEdge;
        return StitchResult{StitchStatus::NoProgress, 0, pendingEdge, pos, fh};
    };

    // Absolute position of this frame's top edge within the long image. New
    // content exists only where the frame overhangs [0, H). If the adjacent-frame
    // match is weak, recover by matching the current frame against the known edge
    // of the accumulated image; this keeps a single bad frame from breaking all
    // later captures.
    int newPos = predictedPos;
    int appliedOffset = offset;
    float effectiveConfidence = confidence;
    StitchEdge edge = StitchEdge::None;
    int knownPos = -1;
    float knownDiff = kNoMatchConfidence;
    bool usedKnown = false;
    int edgeRecoveryPos = -1;
    float edgeRecoveryDiff = kNoMatchConfidence;
    bool usedEdgeRecovery = false;
    bool usedLineRecovery = false;
    StitchEdge lineEdge = StitchEdge::None;
    int lineTrim = 0;
    int bottomTrim = 0;
    int lineMatchedRows = 0;

    const bool anchorNearEdge = m_anchorPos <= m_config.minAppend
        || m_anchorPos + fh >= H - m_config.minAppend;
    auto tryEdgeRecovery = [&]() {
        // Edge recovery is deliberately separated from adjacent-frame matching:
        // a single noisy frame can fail against m_lastFrame while still matching
        // the stable accumulated edge.
        const std::pair<int, float> edgePlacement = findEdgePosition(frameCols, predictedPos);
        edgeRecoveryPos = edgePlacement.first;
        edgeRecoveryDiff = edgePlacement.second;
        if (edgePlacement.second > m_config.acceptDiff) {
            const EdgeLineMatch linePlacement = findLineRunPosition(frame, predictedPos);
            edgeRecoveryPos = linePlacement.position;
            edgeRecoveryDiff = linePlacement.diff;
            if (linePlacement.diff > m_config.acceptDiff || linePlacement.matchedRows < m_config.minOverlap) {
                return false;
            }
            newPos = linePlacement.position;
            appliedOffset = newPos - m_anchorPos;
            effectiveConfidence = linePlacement.diff;
            lineEdge = linePlacement.edge;
            lineTrim = linePlacement.trim;
            bottomTrim = linePlacement.edge == StitchEdge::End ? linePlacement.trim : 0;
            lineMatchedRows = linePlacement.matchedRows;
            usedEdgeRecovery = true;
            usedLineRecovery = true;
            return true;
        }
        newPos = edgePlacement.first;
        appliedOffset = newPos - m_anchorPos;
        effectiveConfidence = edgePlacement.second;
        lineEdge = StitchEdge::None;
        lineTrim = 0;
        bottomTrim = 0;
        lineMatchedRows = 0;
        usedEdgeRecovery = true;
        usedLineRecovery = false;
        return true;
    };

    if (confidence > m_config.acceptDiff) {
        if (anchorNearEdge && tryEdgeRecovery()) {
            logStitchDebug("recover-edge-after-reject alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                           "anchor=%d pred=%d edge_pos=%d edge_diff=%.3f H=%d fh=%d",
                           algorithmDebugName(), axisDebugName(m_axis),
                           offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                           edgeRecoveryPos, edgeRecoveryDiff, H, fh);
        } else {
            if (H >= fh) {
                const std::pair<int, float> known = findKnownPosition(frameCols, predictedPos);
                knownPos = known.first;
                knownDiff = known.second;
                if (known.second <= m_config.acceptDiff) {
                    logStitchDebug("adopt-known-after-reject alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                                   "anchor=%d pred=%d known=%d known_diff=%.3f H=%d fh=%d",
                                   algorithmDebugName(), axisDebugName(m_axis),
                                   offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                                   known.first, known.second, H, fh);
                    return adoptKnownFrame(known.first, known.first - m_anchorPos);
                }
            }
            if (!usedEdgeRecovery && !tryEdgeRecovery()) {
                logStitchDebug("reject-confidence alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                               "anchor=%d pred=%d known=%d known_diff=%.3f edge_pos=%d "
                               "edge_diff=%.3f H=%d fh=%d",
                               algorithmDebugName(), axisDebugName(m_axis),
                               offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                               knownPos, knownDiff, edgeRecoveryPos, edgeRecoveryDiff, H, fh);
                return StitchResult{StitchStatus::NoMatch, 0};
            }
            if (usedEdgeRecovery) {
                logStitchDebug("recover-edge-after-known-miss alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                               "anchor=%d pred=%d known=%d known_diff=%.3f edge_pos=%d "
                               "edge_diff=%.3f H=%d fh=%d",
                               algorithmDebugName(), axisDebugName(m_axis),
                               offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                               knownPos, knownDiff, edgeRecoveryPos, edgeRecoveryDiff, H, fh);
            }
        }
    }

    // 位置已可信:先观察固定区域,再把检测到的固定带并入生长边的裁剪量。
    // 裁掉的行与新切片中的同内容行等价,误判时结果与不裁剪一致,因此本帧
    // 观察结果可以立即生效
    observeFixedRegions(appliedOffset);
    int topTrim = 0;
    if (usedLineRecovery) {
        if (lineEdge == StitchEdge::Start) {
            topTrim = lineTrim;
        }
    } else {
        StitchEdge probeEdge = StitchEdge::None;
        const int probeAmount = overhang(newPos, bottomTrim, &probeEdge);
        if (probeAmount > 0 && probeEdge == StitchEdge::End) {
            const int fixedFooter = fixedObs.valid ? fixedObs.footer : m_fixedRegions.matchBottomIgnore();
            const int maxTrim = std::min({H - 1, fh / 3, H / 3, H - newPos});
            bottomTrim = std::clamp(std::max(bottomTrim, fixedFooter), 0, std::max(0, maxTrim));
        } else if (probeAmount > 0 && probeEdge == StitchEdge::Start) {
            const int fixedHeader = fixedObs.valid ? fixedObs.header : m_fixedRegions.matchTopIgnore();
            topTrim = std::clamp(fixedHeader, 0, std::max(0, std::min(fh - probeAmount, H - 1)));
        }
    }

    int amount = overhang(newPos, bottomTrim, &edge);
    int overlapLen = 0;
    float edgeDiff = 0.0f;
    auto refreshEdgeOverlap = [&]() {
        amount = overhang(newPos, bottomTrim, &edge);
        overlapLen = 0;
        if (amount <= 0) {
            edgeDiff = 0.0f;
            return;
        }
        if (usedLineRecovery && edge == lineEdge) {
            overlapLen = lineMatchedRows;
            edgeDiff = effectiveConfidence;
        } else {
            // 长图上即将被裁掉的旧固定带不参与验证,防止它与帧内容互比
            const int topExclude = edge == StitchEdge::Start ? topTrim : 0;
            const int bottomExclude = edge == StitchEdge::End ? bottomTrim : 0;
            edgeDiff = knownOverlapDiff(frameCols, newPos, &overlapLen, topExclude, bottomExclude);
        }
    };
    refreshEdgeOverlap();

    const bool switchingGrowthEdge =
        m_growthEdge != StitchEdge::None && edge != StitchEdge::None && edge != m_growthEdge;
    const bool switchingAtBoundary =
        (edge == StitchEdge::Start && m_anchorPos <= m_config.minAppend && newPos <= 0)
        || (edge == StitchEdge::End
            && m_anchorPos + fh >= H - m_config.minAppend
            && newPos + fh >= H - bottomTrim);
    const bool trustedOppositeEdgeSwitch =
        switchingGrowthEdge
        && switchingAtBoundary
        && effectiveConfidence <= m_config.acceptDiff
        && std::abs(appliedOffset) >= m_config.minAppend
        && overlapLen >= m_config.minOverlap;

    if (amount > 0 && H >= fh) {
        if (!trustedOppositeEdgeSwitch
            && (edgeDiff > m_config.acceptDiff || overlapLen < m_config.minOverlap)) {
            const std::pair<int, float> known = findKnownPosition(frameCols, predictedPos);
            knownPos = known.first;
            knownDiff = known.second;
            if (known.second <= m_config.acceptDiff) {
                newPos = known.first;
                bottomTrim = 0;
                topTrim = 0;
                lineMatchedRows = 0;
                usedLineRecovery = false;
                lineEdge = StitchEdge::None;
                amount = overhang(newPos, bottomTrim, &edge);
                edgeDiff = 0.0f;
                overlapLen = fh;
                usedKnown = true;
            }
        }
    }

    const int heightDelta = edge == StitchEdge::End ? amount - bottomTrim : amount;
    if (heightDelta >= m_config.minAppend) {
        // Once growth starts on one edge, require strong evidence before
        // switching to the opposite edge. This rejects jitter near repeated
        // content while still allowing the user to reverse at a real boundary.
        auto canSwitchGrowthEdge = [&]() {
            if (m_growthEdge == StitchEdge::None || edge == StitchEdge::None || edge == m_growthEdge) {
                return true;
            }
            if (trustedOppositeEdgeSwitch) {
                return true;
            }
            if (edgeDiff > m_config.acceptDiff || overlapLen < m_config.minOverlap) {
                return false;
            }
            if (edge == StitchEdge::Start) {
                return newPos <= 0;
            }
            if (edge == StitchEdge::End) {
                return newPos + fh >= H - bottomTrim;
            }
            return false;
        };
        if (m_pendingEdge != StitchEdge::None && edge != StitchEdge::None && edge != m_pendingEdge) {
            logStitchDebug("reject-opposite-pending alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                           "anchor=%d pred=%d pos=%d amount=%d edge=%s pending_edge=%s "
                           "edge_diff=%.3f overlap=%d H=%d fh=%d",
                           algorithmDebugName(), axisDebugName(m_axis),
                           offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                           newPos, amount, edgeDebugName(edge), edgeDebugName(m_pendingEdge),
                           edgeDiff, overlapLen, H, fh);
            m_pendingEdge = StitchEdge::None;
            return StitchResult{StitchStatus::NoMatch, 0};
        }
        if (!canSwitchGrowthEdge()) {
            logStitchDebug("reject-opposite-growth alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                           "anchor=%d pred=%d pos=%d amount=%d edge=%s growth_edge=%s "
                           "edge_diff=%.3f overlap=%d H=%d fh=%d",
                           algorithmDebugName(), axisDebugName(m_axis),
                           offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                           newPos, amount, edgeDebugName(edge), edgeDebugName(m_growthEdge),
                           edgeDiff, overlapLen, H, fh);
            return StitchResult{StitchStatus::NoMatch, 0};
        }
        if (m_growthEdge != StitchEdge::None && edge != StitchEdge::None && edge != m_growthEdge) {
            logStitchDebug("switch-growth-edge alg=%s axis=%s old=%s new=%s "
                           "anchor=%d pred=%d pos=%d amount=%d H=%d fh=%d",
                           algorithmDebugName(), axisDebugName(m_axis),
                           edgeDebugName(m_growthEdge), edgeDebugName(edge),
                           m_anchorPos, predictedPos, newPos, amount, H, fh);
        }
        if (edgeDiff > m_config.acceptDiff || overlapLen < m_config.minOverlap) {
            if (trustedOppositeEdgeSwitch) {
                logStitchDebug("allow-opposite-growth-edge alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                               "anchor=%d pred=%d pos=%d amount=%d edge=%s growth_edge=%s "
                               "edge_diff=%.3f overlap=%d H=%d fh=%d",
                               algorithmDebugName(), axisDebugName(m_axis),
                               offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                               newPos, amount, edgeDebugName(edge), edgeDebugName(m_growthEdge),
                               edgeDiff, overlapLen, H, fh);
            } else {
                logStitchDebug("reject-edge-overlap alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                               "anchor=%d pred=%d pos=%d amount=%d edge=%s edge_diff=%.3f overlap=%d "
                               "known=%d known_diff=%.3f used_known=%d edge_recovery=%d "
                               "line_recovery=%d edge_pos=%d edge_recovery_diff=%.3f bottom_trim=%d "
                               "line_rows=%d H=%d fh=%d",
                               algorithmDebugName(), axisDebugName(m_axis),
                               offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                               newPos, amount, edgeDebugName(edge), edgeDiff, overlapLen, knownPos,
                               knownDiff, usedKnown ? 1 : 0, usedEdgeRecovery ? 1 : 0,
                               usedLineRecovery ? 1 : 0, edgeRecoveryPos, edgeRecoveryDiff,
                               bottomTrim, lineMatchedRows, H, fh);
                return StitchResult{StitchStatus::NoMatch, 0};
            }
        }

        if (edge == StitchEdge::End) {
            // End:切片为帧底部 amount 行,先裁掉长图尾部残留的旧固定带
            m_long.appendBottom(frame, frameCols, amount, bottomTrim);
            m_anchorPos = newPos;
            if (bottomTrim > 0) {
                m_bestBottomTrim = std::max(m_bestBottomTrim, bottomTrim);
            }
        } else {
            // Start:前插切片含帧顶固定带,同时裁掉长图顶部残留的旧固定带,
            // header 始终只在长图最顶端保留一份
            topTrim = std::clamp(topTrim, 0, std::max(0, std::min(fh - amount, H - 1)));
            m_long.prependTop(frame, frameCols, amount + topTrim, topTrim);
            m_anchorPos = newPos + amount;
            if (topTrim > 0) {
                m_bestTopTrim = std::max(m_bestTopTrim, topTrim);
            }
        }
        m_axisLocked = true;               // orientation fixed once the image grew
        m_pendingEdge = StitchEdge::None;
        if (edge != StitchEdge::None) {
            m_growthEdge = edge;
        }
        rememberFrame(frame, frameCols);
        m_lastOffset = appliedOffset;
        m_stats.frameCount += 1;
        m_stats.totalHeight = m_long.height();
        m_stats.lastAppend = heightDelta;
        logStitchDebug("append alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                       "old_anchor=%d pred=%d pos=%d amount=%d edge=%s edge_diff=%.3f overlap=%d "
                       "known=%d known_diff=%.3f used_known=%d edge_recovery=%d line_recovery=%d edge_pos=%d "
                       "edge_recovery_diff=%.3f bottom_trim=%d top_trim=%d line_rows=%d "
                       "height_delta=%d H=%d new_H=%d fh=%d frames=%d",
                       algorithmDebugName(), axisDebugName(m_axis),
                       offset, effectiveConfidence, m_config.acceptDiff, predictedPos - offset, predictedPos,
                       m_anchorPos, amount, edgeDebugName(edge), edgeDiff, overlapLen, knownPos,
                       knownDiff, usedKnown ? 1 : 0, usedEdgeRecovery ? 1 : 0,
                       usedLineRecovery ? 1 : 0, edgeRecoveryPos, edgeRecoveryDiff,
                       bottomTrim, topTrim, lineMatchedRows, heightDelta, H, m_long.height(), fh,
                       m_stats.frameCount);
        return StitchResult{StitchStatus::Appended, heightDelta, edge, m_anchorPos, fh};
    }

    if (amount == 0) {
        // Fully inside the long image: back-scroll or re-scroll over seen content.
        // Track the position and re-anchor so the next match basis stays local;
        // never re-stitch. This is what stops dirty-data duplication.
        logStitchDebug("inside-known alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                       "anchor=%d pred=%d pos=%d known=%d known_diff=%.3f used_known=%d H=%d fh=%d",
                       algorithmDebugName(), axisDebugName(m_axis),
                       offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos, newPos,
                       knownPos, knownDiff, usedKnown ? 1 : 0, H, fh);
        return adoptKnownFrame(newPos, appliedOffset);
    }
    // else: a sub-minAppend sliver is forming past an edge. Remember only its
    // direction so jitter cannot flip the next append; keep the anchor frame
    // unchanged until movement clears minAppend.
    logStitchDebug("wait-min-append alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                   "anchor=%d pred=%d pos=%d amount=%d edge=%s edge_diff=%.3f overlap=%d "
                   "min_append=%d known=%d known_diff=%.3f used_known=%d edge_recovery=%d H=%d fh=%d",
                   algorithmDebugName(), axisDebugName(m_axis),
                   offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos, newPos,
                   amount, edgeDebugName(edge), edgeDiff, overlapLen, m_config.minAppend,
                   knownPos, knownDiff, usedKnown ? 1 : 0, usedEdgeRecovery ? 1 : 0, H, fh);
    return holdPendingFrame(newPos, edge);
}

}  // namespace markshot::scroll
