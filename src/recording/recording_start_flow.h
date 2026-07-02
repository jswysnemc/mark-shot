#pragma once

#include "recording/recording_options.h"

#include <QString>

#include <functional>

class QWidget;

namespace markshot::recording {

struct RecordingStartFlowRequest {
    RecordingMode initialMode = RecordingMode::Video;
    QWidget *parent = nullptr;
    bool stayOnTop = false;
    std::function<void(RecordingOptions)> startDisplayRecording;
    std::function<void(RecordingOptions)> selectRegionRecording;
    std::function<void(const QString &)> showError;
};

/**
 * 运行录制启动流程。
 * @param request 启动流程回调和初始配置。
 * @return 用户确认对话框时返回 true，用户取消时返回 false。
 */
bool runRecordingStartFlow(const RecordingStartFlowRequest &request);

}  // namespace markshot::recording
