#include "recording/recording_pipewire_capture_stream.h"

#include "debug_log.h"
#include "screen_capture.h"

#include <QMetaObject>

#include <algorithm>
#include <atomic>
#include <utility>

#ifdef HAVE_PIPEWIRE
#include "screen_capture_pipewire_screencast.h"
#endif

namespace markshot::recording {
namespace {

#ifdef HAVE_PIPEWIRE

class RecordingPipeWireCaptureStream final : public RecordingCaptureStream {
public:
    /**
     * 创建 PipeWire portal 录制采集流。
     * @param options 录制配置。
     * @param parent 父对象。
     */
    explicit RecordingPipeWireCaptureStream(RecordingOptions options, QObject *parent = nullptr)
        : RecordingCaptureStream(parent)
        , m_options(std::move(options))
    {
        qRegisterMetaType<RecordingFrameSample>("markshot::recording::RecordingFrameSample");
    }

    /**
     * 启动 PipeWire portal 采集。
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
                *error = QStringLiteral("PipeWire recording capture does not support all-outputs recording");
            }
            return false;
        }

        m_sequence = 0;
        m_running = true;

        CaptureRequest request;
        request.sourceGeometry = m_options.captureGeometry;
        request.preferredOutputName = m_options.display.outputName;
        request.allOutputs = false;
        request.preferScreencast = true;
        request.allowInteractivePortal = true;
        request.allowPortalScreenshotFallback = false;
        request.includeCursor = true;
        request.targetFps = std::max(1, m_options.fps);

        QString streamError;
        if (!m_screencast.startRawStream(
                request,
                [this](PipeWireScreencastRawFrame frame) {
                    handleRawFrame(std::move(frame));
                },
                [this](QString errorText) {
                    handleRawError(std::move(errorText));
                },
                &streamError)) {
            stop();
            if (error) {
                *error = streamError;
            }
            return false;
        }

        markshot::debugLog("recording",
                           "【录制】【PipeWire采集】started mode=stream-callback geometry=%d,%d %dx%d fps=%d",
                           m_options.captureGeometry.x(),
                           m_options.captureGeometry.y(),
                           m_options.captureGeometry.width(),
                           m_options.captureGeometry.height(),
                           m_options.fps);
        return true;
    }

    /**
     * 停止 PipeWire portal 采集。
     * @return 无返回值。
     */
    void stop() override
    {
        m_running = false;
        m_screencast.stop();
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
        // 背压状态下沉到 PipeWire 回调层，让丢帧发生在拷贝之前
        m_screencast.setRawBackpressure(active);
    }

private:
    /**
     * 处理 PipeWire stream 回调送来的 raw BGRA 帧。
     * @param frame raw BGRA 帧。
     * @return 无返回值。
     */
    void handleRawFrame(PipeWireScreencastRawFrame frame)
    {
        if (!m_running || m_backpressureActive || frame.bgra.isEmpty() || frame.size.isEmpty()) {
            return;
        }

        RecordingFrameSample sample;
        sample.bgra.bytes = std::move(frame.bgra);
        sample.bgra.size = frame.size;
        sample.bgra.stride = frame.stride;
        sample.bgra.yInverted = frame.yInverted;
        sample.timestampMs = frame.timestampMs;
        sample.sequence = ++m_sequence;
        QMetaObject::invokeMethod(
            this,
            [this, sample = std::move(sample)] {
                if (m_running) {
                    emit frameReady(sample);
                }
            },
            Qt::QueuedConnection);
    }

    /**
     * 处理 PipeWire stream 回调错误。
     * @param errorText 错误文本。
     * @return 无返回值。
     */
    void handleRawError(QString errorText)
    {
        if (!m_running || errorText.isEmpty()) {
            return;
        }
        QMetaObject::invokeMethod(
            this,
            [this, errorText = std::move(errorText)] {
                if (m_running) {
                    emit failed(errorText);
                }
            },
            Qt::QueuedConnection);
    }

    RecordingOptions m_options;
    PortalPipeWireScreencast m_screencast;
    qint64 m_sequence = 0;
    std::atomic_bool m_running{false};
    std::atomic_bool m_backpressureActive{false};
};

#endif

}  // namespace

std::unique_ptr<RecordingCaptureStream> createPipeWireRecordingCaptureStream(const RecordingOptions &options,
                                                                             QObject *parent)
{
#ifdef HAVE_PIPEWIRE
    return std::make_unique<RecordingPipeWireCaptureStream>(options, parent);
#else
    Q_UNUSED(options)
    Q_UNUSED(parent)
    return nullptr;
#endif
}

}  // namespace markshot::recording
