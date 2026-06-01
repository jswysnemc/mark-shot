#include "scroll/stitcher.h"

#include "scroll/stitch_orb.h"

#include <QPainter>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdarg>
#include <cstdio>
#include <vector>

namespace markshot::scroll {

namespace {

// linspace with n == 3, matching wayscrollshot's linspace(start, end, 3): the
// three column indices are start, midpoint, end (rounded).
std::array<int, 3> linspace3(int start, int end)
{
    const float step = static_cast<float>(end - start) / 2.0f;
    return {
        static_cast<int>(std::lround(start + 0.0f * step)),
        static_cast<int>(std::lround(start + 1.0f * step)),
        static_cast<int>(std::lround(start + 2.0f * step)),
    };
}

// Mean absolute difference of the overlapping region when cols2 (current frame)
// is shifted down by `offset` relative to cols1 (previous frame). Equivalent to
// wayscrollshot's compute_col_diff for offset >= 0; negative offsets are not
// used by the forward-scroll session.
float computeColDiff(const QVector<std::array<float, 3>> &cols1,
                     const QVector<std::array<float, 3>> &cols2,
                     int offset)
{
    const int h1 = static_cast<int>(cols1.size());
    const int h2 = static_cast<int>(cols2.size());
    if (h1 == 0 || h2 == 0) {
        return kNoMatchConfidence;
    }

    float sum = 0.0f;
    int count = 0;

    if (offset >= 0) {
        const int len = std::min(h1 - offset, h2 - offset);
        for (int i = 0; i < len; ++i) {
            const int y1 = offset + i;
            const int y2 = i;
            for (int g = 0; g < 3; ++g) {
                sum += std::abs(cols1[y1][g] - cols2[y2][g]);
                ++count;
            }
        }
    } else {
        const int o = -offset;
        const int len = std::min(h1 - o, h2 - o);
        for (int i = 0; i < len; ++i) {
            const int y1 = i;
            const int y2 = o + i;
            for (int g = 0; g < 3; ++g) {
                sum += std::abs(cols1[y1][g] - cols2[y2][g]);
                ++count;
            }
        }
    }

    if (count == 0) {
        return kNoMatchConfidence;
    }
    return sum / static_cast<float>(count);
}

// Search order centred on the predicted offset, expanding outward:
// [p, p+1, p-1, p+2, p-2, ...], clamped to [-max, +max]. Signed so reverse
// scrolling (negative offsets) is searched too: the previous offset carries its
// sign as the prediction centre, giving reverse momentum just like forward.
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
// out.
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
    for (int y = 0; y < h; ++y) {
        const QRgb *srcLine = reinterpret_cast<const QRgb *>(rgb.scanLine(y));
        for (int x = 0; x < w; ++x) {
            reinterpret_cast<QRgb *>(dst.scanLine(x))[y] = srcLine[x];
        }
    }
    return dst;
}

const char *algorithmDebugName(StitchAlgorithm algorithm)
{
    return algorithm == StitchAlgorithm::OpenCvOrb ? "opencv-orb" : "col-sample";
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
    FILE *file = std::fopen("/tmp/mark-shot-scroll.log", "a");
    if (!file) {
        return;
    }

    std::fprintf(file, "[stitch] ");
    va_list args;
    va_start(args, format);
    std::vfprintf(file, format, args);
    va_end(args);
    std::fprintf(file, "\n");
    std::fclose(file);
}

}  // namespace

StitchConfig defaultConfigFor(StitchAlgorithm algorithm)
{
    if (algorithm == StitchAlgorithm::OpenCvOrb) {
        return StitchConfig{120, 3.5f, 10, 1.0f, StitchAlgorithm::OpenCvOrb};
    }
    return StitchConfig{100, 9.0f, 15, 1.0f, StitchAlgorithm::ColSample};
}

bool openCvAvailable()
{
#ifdef HAVE_OPENCV
    return true;
#else
    return false;
#endif
}

Stitcher::Stitcher(StitchConfig config) : m_config(config) {}

