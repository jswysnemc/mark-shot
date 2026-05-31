#include "scroll/stitcher.h"

#include "scroll/stitch_orb.h"

#include <QPainter>

#include <algorithm>
#include <array>
#include <cmath>
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
// [p, p+1, p-1, p+2, p-2, ...], clamped to [0, max]. Mirrors
// wayscrollshot's predict_offset_iter.
std::vector<int> predictOffsetIter(int max, int predict)
{
    const int p = std::clamp(predict, 0, std::max(0, max));
    std::vector<int> result;
    result.reserve(static_cast<std::size_t>(std::max(0, max)) + 1);
    result.push_back(p);
    for (int delta = 1; delta <= max; ++delta) {
        if (p + delta <= max) {
            result.push_back(p + delta);
        }
        if (p - delta >= 0) {
            result.push_back(p - delta);
        }
    }
    return result;
}

}  // namespace

StitchConfig defaultConfigFor(StitchAlgorithm algorithm)
{
    if (algorithm == StitchAlgorithm::OpenCvOrb) {
        return StitchConfig{120, 3.5f, 10, 1.0f, StitchAlgorithm::OpenCvOrb};
    }
    return StitchConfig{100, 5.0f, 15, 1.0f, StitchAlgorithm::ColSample};
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

void Stitcher::appendSlice(const QImage &frame, int newHeight)
{
    const int w = m_full.width();
    QImage combined(w, m_full.height() + newHeight, QImage::Format_ARGB32_Premultiplied);
    combined.fill(Qt::transparent);

    QPainter painter(&combined);
    painter.drawImage(0, 0, m_full);
    const int overlap = frame.height() - newHeight;
    const QImage slice = frame.copy(0, overlap, frame.width(), newHeight);
    painter.drawImage(0, m_full.height(), slice);
    painter.end();

    m_full = combined;
}

void Stitcher::rememberFrame(const QImage &frame)
{
    if (m_config.algorithm == StitchAlgorithm::ColSample) {
        m_lastCols = computeCols(frame);
    }
    m_lastFrame = frame;
}

StitchResult Stitcher::pushFrame(const QImage &frame)
{
    if (frame.isNull() || frame.width() <= 0 || frame.height() <= 0) {
        return StitchResult{StitchStatus::NoMatch, 0};
    }

    if (m_full.isNull()) {
        m_full = frame.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        m_stats.frameCount = 1;
        m_stats.totalHeight = m_full.height();
        m_stats.lastAppend = m_full.height();
        rememberFrame(frame);
        return StitchResult{StitchStatus::FirstFrame, m_full.height()};
    }

    // Stitching assumes equal-width frames; drop a frame whose width drifted.
    if (frame.width() != m_full.width()) {
        return StitchResult{StitchStatus::NoMatch, 0};
    }

    const bool useOrb = m_config.algorithm == StitchAlgorithm::OpenCvOrb && openCvAvailable();
    const std::pair<int, float> match = useOrb ? findOffsetOrb(frame) : findOffsetColSample(frame);
    const int offset = match.first;
    const float confidence = match.second;

    // ORB keeps its anchor frame across rejected frames (preserve_anchor);
    // col-sample advances its anchor every frame.
    const bool preserveAnchor = useOrb;

    if (confidence > m_config.acceptDiff) {
        if (!preserveAnchor) {
            rememberFrame(frame);
        }
        return StitchResult{StitchStatus::NoMatch, 0};
    }

    const int newHeight = offset > 0 ? offset : 0;
    if (newHeight < m_config.minAppend) {
        if (!preserveAnchor) {
            rememberFrame(frame);
            m_lastOffset = offset;
        }
        return StitchResult{StitchStatus::NoProgress, 0};
    }

    appendSlice(frame, newHeight);
    rememberFrame(frame);
    m_lastOffset = offset;
    m_stats.frameCount += 1;
    m_stats.totalHeight = m_full.height();
    m_stats.lastAppend = newHeight;
    return StitchResult{StitchStatus::Appended, newHeight};
}

QImage Stitcher::fullImage() const
{
    return m_full;
}

StitchStats Stitcher::stats() const
{
    return m_stats;
}

}  // namespace markshot::scroll
