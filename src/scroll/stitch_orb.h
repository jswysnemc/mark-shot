#pragma once

#include <QImage>

#include <optional>
#include <utility>

// ORB-based vertical offset estimation, ported from wayscrollshot's
// estimate_orb_offset plus its NCC template fallback. The whole body is guarded
// by HAVE_OPENCV so the translation unit compiles to nothing when OpenCV is
// absent; callers must also guard with HAVE_OPENCV.
namespace markshot::scroll {

#ifdef HAVE_OPENCV

struct OrbEstimate {
    double dy = 0.0;
    float confidence = 0.0f;
};

// Estimates the vertical scroll offset between two equally sized frames using
// ORB feature matching + RANSAC affine. Returns nullopt when the match is not
// trustworthy, leaving the caller to fall back.
std::optional<OrbEstimate> estimateOrbOffset(const QImage &prev, const QImage &frame, int minOverlap);

// Normalized cross-correlation template fallback over the content ROI. Returns
// (offset, confidence) or nullopt.
std::optional<std::pair<int, float>> templateFallback(const QImage &prev,
                                                       const QImage &frame,
                                                       int lastOffset,
                                                       int minOverlap);

#endif  // HAVE_OPENCV

}  // namespace markshot::scroll
