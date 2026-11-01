#include "recording/recording_windows_wgc_capture_stream.h"

#include "debug_log.h"
#include "recording/recording_frame_handoff.h"
#include "platform/windows/windows_wgc_stream.h"

#include <algorithm>
#include <atomic>
#include <utility>

namespace markshot::recording {
namespace {

#if defined(Q_OS_WIN)

class RecordingWindowsWgcCaptureStream final : public RecordingCaptureStream {
public:
    /**
     * 创建 Windows Graphics Capture 录制采集流。
     * @param options 录制配置。
     * @param parent 父对象。
     */
    explicit RecordingWindowsWgcCaptureStream(RecordingOptions options, QObject *parent = nullptr)
        : RecordingCaptureStream(parent)
        , m_options(std::move(options))
    {
        qRegisterMetaType<RecordingFrameSample>("markshot::recording::RecordingFrameSample");
    }

    /** 停止采集后再销毁会话，后台发布句柄不引用适配器。 */
    ~RecordingWindowsWgcCaptureStream() override
    {
        stop();
    }

    /**
     * 启动 Windows Graphics Capture 采集。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(QString *error) override
    {
        if (error) {
            error->clear();
        }
        if (m_options.display.allOutputs && m_options.scope == RecordingScope::Display) {
            if (error) {
                *error = QStringLiteral("Windows Graphics Capture recording does not support all-outputs recording");
            }
            return false;
        }

        stop();
        m_backpressureActive = false;
        m_handoff = std::make_unique<RecordingFrameHandoff>(this);
        connect(m_handoff.get(), &RecordingFrameHandoff::frameReady,
                this, &RecordingCaptureStream::frameReady);
        connect(m_handoff.get(), &RecordingFrameHandoff::failed,
                this, &RecordingCaptureStream::failed);

        CaptureRequest request;
        request.sourceGeometry = m_options.captureGeometry;
        request.preferredOutputName = m_options.display.outputName;
        request.allOutputs = false;
        request.preferScreencast = true;
        request.allowInteractivePortal = false;
        request.allowPortalScreenshotFallback = false;
        request.includeCursor = true;
        request.targetFps = std::max(1, m_options.fps);

        QString streamError;
        if (!m_stream.start(
                request,
                [publish = m_handoff->publisher(), sequence = std::make_shared<std::atomic<qint64>>(0)]
                (markshot::platform::windows::WindowsWgcFrame frame) {
                    // 1. 【录制】【帧交接】回调仅持有发布句柄和序号，不访问适配器生命周期
                    RecordingFrameSample sample;
                    sample.bgra.bytes = std::move(frame.bgra);
                    sample.bgra.size = frame.size;
                    sample.bgra.stride = frame.stride;
                    sample.timestampMs = frame.timestampMs;
                    sample.sequence = sequence->fetch_add(1, std::memory_order_relaxed) + 1;
                    publish(std::move(sample));
                },
                m_handoff->errorPublisher(),
                &streamError)) {
            stop();
            if (error) {
                *error = streamError;
            }
            return false;
        }

        markshot::debugLog("recording",
                           "【录制】【Windows采集】started mode=wgc-stream geometry=%d,%d %dx%d fps=%d",
                           m_options.captureGeometry.x(),
                           m_options.captureGeometry.y(),
                           m_options.captureGeometry.width(),
                           m_options.captureGeometry.height(),
                           m_options.fps);
        return true;
    }

    /**
     * 停止 Windows Graphics Capture 采集。
     * @return 无返回值。
     */
    void stop() override
    {
        if (m_handoff) {
            m_handoff->close();
        }
        m_stream.stop();
    }

    /**
     * 设置写出背压状态。
     * @param active 队列繁忙时为 true。
     * @return 无返回值。
     */
    void setBackpressureActive(bool active) override
    {
        if (m_backpressureActive == active) {
            return;
        }
        m_backpressureActive = active;
        if (m_handoff) {
            m_handoff->setEnabled(!active);
        }
        // 背压状态下沉到 WGC 回调层，让丢帧发生在纹理读回之前
        m_stream.setBackpressure(active);
    }

private:
    RecordingOptions m_options;
    markshot::platform::windows::WindowsWgcStream m_stream;
    std::unique_ptr<RecordingFrameHandoff> m_handoff;
    bool m_backpressureActive = false;
};

#endif

}  // namespace

std::unique_ptr<RecordingCaptureStream> createWindowsWgcRecordingCaptureStream(const RecordingOptions &options,
                                                                               QObject *parent)
{
#if defined(Q_OS_WIN)
    QString error;
    if (!markshot::platform::windows::windowsWgcStreamSupported(&error)) {
        markshot::debugLog("recording",
                           "【录制】【Windows采集】unavailable error=%s",
                           error.toUtf8().constData());
        return nullptr;
    }
    return std::make_unique<RecordingWindowsWgcCaptureStream>(options, parent);
#else
    Q_UNUSED(options)
    Q_UNUSED(parent)
    return nullptr;
#endif
}

}  // namespace markshot::recording
