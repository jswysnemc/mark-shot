#include "recording/recording_status.h"

namespace markshot::recording {

QString recordingModeName(RecordingMode mode)
{
    return mode == RecordingMode::Gif ? QStringLiteral("gif") : QStringLiteral("video");
}

}  // namespace markshot::recording
