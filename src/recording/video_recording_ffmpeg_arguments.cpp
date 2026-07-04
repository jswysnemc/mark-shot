#include "recording/video_recording_ffmpeg_arguments.h"

namespace markshot::recording {

QStringList buildVideoRecordingFfmpegArguments(const RecordingOptions &options,
                                               QSize frameSize,
                                               int fps,
                                               const QStringList &audioArguments,
                                               const RecordingVideoEncoderOptions &encoder)
{
    const QString sizeText = QStringLiteral("%1x%2").arg(frameSize.width()).arg(frameSize.height());
    QStringList arguments{
        QStringLiteral("-hide_banner"),
        QStringLiteral("-loglevel"),
        QStringLiteral("error"),
        QStringLiteral("-y"),
    };
    arguments.append(encoder.globalArguments);
    arguments << QStringLiteral("-f")
              << QStringLiteral("rawvideo")
              << QStringLiteral("-pix_fmt")
              << QStringLiteral("bgra")
              << QStringLiteral("-video_size")
              << sizeText
              << QStringLiteral("-framerate")
              << QString::number(fps)
              << QStringLiteral("-i")
              << QStringLiteral("pipe:0");
    arguments.append(audioArguments);
    arguments << QStringLiteral("-map") << QStringLiteral("0:v");
    if (options.includeAudio) {
        arguments << QStringLiteral("-map") << QStringLiteral("1:a") << QStringLiteral("-shortest");
    }
    arguments.append(encoder.videoArguments);
    if (options.includeAudio) {
        arguments << QStringLiteral("-c:a") << QStringLiteral("aac");
    }
    arguments << QStringLiteral("-fps_mode")
              << QStringLiteral("cfr")
              << QStringLiteral("-avoid_negative_ts")
              << QStringLiteral("make_zero")
              << QStringLiteral("-movflags")
              << QStringLiteral("+faststart")
              << options.outputPath;
    return arguments;
}

}  // namespace markshot::recording