Stitcher::ColSamples Stitcher::computeCols(const QImage &frame) const
{
    const int w = frame.width();
    const int h = frame.height();
    if (w <= 0 || h <= 0) {
        return {};
    }

    const std::array<std::array<int, 3>, 3> groups = {
        linspace3(std::min(20, w - 1), w / 4),
        linspace3(w / 2, 5 * w / 8),
        linspace3(6 * w / 8, 7 * w / 8),
    };

    ColSamples result(h);
    for (int g = 0; g < 3; ++g) {
        for (int y = 0; y < h; ++y) {
            float sum = 0.0f;
            int count = 0;
            for (int x : groups[g]) {
                if (x >= 0 && x < w) {
                    const QRgb px = frame.pixel(x, y);
                    sum += 0.299f * qRed(px) + 0.587f * qGreen(px) + 0.114f * qBlue(px);
                    ++count;
                }
            }
            result[y][g] = count > 0 ? sum / static_cast<float>(count) : 0.0f;
        }
    }
    return result;
}

std::pair<int, float> Stitcher::findOffsetColSample(const QImage &frame) const
{
    if (m_lastCols.isEmpty()) {
        return {0, kNoMatchConfidence};
    }

    const ColSamples cols = computeCols(frame);
    const int h1 = m_lastCols.size();
    if (h1 == 0 || cols.isEmpty()) {
        return {0, kNoMatchConfidence};
    }

    const int maxOffset = std::max(0, h1 - m_config.minOverlap);
    int bestOffset = 0;
    float bestDiff = kNoMatchConfidence;
    int approachCount = 0;

    for (int offset : predictOffsetIter(maxOffset, m_lastOffset)) {
        const float diff = computeColDiff(m_lastCols, cols, offset);
        if (diff < bestDiff) {
            bestOffset = offset;
            bestDiff = diff;
        }
        if (bestDiff < m_config.approxDiff) {
            ++approachCount;
            if (approachCount > 10) {
                break;
            }
            if (diff < m_config.approxDiff / 4.0f) {
                break;
            }
        }
    }

    return {bestOffset, bestDiff};
}

std::pair<int, float> Stitcher::findOffsetOrb(const QImage &frame) const
{
#ifdef HAVE_OPENCV
    if (m_lastFrame.isNull()) {
        return {0, kNoMatchConfidence};
    }

    if (const std::optional<OrbEstimate> estimate =
            estimateOrbOffset(m_lastFrame, frame, m_config.minOverlap)) {
        return {static_cast<int>(std::lround(estimate->dy)), estimate->confidence};
    }

    // Relaxed overlap retry, then NCC template fallback, mirroring
    // wayscrollshot's find_offset_opencv_orb.
    constexpr int kRelaxedOverlapFloor = 72;
    if (m_config.minOverlap > kRelaxedOverlapFloor) {
        const int relaxed = std::max(kRelaxedOverlapFloor, m_config.minOverlap - 40);
        if (const std::optional<OrbEstimate> estimate =
                estimateOrbOffset(m_lastFrame, frame, relaxed)) {
            return {static_cast<int>(std::lround(estimate->dy)), estimate->confidence + 0.45f};
        }
    }

    if (const std::optional<std::pair<int, float>> fallback =
            templateFallback(m_lastFrame, frame, m_lastOffset, m_config.minOverlap)) {
        return *fallback;
    }

    return {0, kNoMatchConfidence};
#else
    Q_UNUSED(frame);
    return {0, kNoMatchConfidence};
#endif
}

float Stitcher::knownOverlapDiff(const ColSamples &frameCols, int framePos, int *overlapLen) const
{
    const int fullH = m_fullCols.size();
    const int frameH = frameCols.size();
    const int fullStart = std::max(0, framePos);
    const int fullEnd = std::min(fullH, framePos + frameH);
    const int len = fullEnd - fullStart;
    if (overlapLen) {
        *overlapLen = std::max(0, len);
    }
    if (len < m_config.minOverlap) {
        return kNoMatchConfidence;
    }

    const int frameStart = fullStart - framePos;
    float sum = 0.0f;
    int count = 0;
    for (int row = 0; row < len; ++row) {
        const std::array<float, 3> &fullPx = m_fullCols[fullStart + row];
        const std::array<float, 3> &framePx = frameCols[frameStart + row];
        for (int g = 0; g < 3; ++g) {
            sum += std::abs(fullPx[g] - framePx[g]);
            ++count;
        }
    }
    return count > 0 ? sum / static_cast<float>(count) : kNoMatchConfidence;
}

