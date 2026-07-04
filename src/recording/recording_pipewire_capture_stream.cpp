#include "recording/recording_pipewire_capture_stream.h"

#include "debug_log.h"
#include "screen_capture.h"

#include <QElapsedTimer>
#include <QMetaObject>
#include <QTimer>

#include <algorithm>
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
        m_timer.setTimerType(Qt::PreciseTimer);
        m_timer.setSingleShot(true);
        connect(&m_timer, &QTimer::timeout, this, &RecordingPipeWireCaptureStream::captureNextFrame);
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
        if (m_options.mode != RecordingMode::Video) {
            if (error) {
                *error = QStringLiteral("PipeWire recording capture only supports video mode");
            }
            return false;
        }
        if (m_options.display.allOutputs && m_options.scope == RecordingScope::Display) {
            if (error) {
                *error = QStringLiteral("PipeWire recording capture does not support all-outputs recording");
            }
            return false;
        }

        m_intervalUs = std::max<qint64>(1, 1000000 / std::max(1, m_options.fps));
        m_nextCaptureUs = 0;
        m_baseFrameTimeMs = -1;
        m_sequence = 0;
        m_running = true;
        m_clock.restart();

        // 1. 【录制】【PipeWire采集】同步抓取首帧，以便授权失败时让上层执行回退
        RecordingFrameSample firstSample;
        QString firstError;
        if (!captureSample(&firstSample, &firstError)) {
            stop();
            if (error) {
                *error = firstError;
            }
            return false;
        }

        // 2. 【录制】【PipeWire采集】首帧排入事件队列，避免 start 阶段重入写出器
        QMetaObject::invokeMethod(
            this,
            [this, firstSample] {
                if (m_running) {
                    emit frameReady(firstSample);
                }
            },
            Qt::QueuedConnection);
        advanceNextCaptureTime();
        scheduleNextCapture();
        markshot::debugLog("recording",
                           "【录制】【PipeWire采集】started geometry=%d,%d %dx%d fps=%d",
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
        m_timer.stop();
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
        if (!m_backpressureActive && m_running) {
            scheduleNextCapture(0);
        }
    }

private:
    /**
     * 抓取下一帧 PipeWire 图像。
     * @return 无返回值。
     */
    void captureNextFrame()
    {
        if (!m_running || m_capturing || m_backpressureActive) {
            return;
        }
        if (m_clock.isValid() && m_clock.nsecsElapsed() / 1000 < m_nextCaptureUs) {
            scheduleNextCapture();
            return;
        }

        m_capturing = true;
        RecordingFrameSample sample;
        QString error;
        if (!captureSample(&sample, &error)) {
            m_capturing = false;
            emit failed(error);
            return;
        }
        m_capturing = false;
        advanceNextCaptureTime();
        emit frameReady(sample);
        scheduleNextCapture();
    }

    /**
     * 通过 portal screencast 读取一个录制样本。
     * @param sample 输出录制样本。
     * @param error 输出错误信息。
     * @return 读取成功时返回 true。
     */
    bool captureSample(RecordingFrameSample *sample, QString *error)
    {
        if (error) {
            error->clear();
        }
        CaptureRequest request;
        request.sourceGeometry = m_options.captureGeometry;
        request.preferredOutputName = m_options.display.outputName;
        request.allOutputs = false;
        request.preferScreencast = true;
        request.allowInteractivePortal = true;
        request.allowPortalScreenshotFallback = false;
        request.includeCursor = true;
        request.targetFps = std::max(1, m_options.fps);

        // 1. 【录制】【PipeWire采集】复用 portal screencast 流读取最新帧
        const CaptureResult result = m_screencast.capture(request);
        if (result.image.isNull()) {
            if (error) {
                *error = result.error.isEmpty()
                    ? QStringLiteral("PipeWire recording capture did not produce a frame")
                    : result.error;
            }
            return false;
        }

        // 2. 【录制】【PipeWire采集】使用流时间作为录制时间轴，缺失时回退本地时钟
        qint64 timestampMs = m_clock.isValid() ? m_clock.elapsed() : 0;
        if (result.frameTimeMs > 0) {
            if (m_baseFrameTimeMs < 0) {
                m_baseFrameTimeMs = result.frameTimeMs;
            }
            timestampMs = std::max<qint64>(0, result.frameTimeMs - m_baseFrameTimeMs);
        }
        sample->image = result.image;
        sample->timestampMs = timestampMs;
        sample->sequence = ++m_sequence;
        return true;
    }

    /**
     * 安排下一次 PipeWire 抓取。
     * @param delayOverrideMs 指定延迟，负数表示按目标时间计算。
     * @return 无返回值。
     */
    void scheduleNextCapture(int delayOverrideMs = -1)
    {
        if (!m_running || m_timer.isActive()) {
            return;
        }
        int delayMs = delayOverrideMs;
        if (delayMs < 0) {
            const qint64 nowUs = m_clock.isValid() ? m_clock.nsecsElapsed() / 1000 : 0;
            const qint64 remainingUs = std::max<qint64>(0, m_nextCaptureUs - nowUs);
            delayMs = static_cast<int>((remainingUs + 999) / 1000);
        }
        m_timer.start(std::max(0, delayMs));
    }

    /**
     * 推进下一帧目标时间。
     * @return 无返回值。
     */
    void advanceNextCaptureTime()
    {
        if (!m_clock.isValid()) {
            return;
        }
        const qint64 nowUs = m_clock.nsecsElapsed() / 1000;
        if (m_nextCaptureUs <= 0) {
            m_nextCaptureUs = nowUs + m_intervalUs;
            return;
        }
        do {
            m_nextCaptureUs += m_intervalUs;
        } while (m_nextCaptureUs <= nowUs);
    }

    RecordingOptions m_options;
    PortalPipeWireScreencast m_screencast;
    QTimer m_timer;
    QElapsedTimer m_clock;
    qint64 m_intervalUs = 16666;
    qint64 m_nextCaptureUs = 0;
    qint64 m_baseFrameTimeMs = -1;
    qint64 m_sequence = 0;
    bool m_running = false;
    bool m_capturing = false;
    bool m_backpressureActive = false;
};

#endif

}  // namespace

std::unique_ptr<RecordingCaptureStream> createPipeWireRecordingCaptureStream(const RecordingOptions &options,
                                                                             QObject *parent)
{
#ifdef HAVE_PIPEWIRE
    if (options.mode != RecordingMode::Video) {
        return nullptr;
    }
    return std::make_unique<RecordingPipeWireCaptureStream>(options, parent);
#else
    Q_UNUSED(options)
    Q_UNUSED(parent)
    return nullptr;
#endif
}

}  // namespace markshot::recording
