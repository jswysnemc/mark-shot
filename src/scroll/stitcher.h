#pragma once

#include <QImage>
#include <QVector>

#include <array>
#include <utility>

// Scrolling screenshot stitcher. Accumulates frames captured while the user
// scrolls a region and splices them into a single tall image by detecting the
// vertical overlap between consecutive frames. Ported from the Rust project
// wayscrollshot (src/stitch.rs): the col-sample algorithm is a dependency-free
// column-average MAD search; the OpenCV ORB path lives in stitch_orb.cpp and is
// compiled in only when OpenCV is available.
namespace markshot::scroll {

// Confidence sentinel for "no usable match" (lower confidence is better, so a
// large value means reject). Mirrors f32::MAX in the Rust source.
inline constexpr float kNoMatchConfidence = 1.0e9f;

enum class StitchAlgorithm {
    ColSample,
    OpenCvOrb,
};

struct StitchConfig {
    int minOverlap;
    float acceptDiff;
    int minAppend;
    float approxDiff;
    StitchAlgorithm algorithm;
};

// Parameter presets matching wayscrollshot's two configurations.
StitchConfig defaultConfigFor(StitchAlgorithm algorithm);

// Whether the ORB path was compiled in. When false, requesting OpenCvOrb still
// works but silently behaves like ColSample.
bool openCvAvailable();

enum class StitchStatus {
    FirstFrame,
    Appended,
    NoProgress,
    NoMatch,
};

struct StitchResult {
    StitchStatus status = StitchStatus::NoMatch;
    int added = 0;
};

struct StitchStats {
    int frameCount = 0;
    int totalHeight = 0;
    int lastAppend = 0;
};

class Stitcher {
public:
    explicit Stitcher(StitchConfig config);

    // Feeds one freshly captured frame and returns how it was incorporated.
    StitchResult pushFrame(const QImage &frame);

    // The accumulated tall image (ARGB32_Premultiplied), or a null image before
    // the first frame.
    QImage fullImage() const;

    StitchStats stats() const;

private:
    using ColSamples = QVector<std::array<float, 3>>;

    ColSamples computeCols(const QImage &frame) const;
    std::pair<int, float> findOffsetColSample(const QImage &frame) const;
    std::pair<int, float> findOffsetOrb(const QImage &frame) const;
    void appendSlice(const QImage &frame, int newHeight);
    void rememberFrame(const QImage &frame);

    StitchConfig m_config;
    QImage m_full;
    QImage m_lastFrame;
    ColSamples m_lastCols;
    int m_lastOffset = 0;
    StitchStats m_stats;
};

}  // namespace markshot::scroll
