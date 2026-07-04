#pragma once

#include "recording/recording_options.h"

#include <QJsonObject>

namespace markshot::recording {

struct RecordingDialogConfig {
    RecordingMode mode = RecordingMode::Video;
    RecordingScope scope = RecordingScope::Region;
    RecordingCaptureBackend backend = RecordingCaptureBackend::Auto;
    int fps = 30;
    int videoFps = 30;
    int gifFps = 12;
    bool includeAudio = false;
    QString outputPath;
    QString displayKey;
};

/**
 * 从应用配置根对象读取录制启动界面配置。
 * @param root 应用配置根对象。
 * @param mode 路由决定的录制模式。
 * @return 录制启动界面配置。
 */
RecordingDialogConfig recordingDialogConfigFromRoot(const QJsonObject &root, RecordingMode mode);

/**
 * 读取当前录制启动界面配置。
 * @param mode 路由决定的录制模式。
 * @return 录制启动界面配置。
 */
RecordingDialogConfig configuredRecordingDialogConfig(RecordingMode mode);

/**
 * 保存录制启动界面配置。
 * @param config 录制启动界面配置。
 * @param error 输出错误信息。
 * @return 保存成功时返回 true。
 */
bool saveRecordingDialogConfig(const RecordingDialogConfig &config, QString *error = nullptr);

/**
 * 根据录制选项生成启动界面配置。
 * @param options 录制选项。
 * @return 启动界面配置。
 */
RecordingDialogConfig recordingDialogConfigFromOptions(const RecordingOptions &options);

/**
 * 生成显示器来源持久化键。
 * @param source 显示器来源。
 * @return 持久化键。
 */
QString recordingDisplayPersistenceKey(const DisplaySource &source);

}  // namespace markshot::recording
