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

// The direction the user scrolls (and the long image grows). Vertical grows
// top-to-bottom; Horizontal grows left-to-right. Horizontal is implemented by
// transposing each frame so the whole vertical pipeline runs unchanged.
enum class ScrollAxis {
    Vertical,
    Horizontal,
};

struct StitchConfig {
    int minOverlap;
    float acceptDiff;
    int minAppend;
    float approxDiff;
    StitchAlgorithm algorithm;
};

// Parameter presets for the available matching algorithms.
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

// Which edge of the long image grew on an Appended result. End is the
// bottom/right (forward scroll); Start is the top/left (reverse scroll, which
// prepends). The scrubber uses this to keep the viewed position anchored when
// content is inserted ahead of it.
enum class StitchEdge {
    None,
    Start,
    End,
};

struct StitchResult {
    StitchStatus status = StitchStatus::NoMatch;
    int added = 0;
    StitchEdge edge = StitchEdge::None;
    int position = 0;     // current frame top/left in stitched-image coordinates
    int frameLength = 0;  // current frame extent along the scroll axis
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

    // Switches the matching algorithm mid-session, adopting that algorithm's
    // parameter preset while keeping the accumulated image and anchor state.
    void setAlgorithm(StitchAlgorithm algorithm);
    StitchAlgorithm algorithm() const;

    // The scroll axis. Horizontal capture is implemented by transposing every
    // frame so the whole vertical pipeline runs unchanged, then transposing the
    // result back in fullImage(). The axis can still be switched while only the
    // seed frame exists (the seed is re-transposed to match); it locks once the
    // first directional stitch lands, since the accumulated image then has a
    // fixed orientation. The UI offers the toggle while axisLocked() is false.
    void setAxis(ScrollAxis axis);
    ScrollAxis axis() const;
    bool axisLocked() const;

private:
    using ColSamples = QVector<std::array<float, 3>>;

    ColSamples computeCols(const QImage &frame) const;
    std::pair<int, float> findOffsetColSample(const QImage &frame) const;
    std::pair<int, float> findOffsetOrb(const QImage &frame) const;
    float knownOverlapDiff(const ColSamples &frameCols, int framePos, int *overlapLen = nullptr) const;
    std::pair<int, float> findKnownPosition(const ColSamples &frameCols, int predictedPos) const;
    // Appends the bottom `amount` rows of `frame` below the accumulated image
    // (forward scroll); prependSlice puts the top `amount` rows above it
    // (reverse scroll).
    void appendSlice(const QImage &frame, int amount);
    void prependSlice(const QImage &frame, int amount);
    void rememberFrame(const QImage &frame);

    StitchConfig m_config;
    ScrollAxis m_axis = ScrollAxis::Vertical;
    bool m_axisLocked = false;  // true once the first directional stitch lands
    QImage m_full;
    QImage m_lastFrame;
    ColSamples m_fullCols;
    ColSamples m_lastCols;
    int m_lastOffset = 0;
    // Absolute position of the anchor frame's top edge within the long image,
    // along the scroll axis. The frame the next match is measured against sits at
    // [m_anchorPos, m_anchorPos + frameLen) in long-image coordinates. Scrolling
    // back over already-stitched content only moves this position; new content is
    // appended/prepended only when the frame sees past an end of the long image.
    int m_anchorPos = 0;
    StitchStats m_stats;
};

}  // namespace markshot::scroll