std::pair<int, float> Stitcher::findKnownPosition(const ColSamples &frameCols, int predictedPos) const
{
    const int fullH = m_fullCols.size();
    const int frameH = frameCols.size();
    const int maxPos = fullH - frameH;
    if (maxPos < 0 || frameH <= 0) {
        return {0, kNoMatchConfidence};
    }

    int bestPos = 0;
    float bestDiff = kNoMatchConfidence;
    auto visit = [&](int pos) {
        pos = std::clamp(pos, 0, maxPos);
        const float diff = knownOverlapDiff(frameCols, pos);
        if (diff < bestDiff) {
            bestDiff = diff;
            bestPos = pos;
        }
    };

    constexpr int kCoarseStep = 8;
    visit(predictedPos);
    visit(m_anchorPos);
    visit(0);
    visit(maxPos);
    for (int pos = 0; pos <= maxPos; pos += kCoarseStep) {
        visit(pos);
    }
    visit(maxPos);

    const int refineStart = std::max(0, bestPos - kCoarseStep);
    const int refineEnd = std::min(maxPos, bestPos + kCoarseStep);
    for (int pos = refineStart; pos <= refineEnd; ++pos) {
        visit(pos);
    }

    return {bestPos, bestDiff};
}

void Stitcher::appendSlice(const QImage &frame, int amount)
{
    const int w = m_full.width();
    QImage combined(w, m_full.height() + amount, QImage::Format_ARGB32_Premultiplied);
    combined.fill(Qt::transparent);

    QPainter painter(&combined);
    painter.drawImage(0, 0, m_full);
    const int overlap = frame.height() - amount;
    const QImage slice = frame.copy(0, overlap, frame.width(), amount);
    painter.drawImage(0, m_full.height(), slice);
    painter.end();

    m_full = combined;
    m_fullCols = computeCols(m_full);
}

void Stitcher::prependSlice(const QImage &frame, int amount)
{
    const int w = m_full.width();
    QImage combined(w, m_full.height() + amount, QImage::Format_ARGB32_Premultiplied);
    combined.fill(Qt::transparent);

    QPainter painter(&combined);
    // The top `amount` rows of the current frame are the newly-revealed content
    // above the previous anchor; place them first, then the old image below.
    const QImage slice = frame.copy(0, 0, frame.width(), amount);
    painter.drawImage(0, 0, slice);
    painter.drawImage(0, amount, m_full);
    painter.end();

    m_full = combined;
    m_fullCols = computeCols(m_full);
}

void Stitcher::rememberFrame(const QImage &frame)
{
    if (m_config.algorithm == StitchAlgorithm::ColSample) {
        m_lastCols = computeCols(frame);
    }
    m_lastFrame = frame;
}

