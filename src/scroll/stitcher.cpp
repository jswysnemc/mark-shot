#include "scroll/stitcher.h"

#include "debug_log.h"

#include <QPainter>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdarg>
#include <limits>
#include <vector>

namespace markshot::scroll {

namespace {

constexpr float kColTopIgnoreRatio = 0.10f;
constexpr float kColBottomIgnoreRatio = 0.08f;
constexpr int kColMinIgnorePx = 16;
constexpr int kColMaxBandSamples = 17;
constexpr float kLineRowMaxDiff = 2.0f;
constexpr int kLineMaxSampleCols = 256;

int scaledIgnore(int height, float ratio)
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

float rowMeanAbsDiff(const QImage &a, int ay, const QImage &b, int by, int startX, int width)
{
    if (ay < 0 || ay >= a.height() || by < 0 || by >= b.height() || width <= 0) {
        return kNoMatchConfidence;
    }

    const QRgb *aLine = reinterpret_cast<const QRgb *>(a.constScanLine(ay));
    const QRgb *bLine = reinterpret_cast<const QRgb *>(b.constScanLine(by));
    const int step = std::max(1, width / kLineMaxSampleCols);
    float sum = 0.0f;
    int count = 0;
    for (int x = startX; x < startX + width; x += step) {
        const QRgb ap = aLine[x];
        const QRgb bp = bLine[x];
        sum += std::abs(qRed(ap) - qRed(bp));
        sum += std::abs(qGreen(ap) - qGreen(bp));
        sum += std::abs(qBlue(ap) - qBlue(bp));
        count += 3;
    }
    return count > 0 ? sum / static_cast<float>(count) : kNoMatchConfidence;
}

// Mean absolute difference of the overlapping region when cols2 (current frame)
// is shifted down by `offset` relative to cols1 (previous frame). Low-value top
// and bottom rows are ignored only when enough overlap remains.
float computeColDiff(const QVector<std::array<float, 3>> &cols1,
                     const QVector<std::array<float, 3>> &cols2,
                     int offset,
                     int minOverlap)
{
    const int h1 = static_cast<int>(cols1.size());
    const int h2 = static_cast<int>(cols2.size());
    if (h1 == 0 || h2 == 0) {
        return kNoMatchConfidence;
    }

    const int overlapLen = offset >= 0
        ? std::min(h1 - offset, h2)
        : std::min(h1, h2 + offset);
    if (overlapLen < minOverlap) {
        return kNoMatchConfidence;
    }

    const bool cropRows = shouldCropContentRows(overlapLen, std::min(h1, h2), minOverlap);
    float sum = 0.0f;
    int count = 0;
    int rows = 0;

    if (offset >= 0) {
        const int len = std::min(h1 - offset, h2);
        for (int i = 0; i < len; ++i) {
            const int y1 = offset + i;
            const int y2 = i;
            if (cropRows && (!isContentRow(y1, h1) || !isContentRow(y2, h2))) {
                continue;
            }
            ++rows;
            for (int g = 0; g < 3; ++g) {
                sum += std::abs(cols1[y1][g] - cols2[y2][g]);
                ++count;
            }
        }
    } else {
        const int o = -offset;
        const int len = std::min(h1, h2 - o);
        for (int i = 0; i < len; ++i) {
            const int y1 = i;
            const int y2 = o + i;
            if (cropRows && (!isContentRow(y1, h1) || !isContentRow(y2, h2))) {
                continue;
            }
            ++rows;
            for (int g = 0; g < 3; ++g) {
                sum += std::abs(cols1[y1][g] - cols2[y2][g]);
                ++count;
            }
        }
    }

    if (count == 0 || rows < requiredComparedRows(minOverlap, cropRows)) {
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

}  // namespace

StitchConfig defaultConfig()
{
    return StitchConfig{100, 9.0f, 15, 1.0f};
}

Stitcher::Stitcher(StitchConfig config) : m_config(config) {}

Stitcher::ColSamples Stitcher::computeCols(const QImage &frame) const
{
    const int w = frame.width();
    const int h = frame.height();
    if (w <= 0 || h <= 0) {
        return {};
    }

    const QImage rgb = frame.convertToFormat(QImage::Format_RGB32);
    const std::array<std::pair<int, int>, 3> bands = {
        bandRange(w, 0.08f, 0.32f),
        bandRange(w, 0.34f, 0.66f),
        bandRange(w, 0.68f, 0.92f),
    };

    ColSamples result(h);
    for (int g = 0; g < 3; ++g) {
        const int start = bands[g].first;
        const int end = bands[g].second;
        const int sampleCount = std::max(1, std::min(kColMaxBandSamples, end - start + 1));
        const float step = sampleCount > 1
            ? static_cast<float>(end - start) / static_cast<float>(sampleCount - 1)
            : 0.0f;
        for (int y = 0; y < h; ++y) {
            const QRgb *line = reinterpret_cast<const QRgb *>(rgb.scanLine(y));
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
        const float diff = computeColDiff(m_lastCols, cols, offset, m_config.minOverlap);
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
    const bool cropRows = shouldCropContentRows(len, frameH, m_config.minOverlap);
    float sum = 0.0f;
    int count = 0;
    int rows = 0;
    for (int row = 0; row < len; ++row) {
        const int frameY = frameStart + row;
        if (cropRows && !isContentRow(frameY, frameH)) {
            continue;
        }
        const std::array<float, 3> &fullPx = m_fullCols[fullStart + row];
        const std::array<float, 3> &framePx = frameCols[frameY];
        ++rows;
        for (int g = 0; g < 3; ++g) {
            sum += std::abs(fullPx[g] - framePx[g]);
            ++count;
        }
    }
    if (count == 0 || rows < requiredComparedRows(m_config.minOverlap, cropRows)) {
        return kNoMatchConfidence;
    }
    return sum / static_cast<float>(count);
}

std::pair<int, float> Stitcher::findKnownPosition(const ColSamples &frameCols, int predictedPos) const
{
    const int fullH = m_fullCols.size();
    const int frameH = frameCols.size();
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
    const int fullH = m_fullCols.size();
    const int frameH = frameCols.size();
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
        if (overhangAmount(pos, frameH, fullH, nullptr) <= 0) {
            return;
        }
        int overlapLen = 0;
        const float diff = knownOverlapDiff(frameCols, pos, &overlapLen);
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
    if (m_full.isNull() || frame.isNull()
        || m_full.width() != frame.width()
        || m_full.height() <= 0
        || frame.height() <= 0) {
        return best;
    }

    const QImage current = frame.format() == QImage::Format_ARGB32_Premultiplied
        ? frame
        : frame.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const int fullH = m_full.height();
    const int frameH = current.height();
    const int side = sideIgnoreWidth(current.width());
    const int roiW = current.width() - side * 2;
    if (roiW <= 0) {
        return best;
    }

    const int maxTrim = std::min({fullH - 1, frameH / 3, fullH / 3});
    std::vector<int> trimCandidates{0};
    if (m_bestBottomTrim > 0) {
        trimCandidates.push_back(std::clamp(m_bestBottomTrim, 0, maxTrim));
    }

    int detectedTrim = 0;
    for (int row = 0; row < maxTrim; ++row) {
        const float diff = rowMeanAbsDiff(m_full, fullH - 1 - row, current, frameH - 1 - row, side, roiW);
        if (diff > kLineRowMaxDiff) {
            break;
        }
        ++detectedTrim;
    }
    if (detectedTrim >= std::max(8, m_config.minAppend / 2)) {
        trimCandidates.push_back(detectedTrim);
    }

    std::sort(trimCandidates.begin(), trimCandidates.end());
    trimCandidates.erase(std::unique(trimCandidates.begin(), trimCandidates.end()), trimCandidates.end());

    const int minRows = m_config.minOverlap;
    const int matchLimit = std::max(minRows, frameH / 2);
    for (int trim : trimCandidates) {
        trim = std::clamp(trim, 0, maxTrim);
        const int edgeY = fullH - trim - 1;
        if (edgeY < minRows - 1) {
            continue;
        }

        for (int currentY = frameH - 1; currentY >= 0 && best.matchedRows < matchLimit; --currentY) {
            int rows = 0;
            float diffSum = 0.0f;
            while (rows < matchLimit && edgeY - rows >= 0 && currentY - rows >= 0) {
                const float diff = rowMeanAbsDiff(m_full, edgeY - rows, current, currentY - rows, side, roiW);
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

            const float avgDiff = diffSum / static_cast<float>(rows);
            const int bestDistance = std::abs(best.position - predictedPos);
            const int distance = std::abs(pos - predictedPos);
            if (rows > best.matchedRows
                || (rows == best.matchedRows && avgDiff < best.diff)
                || (rows == best.matchedRows && std::abs(avgDiff - best.diff) < 0.001f
                    && distance < bestDistance)) {
                best.position = pos;
                best.diff = avgDiff;
                best.bottomTrim = trim;
                best.matchedRows = rows;
            }
        }
    }

    return best;
}

void Stitcher::appendSlice(const QImage &frame, int amount, int trimBottom)
{
    const int w = m_full.width();
    trimBottom = std::clamp(trimBottom, 0, std::max(0, m_full.height() - 1));
    const int keepHeight = m_full.height() - trimBottom;
    QImage combined(w, keepHeight + amount, QImage::Format_ARGB32_Premultiplied);
    combined.setDevicePixelRatio(1.0);
    combined.fill(Qt::transparent);

    QPainter painter(&combined);
    painter.drawImage(QRect(0, 0, w, keepHeight),
                      m_full,
                      QRect(0, 0, w, keepHeight));
    const int overlap = frame.height() - amount;
    painter.drawImage(QRect(0, keepHeight, frame.width(), amount),
                      frame,
                      QRect(0, overlap, frame.width(), amount));
    painter.end();

    m_full = combined;
    m_fullCols = computeCols(m_full);
}

void Stitcher::prependSlice(const QImage &frame, int amount)
{
    const int w = m_full.width();
    QImage combined(w, m_full.height() + amount, QImage::Format_ARGB32_Premultiplied);
    combined.setDevicePixelRatio(1.0);
    combined.fill(Qt::transparent);

    QPainter painter(&combined);
    // The top `amount` rows of the current frame are the newly-revealed content
    // above the previous anchor; place them first, then the old image below.
    painter.drawImage(QRect(0, 0, frame.width(), amount),
                      frame,
                      QRect(0, 0, frame.width(), amount));
    painter.drawImage(QRect(0, amount, w, m_full.height()),
                      m_full,
                      QRect(0, 0, w, m_full.height()));
    painter.end();

    m_full = combined;
    m_fullCols = computeCols(m_full);
}

void Stitcher::rememberFrame(const QImage &frame)
{
    m_lastCols = computeCols(frame);
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
    const QImage frame = normalizePixelImage(
        m_axis == ScrollAxis::Horizontal ? transposeImage(rawFrame) : rawFrame);

    if (m_full.isNull()) {
        m_full = frame;
        m_fullCols = computeCols(m_full);
        m_anchorPos = 0;
        m_bestBottomTrim = 0;
        m_pendingEdge = StitchEdge::None;
        m_growthEdge = StitchEdge::None;
        m_stats.frameCount = 1;
        m_stats.totalHeight = m_full.height();
        m_stats.lastAppend = m_full.height();
        rememberFrame(frame);
        logStitchDebug("first-frame alg=%s axis=%s frame=%dx%d full=%dx%d",
                       algorithmDebugName(), axisDebugName(m_axis),
                       frame.width(), frame.height(), m_full.width(), m_full.height());
        return StitchResult{StitchStatus::FirstFrame, m_full.height(), StitchEdge::None, 0, m_full.height()};
    }

    // Stitching assumes equal-width frames; drop a frame whose width drifted.
    if (frame.width() != m_full.width()) {
        logStitchDebug("drop width-mismatch alg=%s axis=%s frame_w=%d full_w=%d frame_h=%d full_h=%d",
                       algorithmDebugName(), axisDebugName(m_axis),
                       frame.width(), m_full.width(), frame.height(), m_full.height());
        return StitchResult{StitchStatus::NoMatch, 0};
    }

    const std::pair<int, float> match = findOffsetColSample(frame);
    const int offset = match.first;
    const float confidence = match.second;

    const int fh = frame.height();
    const int H = m_full.height();
    const ColSamples frameCols = computeCols(frame);
    const int predictedPos = m_anchorPos + offset;
    auto overhang = [&](int pos, int trimBottom, StitchEdge *edge) {
        return overhangAmount(pos, fh, H - trimBottom, edge);
    };

    auto adoptKnownFrame = [&](int pos, int appliedOffset) {
        m_anchorPos = pos;
        rememberFrame(frame);
        m_lastOffset = appliedOffset;
        m_pendingEdge = StitchEdge::None;
        return StitchResult{StitchStatus::NoProgress, 0, StitchEdge::None, pos, fh};
    };

    auto holdPendingFrame = [&](int pos, int appliedOffset, StitchEdge pendingEdge) {
        Q_UNUSED(appliedOffset);
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
    int bottomTrim = 0;
    int lineMatchedRows = 0;

    const bool anchorNearEdge = m_anchorPos <= m_config.minAppend
        || m_anchorPos + fh >= H - m_config.minAppend;
    auto tryEdgeRecovery = [&]() {
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
            bottomTrim = linePlacement.bottomTrim;
            lineMatchedRows = linePlacement.matchedRows;
            usedEdgeRecovery = true;
            usedLineRecovery = true;
            return true;
        }
        newPos = edgePlacement.first;
        appliedOffset = newPos - m_anchorPos;
        effectiveConfidence = edgePlacement.second;
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
        if (usedLineRecovery && edge == StitchEdge::End) {
            overlapLen = lineMatchedRows;
            edgeDiff = effectiveConfidence;
        } else {
            edgeDiff = knownOverlapDiff(frameCols, newPos, &overlapLen);
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

    if (amount > 0 && (edgeDiff > m_config.acceptDiff || overlapLen < m_config.minOverlap)) {
        const int rejectedPos = newPos;
        const float rejectedDiff = edgeDiff;
        const int rejectedOverlap = overlapLen;
        if (!usedEdgeRecovery && confidence > m_config.acceptDiff && tryEdgeRecovery()) {
            refreshEdgeOverlap();
            logStitchDebug("recover-edge-after-overlap-reject alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                           "anchor=%d pred=%d rejected_pos=%d rejected_diff=%.3f rejected_overlap=%d "
                           "edge_pos=%d edge_diff=%.3f edge_overlap=%d bottom_trim=%d line_rows=%d H=%d fh=%d",
                           algorithmDebugName(), axisDebugName(m_axis),
                           offset, confidence, m_config.acceptDiff, m_anchorPos, predictedPos,
                           rejectedPos, rejectedDiff, rejectedOverlap, edgeRecoveryPos,
                           edgeRecoveryDiff, overlapLen, bottomTrim, lineMatchedRows, H, fh);
        }
    }

    if (amount > 0 && H >= fh) {
        if (!trustedOppositeEdgeSwitch
            && (edgeDiff > m_config.acceptDiff || overlapLen < m_config.minOverlap)) {
            const std::pair<int, float> known = findKnownPosition(frameCols, predictedPos);
            knownPos = known.first;
            knownDiff = known.second;
            if (known.second <= m_config.acceptDiff) {
                newPos = known.first;
                bottomTrim = 0;
                lineMatchedRows = 0;
                usedLineRecovery = false;
                amount = overhang(newPos, bottomTrim, &edge);
                edgeDiff = 0.0f;
                overlapLen = fh;
                usedKnown = true;
            }
        }
    }

    const int heightDelta = edge == StitchEdge::End ? amount - bottomTrim : amount;
    if (heightDelta >= m_config.minAppend) {
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
            appendSlice(frame, amount, bottomTrim); // the bottom rows are new content after optional edge trim
            m_anchorPos = newPos;          // an append leaves the top origin unchanged
            if (bottomTrim > 0) {
                m_bestBottomTrim = std::max(m_bestBottomTrim, bottomTrim);
            }
        } else {
            prependSlice(frame, amount);  // the top `amount` rows are new content
            m_anchorPos = newPos + amount; // the prepend shifts the old origin down
        }
        m_axisLocked = true;               // orientation fixed once the image grew
        m_pendingEdge = StitchEdge::None;
        if (edge != StitchEdge::None) {
            m_growthEdge = edge;
        }
        rememberFrame(frame);
        m_lastOffset = appliedOffset;
        m_stats.frameCount += 1;
        m_stats.totalHeight = m_full.height();
        m_stats.lastAppend = heightDelta;
        logStitchDebug("append alg=%s axis=%s off=%d conf=%.3f accept=%.3f "
                       "old_anchor=%d pred=%d pos=%d amount=%d edge=%s edge_diff=%.3f overlap=%d "
                       "known=%d known_diff=%.3f used_known=%d edge_recovery=%d line_recovery=%d edge_pos=%d "
                       "edge_recovery_diff=%.3f bottom_trim=%d line_rows=%d height_delta=%d H=%d new_H=%d fh=%d frames=%d",
                       algorithmDebugName(), axisDebugName(m_axis),
                       offset, effectiveConfidence, m_config.acceptDiff, predictedPos - offset, predictedPos,
                       m_anchorPos, amount, edgeDebugName(edge), edgeDiff, overlapLen, knownPos,
                       knownDiff, usedKnown ? 1 : 0, usedEdgeRecovery ? 1 : 0,
                       usedLineRecovery ? 1 : 0, edgeRecoveryPos, edgeRecoveryDiff,
                       bottomTrim, lineMatchedRows, heightDelta, H, m_full.height(), fh, m_stats.frameCount);
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
    return holdPendingFrame(newPos, appliedOffset, edge);
}

QImage Stitcher::fullImage() const
{
    QImage result = m_axis == ScrollAxis::Horizontal ? transposeImage(m_full) : m_full;
    result.setDevicePixelRatio(1.0);
    return result;
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

    // The seed state is stored in the pipeline's (vertical) space, which is the
    // transpose of the captured frame when horizontal. Flipping the axis flips
    // that mapping, so re-transpose whatever seed we hold to match the new space.
    if (!m_full.isNull()) {
        m_full = transposeImage(m_full);
        m_fullCols = computeCols(m_full);
        m_bestBottomTrim = 0;
        m_stats.totalHeight = m_full.height();
        m_stats.lastAppend = m_full.height();
    }
    if (!m_lastFrame.isNull()) {
        m_lastFrame = transposeImage(m_lastFrame);
        m_lastCols = computeCols(m_lastFrame);
    }
}

}  // namespace markshot::scroll
