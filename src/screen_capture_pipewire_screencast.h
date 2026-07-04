#pragma once

#include "screen_capture_internal.h"

#ifdef HAVE_PIPEWIRE

class PortalPipeWireScreencast final {
public:
    /**
     * 销毁 PipeWire 截屏会话。
     */
    ~PortalPipeWireScreencast();

    /**
     * 捕获当前最新流帧。
     * @param request 捕获请求。
     * @return 捕获结果。
     */
    CaptureResult capture(const CaptureRequest &request);

    /**
     * 停止 Portal 和 PipeWire 会话。
     * @return 无返回值。
     */
    void stop();

private:
    /**
     * 启动可用的 Portal 截屏会话。
     * @param includeCursor 是否请求合成鼠标。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(bool includeCursor, QString *error);

#ifdef HAVE_LIBPORTAL
    /**
     * 使用 libportal 启动截屏会话。
     * @param includeCursor 是否请求合成鼠标。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool startWithLibportal(bool includeCursor, QString *error);
#endif

    /**
     * 使用 D-Bus Portal 启动截屏会话。
     * @param includeCursor 是否请求合成鼠标。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool startWithDbusPortal(bool includeCursor, QString *error);

    /**
     * 连接 PipeWire 流。
     * @param fd PipeWire 远端文件描述符。
     * @param error 输出错误信息。
     * @return 连接成功时返回 true。
     */
    bool startPipeWire(int fd, QString *error);

    /**
     * 读取 PipeWire 状态名称。
     * @param state PipeWire 流状态。
     * @return 状态名称。
     */
    static const char *streamStateName(pw_stream_state state);

    static void onStreamStateChanged(void *data,
                                     pw_stream_state old,
                                     pw_stream_state state,
                                     const char *error);
    static void onStreamParamChanged(void *data, uint32_t id, const spa_pod *param);
    static void onStreamProcess(void *data);

    /**
     * 判断当前 PipeWire 回调帧是否应该丢弃。
     * @param frameTimeMs 当前帧时间。
     * @return 需要丢弃时返回 true。
     */
    bool shouldDropIncomingFrame(qint64 frameTimeMs);

    /**
     * 从 PipeWire buffer 复制 CPU 可访问图像。
     * @param pipewireBuffer PipeWire buffer。
     * @param error 输出错误信息。
     * @return 转换后的图像。
     */
    QImage imageFromBuffer(pw_buffer *pipewireBuffer, QString *error) const;

    /**
     * 转换四字节像素格式。
     * @param source 源像素数据。
     * @param width 宽度。
     * @param height 高度。
     * @param stride 行跨度。
     * @param format PipeWire 像素格式。
     * @return 转换后的图像。
     */
    static QImage convertFourByteFrame(const uchar *source,
                                       int width,
                                       int height,
                                       int stride,
                                       spa_video_format format);

    /**
     * 转换三字节像素格式。
     * @param source 源像素数据。
     * @param width 宽度。
     * @param height 高度。
     * @param stride 行跨度。
     * @param format PipeWire 像素格式。
     * @return 转换后的图像。
     */
    static QImage convertThreeByteFrame(const uchar *source,
                                        int width,
                                        int height,
                                        int stride,
                                        spa_video_format format);

    bool m_started = false;
    uint m_nodeId = 0;
    QString m_targetObject;
    QString m_sessionHandle;
    bool m_ownsDbusSessionHandle = false;
    bool m_cursorIncluded = false;
    int m_targetFps = 0;
    qint64 m_minFrameIntervalUs = 0;
#ifdef HAVE_LIBPORTAL
    XdpPortal *m_libportalPortal = nullptr;
    XdpSession *m_libportalSession = nullptr;
#endif
    QString m_lastError;
    QVariantMap m_streamProperties;
    QMutex m_frameMutex;
    QWaitCondition m_frameReady;
    QImage m_latestFrame;
    qint64 m_latestFrameTimeMs = 0;
    QRect m_streamGeometry;
    pw_thread_loop *m_loop = nullptr;
    pw_context *m_context = nullptr;
    pw_core *m_core = nullptr;
    pw_stream *m_stream = nullptr;
    spa_hook m_streamListener = {};
    pw_stream_events m_streamEvents = {};
    spa_video_info_raw m_videoInfo = {};
    int m_frameCount = 0;
    int m_droppedFrameCount = 0;
};

#endif