StitchResult Stitcher::pushFrame(const QImage &rawFrame)
{
    if (rawFrame.isNull() || rawFrame.width() <= 0 || rawFrame.height() <= 0) {
        logStitchDebug("drop invalid-frame raw_w=%d raw_h=%d",
                       rawFrame.width(), rawFrame.height());
        return StitchResult{StitchStatus::NoMatch, 0};
    }

    // Horizontal capture runs the whole vertical pipeline on a transposed frame;
    // everything below works in the transposed (vertical) space, and fullImage()
    // transposes m_full back for output.
    const QImage frame =
        m_axis == ScrollAxis::Horizontal ? transposeImage(rawFrame) : rawFrame;

    if (m_full.isNull()) {
        m_full = frame.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        m_fullCols = computeCols(m_full);
        m_anchorPos = 0;
        m_stats.frameCount = 1;
        m_stats.totalHeight = m_full.height();
        m_stats.lastAppend = m_full.height();
        rememberFrame(frame);
        logStitchDebug("first-frame alg=%s axis=%s frame=%dx%d full=%dx%d",
                       algorithmDebugName(m_config.algorithm), axisDebugName(m_axis),
                       frame.width(), frame.height(), m_full.width(), m_full.height());
        return StitchResult{StitchStatus::FirstFrame, m_full.height(), StitchEdge::None, 0, m_full.height()};
    }

    // Stitching assumes equal-width frames; drop a frame whose width drifted.
    if (frame.width() != m_full.width()) {
        logStitchDebug("drop width-mismatch alg=%s axis=%s frame_w=%d full_w=%d frame_h=%d full_h=%d",
                       algorithmDebugName(m_config.algorithm), axisDebugName(m_axis),
                       frame.width(), m_full.width(), frame.height(), m_full.height());
        return StitchResult{StitchStatus::NoMatch, 0};
    }

    const bool useOrb = m_config.algorithm == StitchAlgorithm::OpenCvOrb && openCvAvailable();
    const std::pair<int, float> match = useOrb ? findOffsetOrb(frame) : findOffsetColSample(frame);
    const int offset = match.first;
    const float confidence = match.second;

    const int fh = frame.height();
    const int H = m_full.height();
    const ColSamples frameCols = computeCols(frame);
    const int predictedPos = m_anchorPos + offset;
    auto overhang = [&](int pos, StitchEdge *edge) {
        const int overTop = std::max(0, -pos);
        const int overBottom = std::max(0, pos + fh - H);
        if (edge) {
            *edge = overBottom >= overTop ? StitchEdge::End : StitchEdge::Start;
        }
        return std::max(overTop, overBottom);
    };

    auto adoptKnownFrame = [&](int pos) {
        m_anchorPos = pos;
        rememberFrame(frame);
        m_lastOffset = offset;
        return StitchResult{StitchStatus::NoProgress, 0, StitchEdge::None, pos, fh};
    };

    // An unreliable match: the offset can't be trusted, so don't stitch or move
    // the edge. If the frame already exists inside the accumulated image, use that
    // known position as a safe re-anchor; otherwise keep the previous anchor.
    if (confidence > m_config.acceptDiff) {
        int knownPos = -1;
        float knownDiff = kNoMatchConfidence;
        if (H >= fh) {
            const std::pair<int, float> known = findKnownPosition(frameCols, predictedPos);
            knownPos = known.first;
            knownDiff = known.second;
            if (known.second <= m_config.acceptDiff) {
                logStitchDebug("adopt-known-after-reject alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                               "anchor=%d pred=%d known=%d known_diff=%.3f H=%d fh=%d",
                               algorithmDebugName(m_config.algorithm), axisDebugName(m_axis),
                               offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                               known.first, known.second, H, fh);
                return adoptKnownFrame(known.first);
            }
        }
        logStitchDebug("reject-confidence alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                       "anchor=%d pred=%d known=%d known_diff=%.3f H=%d fh=%d",
                       algorithmDebugName(m_config.algorithm), axisDebugName(m_axis),
                       offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                       knownPos, knownDiff, H, fh);
        return StitchResult{StitchStatus::NoMatch, 0};
    }

    // Absolute position of this frame's top edge within the long image. New
    // content exists only where the frame overhangs [0, H). Before welding any
    // overhang, scan the known long image; if the current frame can be placed
    // fully inside it, it is a back-scroll/re-scroll and must not be appended.
    int newPos = predictedPos;
    StitchEdge edge = StitchEdge::None;
    int amount = overhang(newPos, &edge);
    int knownPos = -1;
    float knownDiff = kNoMatchConfidence;
    bool usedKnown = false;
    if (amount > 0 && H >= fh) {
        const std::pair<int, float> known = findKnownPosition(frameCols, predictedPos);
        knownPos = known.first;
        knownDiff = known.second;
        if (known.second <= m_config.acceptDiff) {
            newPos = known.first;
            amount = overhang(newPos, &edge);
            usedKnown = true;
        }
    }

    if (amount >= m_config.minAppend) {
        int overlapLen = 0;
        const float edgeDiff = knownOverlapDiff(frameCols, newPos, &overlapLen);
        if (edgeDiff > m_config.acceptDiff || overlapLen < m_config.minOverlap) {
            logStitchDebug("reject-edge-overlap alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                           "anchor=%d pred=%d pos=%d amount=%d edge=%s edge_diff=%.3f overlap=%d "
                           "known=%d known_diff=%.3f used_known=%d H=%d fh=%d",
                           algorithmDebugName(m_config.algorithm), axisDebugName(m_axis),
                           offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                           newPos, amount, edgeDebugName(edge), edgeDiff, overlapLen, knownPos,
                           knownDiff, usedKnown ? 1 : 0, H, fh);
            return StitchResult{StitchStatus::NoMatch, 0};
        }

        if (edge == StitchEdge::End) {
            appendSlice(frame, amount);   // the bottom `amount` rows are new content
            m_anchorPos = newPos;          // an append leaves the top origin unchanged
        } else {
            prependSlice(frame, amount);  // the top `amount` rows are new content
            m_anchorPos = newPos + amount; // the prepend shifts the old origin down
        }
        m_axisLocked = true;               // orientation fixed once the image grew
        rememberFrame(frame);
        m_lastOffset = offset;
        m_stats.frameCount += 1;
        m_stats.totalHeight = m_full.height();
        m_stats.lastAppend = amount;
        logStitchDebug("append alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                       "old_anchor=%d pred=%d pos=%d amount=%d edge=%s edge_diff=%.3f overlap=%d "
                       "known=%d known_diff=%.3f used_known=%d H=%d new_H=%d fh=%d frames=%d",
                       algorithmDebugName(m_config.algorithm), axisDebugName(m_axis),
                       offset, confidence, m_config.acceptDiff, predictedPos - offset, predictedPos,
                       m_anchorPos, amount, edgeDebugName(edge), edgeDiff, overlapLen, knownPos,
                       knownDiff, usedKnown ? 1 : 0, H, m_full.height(), fh, m_stats.frameCount);
        return StitchResult{StitchStatus::Appended, amount, edge, m_anchorPos, fh};
    }

    if (amount == 0) {
        // Fully inside the long image: back-scroll or re-scroll over seen content.
        // Track the position and re-anchor so the next match basis stays local;
        // never re-stitch. This is what stops dirty-data duplication.
        logStitchDebug("inside-known alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                       "anchor=%d pred=%d pos=%d known=%d known_diff=%.3f used_known=%d H=%d fh=%d",
                       algorithmDebugName(m_config.algorithm), axisDebugName(m_axis),
                       offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos, newPos,
                       knownPos, knownDiff, usedKnown ? 1 : 0, H, fh);
        return adoptKnownFrame(newPos);
    }
    // else: a sub-minAppend sliver is forming past an edge. Hold both the anchor
    // frame and the position so it accumulates and stitches in one clean piece
    // once movement clears minAppend, instead of welding 1-2px jitter into a seam.
    logStitchDebug("wait-min-append alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                   "anchor=%d pred=%d pos=%d amount=%d edge=%s min_append=%d "
                   "known=%d known_diff=%.3f used_known=%d H=%d fh=%d",
                   algorithmDebugName(m_config.algorithm), axisDebugName(m_axis),
                   offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos, newPos,
                   amount, edgeDebugName(edge), m_config.minAppend, knownPos, knownDiff,
                   usedKnown ? 1 : 0, H, fh);
    return StitchResult{StitchStatus::NoProgress, 0};
}

