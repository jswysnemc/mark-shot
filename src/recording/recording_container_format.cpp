#include "recording/recording_container_format.h"

namespace markshot::recording {

QString recordingContainerExtension(RecordingContainerFormat format)
{
    return format == RecordingContainerFormat::Mkv ? QStringLiteral("mkv") : QStringLiteral("mp4");
}

QString recordingContainerMuxerName(RecordingContainerFormat format)
{
    return format == RecordingContainerFormat::Mkv ? QStringLiteral("matroska")
                                                   : QStringLiteral("mp4");
}

QString recordingContainerLabel(RecordingContainerFormat format)
{
    // MKV 在录制中断后仍可播放，MP4 需要写出容器尾才完整
    return format == RecordingContainerFormat::Mkv
        ? QStringLiteral("MKV")
        : QStringLiteral("MP4");
}

RecordingContainerFormat recordingContainerFromName(QString text)
{
    text = text.trimmed().toLower();
    if (text == QStringLiteral("mkv") || text == QStringLiteral("matroska")) {
        return RecordingContainerFormat::Mkv;
    }
    return RecordingContainerFormat::Mp4;
}

QString recordingContainerName(RecordingContainerFormat format)
{
    return recordingContainerExtension(format);
}

}  // namespace markshot::recording
