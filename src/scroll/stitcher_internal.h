#pragma once

#include "scroll/stitcher.h"

#include <QImage>

#include <utility>
#include <vector>

namespace markshot::scroll::stitcher_internal {

// Internal helpers for the normalized vertical stitching pipeline. Horizontal
// sessions transpose frames before calling these functions, so "height", "row",
// "top", and "bottom" mean "length along the active scroll axis".

// Geometry and overlap utilities used by the matcher. overhangAmount() returns
// newly visible content beyond the current full image and identifies the edge.
std::pair<int, int> bandRange(int width, float startRatio, float endRatio);
int overhangAmount(int pos, int frameHeight, int fullHeight, StitchEdge *edge);
int sideIgnoreWidth(int width);
std::vector<int> predictOffsetIter(int max, int predict);

// Image normalization helpers keep all algorithm state in pixel coordinates with
// devicePixelRatio set to 1, avoiding Qt high-DPI scaling in comparisons.
QImage transposeImage(const QImage &src);
QImage normalizePixelImage(QImage image);

// Debug labels are centralized so logs from the session window and stitcher use
// the same terminology.
const char *algorithmDebugName();
const char *axisDebugName(ScrollAxis axis);
const char *edgeDebugName(StitchEdge edge);
void logStitchDebug(const char *format, ...);

}  // namespace markshot::scroll::stitcher_internal
