#pragma once

#include "recording/libav_gif_recording_process.h"
#include "recording/recording_options.h"
#include "recording/recording_writer.h"

namespace markshot::recording {

class GifRecordingWriter final : public RecordingWriter {
public:
    /**
     * 创建 GIF 写出器。
     * @param options 录制配置。
     */
    explicit GifRecordingWriter(RecordingOptions options);

    bool start(QSize frameSize, int fps, QString *error) override;
    bool writeFrame(const RecordingFrameSample &sample, QString *error) override;
    bool finish(QString *error) override;
    void cancel() override;

private:
    RecordingOptions m_options;
    LibavGifRecordingProcess m_process;
};

}  // namespace markshot::recording
