#pragma once

#include "capture_freeze_scope.h"
#include "recording/recording_options.h"
#include "shot_window.h"
#include "startup_config.h"

#include <QPointer>
#include <QVector>

#include <optional>

class QApplication;

namespace markshot {

/// @brief 启动截图会话并显示对应冻结窗口。
/// @param app QApplication 实例。
/// @param allOutputs 是否把所有输出捕获为一张虚拟桌面图片。
/// @param freezeScope 普通区域截图的显示器冻结范围。
/// @param includeCursor 冻结图是否包含鼠标。
/// @param useRegularWindow 是否使用普通窗口替代 layer-shell。
/// @param fullscreenAnnotation 是否直接进入全屏标注。
/// @param defaultTools 默认工具配置。
/// @param error 输出错误信息。
/// @param regionRecordingOptions 区域录制配置，为空时启动普通截图流程。
/// @return 创建出的截图窗口列表。
QVector<QPointer<ShotWindow>> showCaptureSession(QApplication *app,
                                                 bool allOutputs,
                                                 CaptureFreezeScope freezeScope,
                                                 bool includeCursor,
                                                 bool useRegularWindow,
                                                 bool fullscreenAnnotation,
                                                 const DefaultTools &defaultTools,
                                                 QString *error,
                                                 std::optional<recording::RecordingOptions> regionRecordingOptions = std::nullopt);

}  // namespace markshot
