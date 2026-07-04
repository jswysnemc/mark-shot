#pragma once

#include <QString>
#include <QVector>

namespace markshot::recording {

enum class RecordingCaptureBackend {
    Auto,
    Wlroots,
    PipeWire,
    Polling,
};

/**
 * 从环境变量读取录制采集后端。
 * @return 环境变量未设置或无效时返回 Auto。
 */
RecordingCaptureBackend recordingCaptureBackendFromEnvironment();

/**
 * 读取录制采集后端名称。
 * @param backend 采集后端。
 * @return 后端名称。
 */
QString recordingCaptureBackendName(RecordingCaptureBackend backend);

/**
 * 计算录制采集后端尝试顺序。
 * @param requested 用户请求的后端。
 * @return 后端尝试顺序。
 */
QVector<RecordingCaptureBackend> recordingCaptureBackendOrder(RecordingCaptureBackend requested);

}  // namespace markshot::recording
