#include "recording/recording_capture_memory_policy.h"

#include <algorithm>
#include <chrono>
#include <limits>

namespace markshot::recording {

quint64 recordingCaptureMemoryBudget()
{
    bool valid = false;
    const int mib = qEnvironmentVariableIntValue("MARK_SHOT_RECORDING_CAPTURE_MIB", &valid);
    return static_cast<quint64>(valid && mib > 0 ? mib : 96) * 1024 * 1024;
}

qint64 recordingCaptureClockMs()
{
    return std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now().time_since_epoch()).count();
}

quint64 recordingEffectiveCaptureBudget(quint64 requestedBytes, quint64 largestHeldBytes,
                                       quint64 configuredBytes)
{
    const quint64 largest = std::max(requestedBytes, largestHeldBytes);
    const quint64 twoFrames = largest > std::numeric_limits<quint64>::max() / 2
        ? std::numeric_limits<quint64>::max() : largest * 2;
    return std::max(configuredBytes, twoFrames);
}

}  // namespace markshot::recording
