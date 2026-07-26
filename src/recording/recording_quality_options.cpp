#include "recording/recording_quality_options.h"

namespace markshot::recording {

RecordingQualityProfile recordingQualityProfile(RecordingQuality quality, int fps)
{
    // 高帧率下留给每帧的编码预算更少，统一放宽一档速度预设
    const bool highFrameRate = fps >= 48;

    RecordingQualityProfile profile;
    switch (quality) {
    case RecordingQuality::Efficient:
        profile.constantQuality = 28;
        profile.softwarePreset = QStringLiteral("ultrafast");
        profile.hardwarePreset = QStringLiteral("p1");
        profile.bitRateFactor = 0.6;
        return profile;
    case RecordingQuality::High:
        profile.constantQuality = 18;
        profile.softwarePreset = highFrameRate ? QStringLiteral("veryfast")
                                               : QStringLiteral("faster");
        profile.hardwarePreset = highFrameRate ? QStringLiteral("p4") : QStringLiteral("p6");
        profile.bitRateFactor = 1.8;
        return profile;
    case RecordingQuality::Balanced:
        break;
    }

    profile.constantQuality = 23;
    profile.softwarePreset = highFrameRate ? QStringLiteral("ultrafast") : QStringLiteral("veryfast");
    profile.hardwarePreset = highFrameRate ? QStringLiteral("p2") : QStringLiteral("p4");
    profile.bitRateFactor = 1.0;
    return profile;
}

QString recordingQualityName(RecordingQuality quality)
{
    switch (quality) {
    case RecordingQuality::Efficient:
        return QStringLiteral("efficient");
    case RecordingQuality::High:
        return QStringLiteral("high");
    case RecordingQuality::Balanced:
        break;
    }
    return QStringLiteral("balanced");
}

RecordingQuality recordingQualityFromName(QString text)
{
    text = text.trimmed().toLower();
    if (text == QStringLiteral("efficient") || text == QStringLiteral("small")) {
        return RecordingQuality::Efficient;
    }
    if (text == QStringLiteral("high") || text == QStringLiteral("quality")) {
        return RecordingQuality::High;
    }
    return RecordingQuality::Balanced;
}

}  // namespace markshot::recording
