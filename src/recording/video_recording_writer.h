#pragma once

#include "recording/ffmpeg_recording_process.h"
#include "recording/recording_options.h"
#include "recording/recording_writer.h"

namespace markshot::recording {

class VideoRecordingWriter final : public RecordingWriter {
public:
    /**
     * 创建视频写出器。
     * @param options 录制配置。
     */
    explicit VideoRecordingWriter(RecordingOptions options);

    bool start(QSize frameSize, int fps, QString *error) override;
    bool writeFrame(const QImage &frame, QString *error) override;
    bool finish(QString *error) override;
    void cancel() override;

private:
    RecordingOptions m_options;
    FfmpegRecordingProcess m_process;
};

}  // namespace markshot::recording
