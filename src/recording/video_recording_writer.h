#pragma once

#include "recording/libav_recording_process.h"
#include "recording/recording_frame_pacer.h"
#include "recording/recording_options.h"
#include "recording/recording_video_encoder_options.h"
#include "recording/recording_writer.h"

#include <QVector>

namespace markshot::recording {

class VideoRecordingWriter final : public RecordingWriter {
public:
    /**
     * 创建视频写出器。
     * @param options 录制配置。
     */
    explicit VideoRecordingWriter(RecordingOptions options);

    bool start(QSize frameSize, int fps, QString *error) override;
    bool writeFrame(const RecordingFrameSample &sample, QString *error) override;
    bool finish(QString *error) override;
    void cancel() override;

private:
    /**
     * 启动指定编码候选。
     * @param candidateIndex 编码候选下标。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool startCandidate(int candidateIndex, QString *error);

    /**
     * 失败时尝试启动下一个编码候选。
     * @param error 输出错误信息。
     * @return 成功切换到下一个候选时返回 true。
     */
    bool startNextCandidate(QString *error);

    /**
     * 判断当前写入失败是否仍处于可安全回退的启动阶段。
     * @param sample 触发失败的帧样本。
     * @return 可以放弃当前编码器并重启下一个候选时返回 true。
     */
    bool canFallbackAfterWriteFailure(const RecordingFrameSample &sample) const;

    /**
     * 写入当前样本并按需补写上一帧。
     * @param sample 当前帧样本。
     * @param error 输出错误信息。
     * @return 写入成功时返回 true。
     */
    bool writeSampleWithPacing(const RecordingFrameSample &sample, QString *error);

    RecordingOptions m_options;
    LibavRecordingProcess m_libavProcess;
    RecordingFramePacer m_pacer;
    QVector<RecordingVideoEncoderOptions> m_candidates;
    QSize m_frameSize;
    int m_fps = 30;
    int m_candidateIndex = 0;
    int m_writtenFrames = 0;
};

}  // namespace markshot::recording
