#include "scroll/stitch_orb.h"

#ifdef HAVE_OPENCV

#include <opencv2/calib3d.hpp>
#include <opencv2/core.hpp>
#include <opencv2/features2d.hpp>
#include <opencv2/imgproc.hpp>

#include <algorithm>
#include <cmath>
#include <limits>
#include <vector>

// ORB-based vertical offset estimation, ported from wayscrollshot
// (src/stitch.rs: estimate_orb_offset + find_offset_template_content). Constants
// and thresholds are copied verbatim so the C++ path behaves like the Rust one.
namespace markshot::scroll {

namespace {

constexpr int kOrbMaxFeatures = 1500;
constexpr int kOrbMinKeypoints = 80;
constexpr int kOrbMinMatches = 24;
constexpr int kOrbMinInliers = 18;
constexpr double kOrbMaxDx = 12.0;
constexpr double kOrbMaxGeometryDrift = 0.12;
constexpr float kOrbTopIgnoreRatio = 0.12f;
constexpr float kOrbBottomIgnoreRatio = 0.08f;
constexpr float kOrbSideIgnoreRatio = 0.04f;
constexpr int kOrbMinIgnorePx = 24;
constexpr double kLoweRatio = 0.78;

constexpr int kTemplateMinHeight = 48;
constexpr float kTemplateFallbackMinScore = 0.72f;
constexpr float kTemplateFallbackMinMargin = 0.015f;
constexpr float kTemplateVerifyMaxDiff = 18.0f;

// Converts a QImage to a single-channel CV_8UC1 Mat. Qt's Grayscale8 conversion
// already applies the standard luma weights, matching wayscrollshot's
// rgba_to_gray. The Mat is cloned so it owns its memory independent of the
// QImage (whose buffer may have a stride wider than width).
cv::Mat grayToMat(const QImage &image)
{
    const QImage gray = image.convertToFormat(QImage::Format_Grayscale8);
    const cv::Mat view(gray.height(), gray.width(), CV_8UC1,
                       const_cast<uchar *>(gray.bits()),
                       static_cast<std::size_t>(gray.bytesPerLine()));
    return view.clone();
}

// Content region of interest: drop the outer edges so scrollbars, shadows, and
// sticky headers/footers don't anchor the match. Mirrors content_roi.
cv::Rect contentRoi(int width, int height)
{
    const int side = std::max(static_cast<int>(width * kOrbSideIgnoreRatio), kOrbMinIgnorePx);
    const int top = std::max(static_cast<int>(height * kOrbTopIgnoreRatio), kOrbMinIgnorePx);
    const int bottom = std::max(static_cast<int>(height * kOrbBottomIgnoreRatio), kOrbMinIgnorePx);
    const int x = std::min(side, std::max(0, width - 1));
    const int y = std::min(top, std::max(0, height - 1));
    const int w = std::max(1, width - x * 2);
    const int h = std::max(1, height - y - bottom);
    return cv::Rect(x, y, w, h);
}

cv::Mat buildFeatureMask(int width, int height)
{
    cv::Mat mask = cv::Mat::zeros(height, width, CV_8UC1);
    cv::rectangle(mask, contentRoi(width, height), cv::Scalar(255), cv::FILLED);
    return mask;
}

// Per-pixel grayscale (0.299/0.587/0.114) over the whole frame, used by the
// NCC template fallback. Matches to_grayscale_vec.
std::vector<float> toGrayscaleVec(const QImage &image)
{
    const QImage rgb = image.convertToFormat(QImage::Format_RGB32);
    const int width = rgb.width();
    const int height = rgb.height();
    std::vector<float> result(static_cast<std::size_t>(width) * height);
    for (int y = 0; y < height; ++y) {
        const QRgb *line = reinterpret_cast<const QRgb *>(rgb.scanLine(y));
        for (int x = 0; x < width; ++x) {
            const QRgb px = line[x];
            result[static_cast<std::size_t>(y) * width + x] =
                0.299f * qRed(px) + 0.587f * qGreen(px) + 0.114f * qBlue(px);
        }
    }
    return result;
}

// Search order centred on the predicted offset, expanding outward. Identical to
// the col-sample path's predict_offset_iter.
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

// Normalized cross-correlation of a template band against the previous frame at
// a given vertical position, restricted to the content ROI columns. Mirrors
// ncc_score_region.
float nccScoreRegion(const std::vector<float> &imageGray,
                     const std::vector<float> &templateGray,
                     int width,
                     int roiX,
                     int roiW,
                     int imageY,
                     int templateY,
                     int templateH)
{
    if (roiW <= 0 || templateH <= 0 || width <= 0) {
        return std::numeric_limits<float>::lowest();
    }

    float tmplSum = 0.0f;
    float imgSum = 0.0f;
    int count = 0;
    for (int row = 0; row < templateH; ++row) {
        const std::size_t tmplBase = static_cast<std::size_t>(templateY + row) * width + roiX;
        const std::size_t imgBase = static_cast<std::size_t>(imageY + row) * width + roiX;
        for (int col = 0; col < roiW; ++col) {
            tmplSum += templateGray[tmplBase + col];
            imgSum += imageGray[imgBase + col];
            ++count;
        }
    }
    if (count == 0) {
        return std::numeric_limits<float>::lowest();
    }

    const float tmplMean = tmplSum / count;
    const float imgMean = imgSum / count;
    float numerator = 0.0f;
    float tmplVar = 0.0f;
    float imgVar = 0.0f;
    for (int row = 0; row < templateH; ++row) {
        const std::size_t tmplBase = static_cast<std::size_t>(templateY + row) * width + roiX;
        const std::size_t imgBase = static_cast<std::size_t>(imageY + row) * width + roiX;
        for (int col = 0; col < roiW; ++col) {
            const float tmpl = templateGray[tmplBase + col] - tmplMean;
            const float img = imageGray[imgBase + col] - imgMean;
            numerator += tmpl * img;
            tmplVar += tmpl * tmpl;
            imgVar += img * img;
        }
    }
    if (tmplVar <= 1.0f || imgVar <= 1.0f) {
        return std::numeric_limits<float>::lowest();
    }
    return numerator / (std::sqrt(tmplVar) * std::sqrt(imgVar));
}

// Mean absolute difference of the overlapping region after shifting the frame
// up by `offset`, restricted to the content ROI columns. Mirrors
// overlap_mean_abs_diff (used to verify the template match).
float overlapMeanAbsDiff(const std::vector<float> &prevGray,
                         const std::vector<float> &frameGray,
                         int width,
                         int roiX,
                         int roiW,
                         int offset,
                         int overlapHeight)
{
    if (roiW <= 0 || overlapHeight <= 0 || width <= 0) {
        return std::numeric_limits<float>::infinity();
    }

    float sum = 0.0f;
    int count = 0;
    for (int row = 0; row < overlapHeight; ++row) {
        const int prevY = offset + row;
        const int frameY = row;
        const std::size_t prevBase = static_cast<std::size_t>(prevY) * width + roiX;
        const std::size_t frameBase = static_cast<std::size_t>(frameY) * width + roiX;
        for (int col = 0; col < roiW; ++col) {
            sum += std::abs(prevGray[prevBase + col] - frameGray[frameBase + col]);
            ++count;
        }
    }
    if (count == 0) {
        return std::numeric_limits<float>::infinity();
    }
    return sum / count;
}

}  // namespace

std::optional<OrbEstimate> estimateOrbOffset(const QImage &prev, const QImage &frame, int minOverlap)
{
    if (prev.width() != frame.width() || prev.height() != frame.height()) {
        return std::nullopt;
    }
    if (prev.width() < 80 || prev.height() < 120) {
        return std::nullopt;
    }

    const cv::Mat prevMat = grayToMat(prev);
    const cv::Mat frameMat = grayToMat(frame);
    const cv::Mat mask = buildFeatureMask(prev.width(), prev.height());

    cv::Ptr<cv::ORB> orb = cv::ORB::create(kOrbMaxFeatures);

    std::vector<cv::KeyPoint> prevKeypoints;
    cv::Mat prevDescriptors;
    orb->detectAndCompute(prevMat, mask, prevKeypoints, prevDescriptors);

    std::vector<cv::KeyPoint> currKeypoints;
    cv::Mat currDescriptors;
    orb->detectAndCompute(frameMat, mask, currKeypoints, currDescriptors);

    if (prevKeypoints.size() < static_cast<std::size_t>(kOrbMinKeypoints)
        || currKeypoints.size() < static_cast<std::size_t>(kOrbMinKeypoints)
        || prevDescriptors.empty() || currDescriptors.empty()) {
        return std::nullopt;
    }

    cv::BFMatcher matcher(cv::NORM_HAMMING, false);
    std::vector<std::vector<cv::DMatch>> knnMatches;
    matcher.knnMatch(currDescriptors, prevDescriptors, knnMatches, 2);

    std::vector<cv::Point2f> currPoints;
    std::vector<cv::Point2f> prevPoints;
    int rawMatches = 0;
    for (const std::vector<cv::DMatch> &pair : knnMatches) {
        if (pair.size() < 2) {
            continue;
        }
        const cv::DMatch &best = pair[0];
        const cv::DMatch &second = pair[1];
        if (best.distance >= second.distance * static_cast<float>(kLoweRatio)) {
            continue;
        }

        const cv::Point2f currPt = currKeypoints[best.queryIdx].pt;
        const cv::Point2f prevPt = prevKeypoints[best.trainIdx].pt;
        const double dx = static_cast<double>(prevPt.x) - currPt.x;
        const double dy = static_cast<double>(prevPt.y) - currPt.y;
        if (dy <= 1.0 || std::abs(dx) > kOrbMaxDx * 2.0) {
            continue;
        }

        currPoints.push_back(currPt);
        prevPoints.push_back(prevPt);
        ++rawMatches;
    }

    if (rawMatches < kOrbMinMatches) {
        return std::nullopt;
    }

    cv::Mat inliers;
    const cv::Mat affine = cv::estimateAffinePartial2D(currPoints, prevPoints, inliers,
                                                       cv::RANSAC, 3.0, 2000, 0.99, 10);
    if (affine.empty()) {
        return std::nullopt;
    }

    const double a = affine.at<double>(0, 0);
    const double b = affine.at<double>(0, 1);
    const double c = affine.at<double>(1, 0);
    const double d = affine.at<double>(1, 1);
    const double tx = affine.at<double>(0, 2);
    const double ty = affine.at<double>(1, 2);

    const double scale = (std::sqrt(a * a + c * c) + std::sqrt(b * b + d * d)) * 0.5;
    const double geomDrift = std::abs(a - 1.0) + std::abs(d - 1.0) + std::abs(b) + std::abs(c);

    if (std::abs(tx) > kOrbMaxDx
        || ty <= 1.0
        || ty >= static_cast<double>(prev.height() - minOverlap)
        || std::abs(scale - 1.0) > kOrbMaxGeometryDrift
        || geomDrift > kOrbMaxGeometryDrift) {
        return std::nullopt;
    }

    int inlierCount = 0;
    for (int row = 0; row < inliers.rows; ++row) {
        if (inliers.at<uchar>(row, 0) != 0) {
            ++inlierCount;
        }
    }
    if (inlierCount < kOrbMinInliers) {
        return std::nullopt;
    }

    const float inlierRatio = static_cast<float>(inlierCount) / rawMatches;
    const float confidence = (1.0f - inlierRatio) * 3.5f
        + static_cast<float>(std::abs(tx) / kOrbMaxDx)
        + static_cast<float>(geomDrift) * 6.0f;

    return OrbEstimate{ty, confidence};
}

std::optional<std::pair<int, float>> templateFallback(const QImage &prev,
                                                      const QImage &frame,
                                                      int lastOffset,
                                                      int minOverlap)
{
    if (prev.width() != frame.width() || prev.height() != frame.height()) {
        return std::nullopt;
    }

    const int width = prev.width();
    const int height = prev.height();
    const cv::Rect roi = contentRoi(width, height);
    const int roiX = roi.x;
    const int roiY = roi.y;
    const int roiW = roi.width;
    const int roiH = roi.height;
    if (roiH < kTemplateMinHeight * 2 || roiW < 40) {
        return std::nullopt;
    }

    const int templateH = std::min(std::max(roiH / 3, kTemplateMinHeight), roiH - 1);
    const int searchStart = roiY;
    const int searchEnd = roiY + roiH - templateH;
    if (searchEnd <= searchStart) {
        return std::nullopt;
    }

    const std::vector<float> prevGray = toGrayscaleVec(prev);
    const std::vector<float> frameGray = toGrayscaleVec(frame);
    const int frameTemplateY = roiY;

    int bestOffset = 0;
    float bestScore = std::numeric_limits<float>::lowest();
    float secondScore = std::numeric_limits<float>::lowest();

    const int maxOffset = std::max(0, height - minOverlap);
    const int predict = std::clamp(lastOffset, 0, std::min(maxOffset, searchEnd - searchStart));
    for (int offset : predictOffsetIter(searchEnd - searchStart, predict)) {
        const int searchY = searchStart + offset;
        if (searchY < 0 || searchY + templateH > height) {
            continue;
        }
        const float score = nccScoreRegion(prevGray, frameGray, width, roiX, roiW,
                                            searchY, frameTemplateY, templateH);
        if (score > bestScore) {
            secondScore = bestScore;
            bestScore = score;
            bestOffset = offset;
        } else if (score > secondScore) {
            secondScore = score;
        }
    }

    if (bestScore < kTemplateFallbackMinScore) {
        return std::nullopt;
    }
    if (std::isfinite(secondScore) && bestScore - secondScore < kTemplateFallbackMinMargin) {
        return std::nullopt;
    }

    const float verification = overlapMeanAbsDiff(prevGray, frameGray, width, roiX, roiW,
                                                  bestOffset, std::max(0, height - bestOffset));
    if (!std::isfinite(verification) || verification > kTemplateVerifyMaxDiff) {
        return std::nullopt;
    }

    const float confidence = (1.0f - std::max(bestScore, 0.0f)) * 8.0f + verification / 10.0f;
    return std::make_pair(bestOffset, confidence);
}

}  // namespace markshot::scroll

#endif  // HAVE_OPENCV