QImage Stitcher::fullImage() const
{
    if (m_axis == ScrollAxis::Horizontal) {
        return transposeImage(m_full);
    }
    return m_full;
}

StitchStats Stitcher::stats() const
{
    return m_stats;
}

StitchAlgorithm Stitcher::algorithm() const
{
    return m_config.algorithm;
}

void Stitcher::setAlgorithm(StitchAlgorithm algorithm)
{
    if (m_config.algorithm == algorithm) {
        return;
    }

    // Adopt the new algorithm's parameter preset but keep the accumulated image
    // and anchor frame, so switching mid-session is seamless. Recompute the
    // col-sample anchor if we're switching into col-sample and have an anchor.
    m_config = defaultConfigFor(algorithm);
    if (algorithm == StitchAlgorithm::ColSample && !m_lastFrame.isNull()) {
        m_lastCols = computeCols(m_lastFrame);
    }
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

    // The seed state is stored in the pipeline's (vertical) space, which is the
    // transpose of the captured frame when horizontal. Flipping the axis flips
    // that mapping, so re-transpose whatever seed we hold to match the new space.
    if (!m_full.isNull()) {
        m_full = transposeImage(m_full);
        m_fullCols = computeCols(m_full);
        m_stats.totalHeight = m_full.height();
        m_stats.lastAppend = m_full.height();
    }
    if (!m_lastFrame.isNull()) {
        m_lastFrame = transposeImage(m_lastFrame);
        if (m_config.algorithm == StitchAlgorithm::ColSample) {
            m_lastCols = computeCols(m_lastFrame);
        }
    }
}

}  // namespace markshot::scroll
