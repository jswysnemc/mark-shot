#include "platform/wayland/wlroots_screencopy_worker.h"

#include <QTimer>

#include <algorithm>
#include <utility>

namespace markshot::recording {

#ifdef HAVE_WLROOTS_SCREENCOPY

/**
 * 【录制】【wlroots采集】处理 compositor 返回的 wl_shm buffer 参数。
 * @param data 工作对象指针。
 * @param frame screencopy frame。
 * @param format 像素格式。
 * @param width 帧宽度。
 * @param height 帧高度。
 * @param stride 行跨度。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleFrameBuffer(void *data,
                                                zwlr_screencopy_frame_v1 *frame,
                                                uint32_t format,
                                                uint32_t width,
                                                uint32_t height,
                                                uint32_t stride)
{
    Q_UNUSED(frame)
    auto *self = static_cast<WlrootsScreencopyWorker *>(data);
    if (!self) {
        return;
    }
    self->m_pendingFormat = format;
    self->m_pendingWidth = static_cast<int>(width);
    self->m_pendingHeight = static_cast<int>(height);
    self->m_pendingStride = static_cast<int>(stride);
    self->m_hasPendingBuffer = true;
    if (self->m_screencopyVersion < 3) {
        self->copyPendingFrame();
    }
}

/**
 * 【录制】【wlroots采集】记录 frame 标志。
 * @param data 工作对象指针。
 * @param frame screencopy frame。
 * @param flags frame 标志。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleFrameFlags(void *data,
                                               zwlr_screencopy_frame_v1 *frame,
                                               uint32_t flags)
{
    Q_UNUSED(frame)
    auto *self = static_cast<WlrootsScreencopyWorker *>(data);
    if (self) {
        self->m_yInvert = (flags & ZWLR_SCREENCOPY_FRAME_V1_FLAGS_Y_INVERT) != 0;
    }
}

/**
 * 【录制】【wlroots采集】处理 frame 就绪事件。
 * @param data 工作对象指针。
 * @param frame screencopy frame。
 * @param tvSecHi 秒高位。
 * @param tvSecLo 秒低位。
 * @param tvNsec 纳秒。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleFrameReady(void *data,
                                               zwlr_screencopy_frame_v1 *frame,
                                               uint32_t tvSecHi,
                                               uint32_t tvSecLo,
                                               uint32_t tvNsec)
{
    Q_UNUSED(frame)
    auto *self = static_cast<WlrootsScreencopyWorker *>(data);
    if (self) {
        self->publishReadyFrame(tvSecHi, tvSecLo, tvNsec);
    }
}

/**
 * 【录制】【wlroots采集】处理 frame 失败事件。
 * @param data 工作对象指针。
 * @param frame screencopy frame。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleFrameFailed(void *data,
                                                zwlr_screencopy_frame_v1 *frame)
{
    Q_UNUSED(frame)
    auto *self = static_cast<WlrootsScreencopyWorker *>(data);
    if (self) {
        self->handleCopyFailed();
    }
}

/**
 * 【录制】【wlroots采集】接收 damage 区域。
 * @param data 工作对象指针。
 * @param frame screencopy frame。
 * @param x 损坏区域横坐标。
 * @param y 损坏区域纵坐标。
 * @param width 损坏区域宽度。
 * @param height 损坏区域高度。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleFrameDamage(void *data,
                                                zwlr_screencopy_frame_v1 *frame,
                                                uint32_t x,
                                                uint32_t y,
                                                uint32_t width,
                                                uint32_t height)
{
    Q_UNUSED(data)
    Q_UNUSED(frame)
    Q_UNUSED(x)
    Q_UNUSED(y)
    Q_UNUSED(width)
    Q_UNUSED(height)
}

/**
 * 【录制】【wlroots采集】接收 DMA-BUF 参数。
 * @param data 工作对象指针。
 * @param frame screencopy frame。
 * @param format DMA-BUF 格式。
 * @param width 帧宽度。
 * @param height 帧高度。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleFrameLinuxDmabuf(void *data,
                                                     zwlr_screencopy_frame_v1 *frame,
                                                     uint32_t format,
                                                     uint32_t width,
                                                     uint32_t height)
{
    Q_UNUSED(data)
    Q_UNUSED(frame)
    Q_UNUSED(format)
    Q_UNUSED(width)
    Q_UNUSED(height)
}

/**
 * 【录制】【wlroots采集】在 buffer 枚举完成后提交 wl_shm buffer。
 * @param data 工作对象指针。
 * @param frame screencopy frame。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleFrameBufferDone(void *data,
                                                    zwlr_screencopy_frame_v1 *frame)
{
    Q_UNUSED(frame)
    auto *self = static_cast<WlrootsScreencopyWorker *>(data);
    if (self) {
        self->copyPendingFrame();
    }
}

/**
 * 【录制】【wlroots采集】请求下一帧。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::requestFrame()
{
    if (!m_running || m_backpressureActive || m_capturePending
        || !m_screencopy || !m_selectedOutput || !m_selectedOutput->output) {
        return;
    }

    // 1. 【录制】【wlroots采集】根据录制区域决定请求整个输出或输出内局部区域
    const QRect outputGeometry = m_selectedOutput->geometry;
    const QRect requested = m_options.captureGeometry.normalized();
    const bool useRegion = requested.isValid() && !requested.isEmpty()
        && outputGeometry.isValid() && !outputGeometry.isEmpty()
        && requested != outputGeometry;
    if (useRegion) {
        const QRect clipped = requested.intersected(outputGeometry);
        if (clipped.isEmpty()) {
            emit failed(QStringLiteral("wlroots screencopy requested region is outside the output"));
            return;
        }
        const QRect relative = clipped.translated(-outputGeometry.topLeft());
        m_frame = zwlr_screencopy_manager_v1_capture_output_region(m_screencopy,
                                                                    1,
                                                                    m_selectedOutput->output,
                                                                    relative.x(),
                                                                    relative.y(),
                                                                    relative.width(),
                                                                    relative.height());
    } else {
        m_frame = zwlr_screencopy_manager_v1_capture_output(m_screencopy,
                                                            1,
                                                            m_selectedOutput->output);
    }

    if (!m_frame) {
        emit failed(QStringLiteral("failed to create wlroots screencopy frame"));
        return;
    }
    // 2. 【录制】【wlroots采集】注册 frame 回调并交给 compositor 异步复制
    m_capturePending = true;
    m_yInvert = false;
    m_hasPendingBuffer = false;
    zwlr_screencopy_frame_v1_add_listener(m_frame, &s_frameListener, this);
    wl_display_flush(m_display);
}

/**
 * 【录制】【wlroots采集】提交当前 pending frame 的 wl_shm buffer。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::copyPendingFrame()
{
    if (!m_frame || !m_hasPendingBuffer) {
        return;
    }
    // 1. 【录制】【wlroots采集】从缓冲池复用或创建满足当前帧参数的共享内存 buffer
    QString error;
    m_currentBuffer = m_buffers.acquire(m_shm,
                                        m_pendingFormat,
                                        m_pendingWidth,
                                        m_pendingHeight,
                                        m_pendingStride,
                                        &error);
    if (!m_currentBuffer) {
        destroyCurrentFrame();
        if (error == QStringLiteral("wlroots screencopy buffer pool is temporarily full")) {
            QTimer::singleShot(2, this, [this] {
                requestFrame();
            });
            return;
        }
        emit failed(error);
        return;
    }
    // 2. 【录制】【wlroots采集】优先使用带 damage 的复制接口，旧协议回退普通 copy
    if (m_screencopyVersion >= 2) {
        zwlr_screencopy_frame_v1_copy_with_damage(m_frame, m_currentBuffer->buffer());
    } else {
        zwlr_screencopy_frame_v1_copy(m_frame, m_currentBuffer->buffer());
    }
    wl_display_flush(m_display);
}

/**
 * 【录制】【wlroots采集】发布已经复制完成的帧。
 * @param tvSecHi 秒高位。
 * @param tvSecLo 秒低位。
 * @param tvNsec 纳秒。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::publishReadyFrame(uint32_t tvSecHi,
                                                uint32_t tvSecLo,
                                                uint32_t tvNsec)
{
    // 1. 【录制】【wlroots采集】把 wl_shm 内容包装成 raw BGRA 视图，由样本持有缓冲生命周期
    QString error;
    if (!m_currentBuffer) {
        emit failed(QStringLiteral("wlroots screencopy ready frame has no buffer"));
        destroyCurrentFrame();
        return;
    }
    RecordingRawBgraFrame frame =
        m_currentBuffer->mappedBgraFrame(std::static_pointer_cast<const void>(m_currentBuffer),
                                         m_yInvert,
                                         &error);
    if (!frame.isValid()) {
        emit failed(error);
        destroyCurrentFrame();
        return;
    }

    // 2. 【录制】【wlroots采集】使用 compositor presentation time 保持录制时间轴稳定
    const qint64 presentationUs = presentationTimeUs(tvSecHi, tvSecLo, tvNsec);
    if (m_basePresentationUs < 0) {
        m_basePresentationUs = presentationUs;
    }
    RecordingFrameSample sample;
    sample.bgra = std::move(frame);
    sample.timestampMs = std::max<qint64>(0, (presentationUs - m_basePresentationUs) / 1000);
    sample.sequence = ++m_sequence;
    m_failedFrames = 0;
    emit frameReady(sample);
    destroyCurrentFrame();
    requestFrame();
}

/**
 * 【录制】【wlroots采集】处理单帧复制失败。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::handleCopyFailed()
{
    destroyCurrentFrame();
    ++m_failedFrames;
    if (m_failedFrames > 16) {
        emit failed(QStringLiteral("wlroots screencopy failed too many times"));
        return;
    }
    requestFrame();
}

/**
 * 【录制】【wlroots采集】销毁当前 screencopy frame。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::destroyCurrentFrame()
{
    if (m_frame) {
        zwlr_screencopy_frame_v1_destroy(m_frame);
        m_frame = nullptr;
    }
    m_currentBuffer.reset();
    m_capturePending = false;
    m_hasPendingBuffer = false;
}

/**
 * 【录制】【wlroots采集】计算 compositor presentation 时间。
 * @param tvSecHi 秒高位。
 * @param tvSecLo 秒低位。
 * @param tvNsec 纳秒。
 * @return 微秒时间。
 */
qint64 WlrootsScreencopyWorker::presentationTimeUs(uint32_t tvSecHi,
                                                   uint32_t tvSecLo,
                                                   uint32_t tvNsec) const
{
    const quint64 seconds = (static_cast<quint64>(tvSecHi) << 32)
        | static_cast<quint64>(tvSecLo);
    if (seconds == 0 && tvNsec == 0) {
        return m_clock.isValid() ? m_clock.nsecsElapsed() / 1000 : 0;
    }
    return static_cast<qint64>(seconds * 1000000ULL + tvNsec / 1000ULL);
}

const zwlr_screencopy_frame_v1_listener WlrootsScreencopyWorker::s_frameListener{
    &WlrootsScreencopyWorker::handleFrameBuffer,
    &WlrootsScreencopyWorker::handleFrameFlags,
    &WlrootsScreencopyWorker::handleFrameReady,
    &WlrootsScreencopyWorker::handleFrameFailed,
    &WlrootsScreencopyWorker::handleFrameDamage,
    &WlrootsScreencopyWorker::handleFrameLinuxDmabuf,
    &WlrootsScreencopyWorker::handleFrameBufferDone,
};

#endif

}  // namespace markshot::recording
