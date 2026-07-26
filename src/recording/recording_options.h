#pragma once

#include "recording/recording_capture_backend.h"
#include "recording/recording_container_format.h"
#include "recording/recording_quality_options.h"

#include <QRect>
#include <QString>

namespace markshot::recording {

enum class RecordingMode {
    Gif,
    Video,
};

enum class RecordingScope {
    Display,
    Region,
};

struct DisplaySource {
    bool allOutputs = false;
    QString screenName;
    QString outputName;
    QString title;
    QRect geometry;
};

struct RecordingOptions {
    RecordingMode mode = RecordingMode::Gif;
    RecordingScope scope = RecordingScope::Region;
    int fps = 12;
    int countdownSeconds = 0;
    bool includeAudio = false;
    RecordingCaptureBackend captureBackend = RecordingCaptureBackend::Auto;
    RecordingContainerFormat container = RecordingContainerFormat::Mp4;
    RecordingQuality quality = RecordingQuality::Balanced;
    DisplaySource display;
    QRect captureGeometry;
    QString outputPath;
};

}  // namespace markshot::recording
