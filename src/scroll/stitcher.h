#pragma once

#include "scroll/stitcher_fixed_regions.h"
#include "scroll/stitcher_frame_profile.h"
#include "scroll/stitcher_long_image.h"

#include <QImage>

#include <utility>

// Scrolling screenshot stitcher. Accumulates frames captured while the user
// scrolls a region and splices them into a single tall image by detecting the
// vertical overlap between consecutive frames.
namespace markshot::scroll {

// Confidence sentinel for "no usable match" (lower confidence is better, so a
// large value means reject). Mirrors f32::MAX in the Rust source.
inline constexpr float kNoMatchConfidence = 1.0e9f;

// The direction the user scrolls (and the long image grows). Vertical grows
// top-to-bottom; Horizontal grows left-to-right. Horizontal is implemented by
// transposing each frame so the whole vertical pipeline runs unchanged.
enum class ScrollAxis {
    Vertical,
    Horizontal,
};

struct StitchConfig {
    int minOverlap;   // Minimum overlapping rows/columns required to trust a match.
    float acceptDiff; // Maximum mean absolute color difference accepted as the same content.
    int minAppend;    // Minimum newly exposed length before mutating the full image.
    float approxDiff; // Early-exit threshold for the adjacent-frame coarse search.
};

// Production thresholds tuned for real scrolling pages: enough overlap to avoid
// repeating sticky chrome, but low enough latency for a live capture loop.
StitchConfig defaultConfig();

enum class StitchStatus {
    FirstFrame, // The seed frame initialized the stitcher.
    Appended,   // New content was appended/prepended to the full image.
    NoProgress, // The frame matched already-known content or is below minAppend.
    NoMatch,    // No trusted overlap was found.
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
    int added = 0;          // Newly committed length along the scroll axis.
    StitchEdge edge = StitchEdge::None;
    int position = 0;     // current frame top/left in stitched-image coordinates
    int frameLength = 0;  // current frame extent along the scroll axis
};

struct StitchStats {
    int frameCount = 0;  // Number of frames that changed the stitcher state.
    int totalHeight = 0; // Long-image length in the vertical pipeline space.
    int lastAppend = 0;  // Last committed delta; equals width for horizontal output.
};

class Stitcher {
public:
    explicit Stitcher(StitchConfig config);

    // Feeds one freshly captured frame and returns how it was incorporated.
    StitchResult pushFrame(const QImage &frame);

    // The accumulated tall image (ARGB32_Premultiplied), or a null image before
    // the first frame. Cached per long-image revision, including the transposed
    // variant for horizontal sessions.
    QImage fullImage() const;

    StitchStats stats() const;

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
    // Pixel-row fallback match used when column samples are ambiguous near an
    // edge, typically because sticky chrome was captured in both frames. The
    // trim is the stale fixed band to cut from the matching edge of the long
    // image: bottom rows for End matches, top rows for Start matches.
    struct EdgeLineMatch {
        int position = 0;
        float diff = kNoMatchConfidence;
        int trim = 0;
        int matchedRows = 0;
        StitchEdge edge = StitchEdge::None;
    };

    // Finds the current frame offset relative to the previous captured frame.
    std::pair<int, float> findOffsetColSample(const ColSamples &frameCols) const;
    // Scores a frame placed at an absolute position inside the full image. The
    // exclude arguments drop the long image's stale fixed bands (about to be
    // trimmed by the pending append/prepend) from the comparison.
    float knownOverlapDiff(const ColSamples &frameCols, int framePos, int *overlapLen = nullptr,
                           int fullTopExclude = 0, int fullBottomExclude = 0) const;
    // Recovers from a bad adjacent-frame match by searching already-stitched content.
    std::pair<int, float> findKnownPosition(const ColSamples &frameCols, int predictedPos) const;
    // Searches placements that reveal new content past either edge of the full image.
    std::pair<int, float> findEdgePosition(const ColSamples &frameCols, int predictedPos) const;
    // Exact row-run fallback for edge matches with repeated bottom chrome.
    EdgeLineMatch findLineRunPosition(const QImage &frame, int predictedPos) const;
    // Seeds the long image from the first accepted frame.
    StitchResult initializeSeed(const QImage &frame, const ColSamples &frameCols);
    void rememberFrame(const QImage &frame, const ColSamples &frameCols);

    StitchConfig m_config;
    ScrollAxis m_axis = ScrollAxis::Vertical;
    bool m_axisLocked = false;  // true once the first directional stitch lands
    LongImage m_long;
    QImage m_lastFrame;
    ColSamples m_lastCols;
    FixedRegionDetector m_fixedRegions;
    int m_lastOffset = 0;
    int m_bestBottomTrim = 0;
    int m_bestTopTrim = 0;
    StitchEdge m_pendingEdge = StitchEdge::None;
    StitchEdge m_growthEdge = StitchEdge::None;
    // Absolute position of the anchor frame's top edge within the long image,
    // along the scroll axis. The frame the next match is measured against sits at
    // [m_anchorPos, m_anchorPos + frameLen) in long-image coordinates. Scrolling
    // back over already-stitched content only moves this position; new content is
    // appended/prepended only when the frame sees past an end of the long image.
    int m_anchorPos = 0;
    StitchStats m_stats;
    mutable QImage m_fullCache;
    mutable quint64 m_fullCacheRevision = 0;
    mutable bool m_fullCacheValid = false;
};

}  // namespace markshot::scroll
