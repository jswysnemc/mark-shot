#include "scroll/stitch_orb.h"

#ifdef HAVE_OPENCV

namespace markshot::scroll {

std::optional<OrbEstimate> estimateOrbOffset(const QImage &, const QImage &, int)
{
    // Implemented in task #9.
    return std::nullopt;
}

std::optional<std::pair<int, float>> templateFallback(const QImage &, const QImage &, int, int)
{
    // Implemented in task #9.
    return std::nullopt;
}

}  // namespace markshot::scroll

#endif  // HAVE_OPENCV
