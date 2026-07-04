#pragma once

#include "platform/wayland/wlroots_screencopy_buffer_pool.h"
#include "recording/recording_frame_sample.h"
#include "recording/recording_options.h"

#include <QElapsedTimer>
#include <QObject>
#include <QRect>

#include <memory>
#include <vector>

#ifdef HAVE_WLROOTS_SCREENCOPY
#include <wayland-client.h>

#include "wlr-screencopy-unstable-v1-client-protocol.h"
#endif

class QSocketNotifier;

namespace markshot::recording {

#ifdef HAVE_WLROOTS_SCREENCOPY

struct WlrootsOutput {
    wl_output *output = nullptr;
    QRect geometry;
    QString name;
    QString description;
    int scale = 1;
};

class WlrootsScreencopyWorker final : public QObject {
    Q_OBJECT

public:
    /**
     * 创建 wlroots screencopy 工作对象。
     * @param options 录制配置。
     */
    explicit WlrootsScreencopyWorker(RecordingOptions options);

    /**
     * 启动 Wayland screencopy 会话。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(QString *error);

    /**
     * 停止 Wayland screencopy 会话。
     * @return 无返回值。
     */
    void stop();

    /**
     * 设置背压状态。
     * @param active 队列繁忙时为 true。
     * @return 无返回值。
     */
    void setBackpressureActive(bool active);

signals:
    void frameReady(const markshot::recording::RecordingFrameSample &sample);
    void failed(const QString &error);

private:
    static void handleRegistryGlobal(void *data,
                                     wl_registry *registry,
                                     uint32_t name,
                                     const char *interface,
                                     uint32_t version);
    static void handleRegistryRemove(void *data, wl_registry *registry, uint32_t name);
    static void handleOutputGeometry(void *data,
                                     wl_output *output,
                                     int32_t x,
                                     int32_t y,
                                     int32_t physicalWidth,
                                     int32_t physicalHeight,
                                     int32_t subpixel,
                                     const char *make,
                                     const char *model,
                                     int32_t transform);
    static void handleOutputMode(void *data,
                                 wl_output *output,
                                 uint32_t flags,
                                 int32_t width,
                                 int32_t height,
                                 int32_t refresh);
    static void handleOutputDone(void *data, wl_output *output);
    static void handleOutputScale(void *data, wl_output *output, int32_t scale);
    static void handleOutputName(void *data, wl_output *output, const char *name);
    static void handleOutputDescription(void *data, wl_output *output, const char *description);
    static void handleFrameBuffer(void *data,
                                  zwlr_screencopy_frame_v1 *frame,
                                  uint32_t format,
                                  uint32_t width,
                                  uint32_t height,
                                  uint32_t stride);
    static void handleFrameFlags(void *data, zwlr_screencopy_frame_v1 *frame, uint32_t flags);
    static void handleFrameReady(void *data,
                                 zwlr_screencopy_frame_v1 *frame,
                                 uint32_t tvSecHi,
                                 uint32_t tvSecLo,
                                 uint32_t tvNsec);
    static void handleFrameFailed(void *data, zwlr_screencopy_frame_v1 *frame);
    static void handleFrameDamage(void *data,
                                  zwlr_screencopy_frame_v1 *frame,
                                  uint32_t x,
                                  uint32_t y,
                                  uint32_t width,
                                  uint32_t height);
    static void handleFrameLinuxDmabuf(void *data,
                                       zwlr_screencopy_frame_v1 *frame,
                                       uint32_t format,
                                       uint32_t width,
                                       uint32_t height);
    static void handleFrameBufferDone(void *data, zwlr_screencopy_frame_v1 *frame);

    /**
     * 分发 Wayland 事件。
     * @return 无返回值。
     */
    void dispatchEvents();

    /**
     * 选择需要录制的输出。
     * @return 输出状态，找不到时返回空。
     */
    WlrootsOutput *chooseOutput() const;

    /**
     * 请求下一帧。
     * @return 无返回值。
     */
    void requestFrame();

    /**
     * 提交当前 pending frame 的 wl_shm buffer。
     * @return 无返回值。
     */
    void copyPendingFrame();

    /**
     * 发布已经复制完成的帧。
     * @param tvSecHi 秒高位。
     * @param tvSecLo 秒低位。
     * @param tvNsec 纳秒。
     * @return 无返回值。
     */
    void publishReadyFrame(uint32_t tvSecHi, uint32_t tvSecLo, uint32_t tvNsec);

    /**
     * 处理单帧复制失败。
     * @return 无返回值。
     */
    void handleCopyFailed();

    /**
     * 销毁当前 screencopy frame。
     * @return 无返回值。
     */
    void destroyCurrentFrame();

    /**
     * 计算 compositor presentation 时间。
     * @param tvSecHi 秒高位。
     * @param tvSecLo 秒低位。
     * @param tvNsec 纳秒。
     * @return 微秒时间。
     */
    qint64 presentationTimeUs(uint32_t tvSecHi, uint32_t tvSecLo, uint32_t tvNsec) const;

    /**
     * 释放 Wayland 资源。
     * @return 无返回值。
     */
    void cleanup();

    static const wl_registry_listener s_registryListener;
    static const wl_output_listener s_outputListener;
    static const zwlr_screencopy_frame_v1_listener s_frameListener;

    RecordingOptions m_options;
    QSocketNotifier *m_notifier = nullptr;
    QElapsedTimer m_clock;
    wl_display *m_display = nullptr;
    wl_registry *m_registry = nullptr;
    wl_shm *m_shm = nullptr;
    zwlr_screencopy_manager_v1 *m_screencopy = nullptr;
    zwlr_screencopy_frame_v1 *m_frame = nullptr;
    std::vector<std::unique_ptr<WlrootsOutput>> m_outputs;
    WlrootsOutput *m_selectedOutput = nullptr;
    WlrootsScreencopyBufferPool m_buffers;
    std::shared_ptr<WlrootsScreencopyShmBuffer> m_currentBuffer;
    uint32_t m_screencopyVersion = 1;
    uint32_t m_pendingFormat = 0;
    int m_pendingWidth = 0;
    int m_pendingHeight = 0;
    int m_pendingStride = 0;
    qint64 m_basePresentationUs = -1;
    qint64 m_sequence = 0;
    int m_failedFrames = 0;
    bool m_running = false;
    bool m_backpressureActive = false;
    bool m_capturePending = false;
    bool m_hasPendingBuffer = false;
    bool m_yInvert = false;
};

#endif

}  // namespace markshot::recording
