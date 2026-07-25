#include "recording/recording_frame_grabber.h"

#include "platform/wayland/wlroots_screencopy_capture_stream.h"
#include "recording/recording_pipewire_capture_stream.h"
#include "recording/recording_polling_capture_stream.h"
#include "recording/recording_windows_wgc_capture_stream.h"

#include <utility>

namespace markshot::recording {
namespace {

/**
 * 【录制】【采集后端】创建平台默认采集流。
 * @param backend 采集后端。
 * @param options 录制配置。
 * @param parent 父对象。
 * @return 可用时返回采集流，否则返回空。
 */
std::unique_ptr<RecordingCaptureStream> createDefaultCaptureStream(
    RecordingCaptureBackend backend,
    const RecordingOptions &options,
    QObject *parent)
{
    switch (backend) {
    case RecordingCaptureBackend::Wlroots:
        return createWlrootsScreencopyCaptureStream(options, parent);
    case RecordingCaptureBackend::PipeWire:
        return createPipeWireRecordingCaptureStream(options, parent);
    case RecordingCaptureBackend::WindowsWgc:
        return createWindowsWgcRecordingCaptureStream(options, parent);
    case RecordingCaptureBackend::Polling:
        return std::make_unique<RecordingPollingCaptureStream>(options, parent);
    case RecordingCaptureBackend::Auto:
        break;
    }
    return nullptr;
}

}  // namespace

RecordingFrameGrabber::RecordingFrameGrabber(RecordingOptions options, QObject *parent)
    : RecordingFrameGrabber(std::move(options), createDefaultCaptureStream, parent)
{
}

}  // namespace markshot::recording
