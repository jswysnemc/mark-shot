#pragma once

#include "recording/recording_frame_sample.h"
#include "recording/recording_options.h"
#include "recording/recording_video_encoder_options.h"

#include <QSize>
#include <QString>

#include <memory>

namespace markshot::recording {

class LibavRecordingProcessPrivate;

class LibavRecordingProcess final {
public:
    LibavRecordingProcess();
    ~LibavRecordingProcess();

    /**
     * 启动库内 FFmpeg 编码器。
     * @param options 录制配置。
     * @param encoder 编码器候选。
     * @param frameSize 输入帧尺寸。
     * @param fps 目标帧率。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(const RecordingOptions &options,
               const RecordingVideoEncoderOptions &encoder,
               QSize frameSize,
               int fps,
               QString *error);

    /**
     * 写入一帧录制样本。
     * @param sample 录制帧样本。
     * @param error 输出错误信息。
     * @return 写入成功时返回 true。
     */
    bool writeFrame(const RecordingFrameSample &sample, QString *error);

    /**
     * 冲刷编码器并关闭输出文件。
     * @param error 输出错误信息。
     * @return 完成成功时返回 true。
     */
    bool finish(QString *error);

    /**
     * 取消当前库内编码任务。
     * @return 无返回值。
     */
    void cancel();

private:
    std::unique_ptr<LibavRecordingProcessPrivate> m_impl;
};

}  // namespace markshot::recording
