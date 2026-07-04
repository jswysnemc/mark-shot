#include "recording/recording_capture_backend.h"

#include <QProcessEnvironment>

namespace markshot::recording {
namespace {

/**
 * 【录制】【采集后端】解析后端名称。
 * @param text 环境变量文本。
 * @return 识别到的后端。
 */
RecordingCaptureBackend parseBackend(QString text)
{
    text = text.trimmed().toLower();
    if (text == QStringLiteral("wlroots") || text == QStringLiteral("wlroots-screencopy")) {
        return RecordingCaptureBackend::Wlroots;
    }
    if (text == QStringLiteral("pipewire") || text == QStringLiteral("portal")) {
        return RecordingCaptureBackend::PipeWire;
    }
    if (text == QStringLiteral("polling") || text == QStringLiteral("fallback")) {
        return RecordingCaptureBackend::Polling;
    }
    return RecordingCaptureBackend::Auto;
}

}  // namespace

/**
 * 【录制】【采集后端】从环境变量读取录制采集后端。
 * @return 环境变量未设置或无效时返回 Auto。
 */
RecordingCaptureBackend recordingCaptureBackendFromEnvironment()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    return parseBackend(env.value(QStringLiteral("MARK_SHOT_RECORDING_BACKEND")));
}

/**
 * 【录制】【采集后端】读取录制采集后端名称。
 * @param backend 采集后端。
 * @return 后端名称。
 */
QString recordingCaptureBackendName(RecordingCaptureBackend backend)
{
    switch (backend) {
    case RecordingCaptureBackend::Auto:
        return QStringLiteral("auto");
    case RecordingCaptureBackend::Wlroots:
        return QStringLiteral("wlroots-screencopy");
    case RecordingCaptureBackend::PipeWire:
        return QStringLiteral("pipewire");
    case RecordingCaptureBackend::Polling:
        return QStringLiteral("polling");
    }
    return QStringLiteral("auto");
}

/**
 * 【录制】【采集后端】计算录制采集后端尝试顺序。
 * @param requested 用户请求的后端。
 * @return 后端尝试顺序。
 */
QVector<RecordingCaptureBackend> recordingCaptureBackendOrder(RecordingCaptureBackend requested)
{
    switch (requested) {
    case RecordingCaptureBackend::Auto:
        return {
            RecordingCaptureBackend::Wlroots,
            RecordingCaptureBackend::PipeWire,
            RecordingCaptureBackend::Polling,
        };
    case RecordingCaptureBackend::Wlroots:
        return {
            RecordingCaptureBackend::Wlroots,
            RecordingCaptureBackend::Polling,
        };
    case RecordingCaptureBackend::PipeWire:
        return {
            RecordingCaptureBackend::PipeWire,
            RecordingCaptureBackend::Wlroots,
            RecordingCaptureBackend::Polling,
        };
    case RecordingCaptureBackend::Polling:
        return {requested};
    }
    return {RecordingCaptureBackend::Wlroots,
            RecordingCaptureBackend::PipeWire,
            RecordingCaptureBackend::Polling};
}

}  // namespace markshot::recording
