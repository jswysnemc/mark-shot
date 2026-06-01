#pragma once

#include "scroll/stitcher.h"

#include <QString>

// Resolution of the scrolling-capture algorithm from its name, the user config
// file (config.json -> scroll.algorithm), and the built-in default. The default
// is col-sample: it is ~24x faster than ORB, has motion prediction, and handles
// repetitive/low-texture content, which suits typical page/document scrolling.
// ORB is offered as an opt-in for noisy or slightly-scaled image content.
namespace markshot::scroll {

// The built-in default when nothing else specifies one.
inline constexpr StitchAlgorithm kDefaultAlgorithm = StitchAlgorithm::ColSample;

// Canonical CLI/config name for an algorithm ("col-sample" / "opencv-orb").
QString algorithmName(StitchAlgorithm algorithm);

// Parses a name (case-insensitive, accepts a few aliases) into an algorithm.
// Returns fallback when the name is empty or unrecognised.
StitchAlgorithm parseAlgorithmName(const QString &name, StitchAlgorithm fallback);

// Reads scroll.algorithm from config.json, falling back to kDefaultAlgorithm.
StitchAlgorithm configuredScrollAlgorithm();

// Process-level override, set once from the CLI (--scroll-algorithm). When set,
// it wins over the config file. Empty/unset means "no override".
void setScrollAlgorithmOverride(StitchAlgorithm algorithm);
void clearScrollAlgorithmOverride();

// The algorithm a new session should start with: CLI override, else config
// file, else kDefaultAlgorithm.
StitchAlgorithm resolveScrollAlgorithm();

}  // namespace markshot::scroll
