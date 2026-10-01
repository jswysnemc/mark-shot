#include "screen_capture_pipewire_screencast.h"

#include "screen_capture_portal_guard.h"

#include "pipewire/pipewire_dmabuf_importer.h"

#include <QDateTime>

#ifdef HAVE_PIPEWIRE

PortalPipeWireScreencast::~PortalPipeWireScreencast()
{
    stop();
}

CaptureResult PortalPipeWireScreencast::capture(const CaptureRequest &request)
{
    if (m_rawStreamMode.load(std::memory_order_acquire)) {
        stop();
    }
    const int requestedTargetFps = std::max(0, request.targetFps);
    if (m_started && requestedTargetFps != m_targetFps) {
        stop();
    }
    m_targetFps = requestedTargetFps;
    m_minFrameIntervalUs = m_targetFps > 0
        ? std::max<qint64>(1, 1000000 / m_targetFps)
        : 0;

    const bool firstStart = !m_started;
    if (!m_started) {
        if (!request.allowInteractivePortal) {
            markshot::debugLog("screencast",
                               "【录制】【PipeWire授权】skip interactive screencast start");
            return {{},
                    QStringLiteral("portal screencast requires interactive authorization"),
                    {},
                    request.sourceGeometry};
        }
        QString error;
        if (!start(request, &error)) {
            markshot::debugLog("screencast", "start failed: %s",
                               error.toUtf8().constData());
            stop();
            return {{}, error, {}, request.sourceGeometry};
        }
        markshot::debugLog("screencast",
                           "started node_id=%u target=%s session=%s",
                           m_nodeId,
                           m_targetObject.isEmpty() ? "(none)"
                                                    : m_targetObject.toUtf8().constData(),
                           m_sessionHandle.toUtf8().constData());
    }

    if (firstStart) {
        markshot::debugLog("screencast",
                           "first-frame wait timeout_ms=%lu",
                           kScreencastFirstFrameTimeoutMs);
    }

    QMutexLocker locker(&m_frameMutex);
    bool waited = false;
    const qint64 minimumFrameTimeMs = request.minimumFrameTimeMs;
    auto hasUsableFrame = [&] {
        return !m_latestFrame.isNull()
            && (minimumFrameTimeMs <= 0 || m_latestFrameTimeMs >= minimumFrameTimeMs);
    };
    if (!hasUsableFrame()) {
        waited = true;
        const qint64 deadlineMs = QDateTime::currentMSecsSinceEpoch()
            + static_cast<qint64>(kScreencastFirstFrameTimeoutMs);
        while (!hasUsableFrame()) {
            const qint64 remainingMs = deadlineMs - QDateTime::currentMSecsSinceEpoch();
            if (remainingMs <= 0) {
                break;
            }
            const unsigned long waitMs =
                static_cast<unsigned long>(std::min<qint64>(remainingMs, 250));
            if (!m_frameReady.wait(&m_frameMutex, waitMs)) {
                continue;
            }
        }
    }
    if (!hasUsableFrame()) {
        const QString error = m_lastError.isEmpty()
            ? QStringLiteral("portal screencast did not produce a usable frame")
            : QStringLiteral("portal screencast did not produce a usable frame: %1").arg(m_lastError);
        markshot::debugLog("screencast",
                           "no-frame first_start=%d waited=%d latest_time=%lld minimum_time=%lld last_error=%s",
                           firstStart ? 1 : 0, waited ? 1 : 0,
                           m_latestFrameTimeMs, minimumFrameTimeMs,
                           m_lastError.isEmpty() ? "(none)"
                                                 : m_lastError.toUtf8().constData());
        return {{}, error, {}, request.sourceGeometry};
    }

    const QImage frame = m_latestFrame;
    const QRect streamGeometry = m_streamGeometry;
    const qint64 frameTimeMs = m_latestFrameTimeMs;
    locker.unlock();

    const QRect requested = request.sourceGeometry;
    const bool wantCrop = requested.isValid() && !requested.isEmpty();
    QImage image = wantCrop
        ? markshot::capture::cropFrameToRequest(frame, streamGeometry, requested)
        : frame;
    const QSize croppedSize = image.size();
    markshot::debugLog("screencast",
                       "frame raw=%dx%d stream_geom=%d,%d %dx%d requested=%d,%d %dx%d "
                       "want_crop=%d result=%dx%d frame_time=%lld minimum_time=%lld",
                       frame.width(), frame.height(),
                       streamGeometry.x(), streamGeometry.y(),
                       streamGeometry.width(), streamGeometry.height(),
                       requested.x(), requested.y(), requested.width(), requested.height(),
                       wantCrop ? 1 : 0,
                       croppedSize.width(), croppedSize.height(),
                       frameTimeMs, minimumFrameTimeMs);
    if (image.isNull()) {
        // 1. 【屏幕共享】【输出匹配】错误显示器的授权不应固定影响下一次长截图
        if (m_persistence) {
            m_persistence->invalidate();
        }
        markshot::debugLog("screencast",
                           "crop-miss frame=%dx%d stream_geom=%d,%d %dx%d requested=%d,%d %dx%d",
                           frame.width(), frame.height(),
                           streamGeometry.x(), streamGeometry.y(),
                           streamGeometry.width(), streamGeometry.height(),
                           requested.x(), requested.y(),
                           requested.width(), requested.height());
        return {{}, QStringLiteral("portal screencast frame does not cover requested geometry"), {}, request.sourceGeometry};
    }
    return {image, {}, request.preferredOutputName, request.sourceGeometry, m_cursorIncluded, frameTimeMs};
}

void PortalPipeWireScreencast::setRawBackpressure(bool active)
{
    m_rawBackpressure.store(active, std::memory_order_relaxed);
}

bool PortalPipeWireScreencast::startRawStream(const CaptureRequest &request,
                                             RawFrameCallback frameCallback,
                                             ErrorCallback errorCallback,
                                             QString *error)
{
    if (error) {
        error->clear();
    }
    if (!frameCallback) {
        if (error) {
            *error = QStringLiteral("PipeWire raw stream callback is empty");
        }
        return false;
    }
    if (m_started) {
        stop();
    }
    if (!request.allowInteractivePortal) {
        if (error) {
            *error = QStringLiteral("portal screencast requires interactive authorization");
        }
        return false;
    }

    {
        QMutexLocker locker(&m_frameMutex);
        m_rawFrameCallback = std::move(frameCallback);
        m_rawErrorCallback = std::move(errorCallback);
        m_rawStreamMode.store(true, std::memory_order_release);
    }
    m_rawRequestedGeometry = request.sourceGeometry;
    m_rawOutputName = request.preferredOutputName;
    m_rawBaseFrameTimeMs = -1;
    m_rawBackpressure = false;
    m_rawDmaBufDirectReadBroken = false;
    m_targetFps = std::max(0, request.targetFps);
    m_minFrameIntervalUs = m_targetFps > 0
        ? std::max<qint64>(1, 1000000 / m_targetFps)
        : 0;

    QString startError;
    if (!start(request, &startError)) {
        stop();
        if (error) {
            *error = startError;
        }
        return false;
    }
    markshot::debugLog("screencast",
                       "【录制】【PipeWire流回调】started node_id=%u target=%s session=%s fps=%d",
                       m_nodeId,
                       m_targetObject.isEmpty() ? "(none)" : m_targetObject.toUtf8().constData(),
                       m_sessionHandle.toUtf8().constData(),
                       m_targetFps);
    return true;
}

void PortalPipeWireScreencast::stop()
{
    {
        QMutexLocker locker(&m_frameMutex);
        m_rawStreamMode.store(false, std::memory_order_release);
        m_rawFrameCallback = {};
        m_rawErrorCallback = {};
    }
    if (m_loop) {
        pw_thread_loop_lock(m_loop);
        if (m_stream) {
            pw_stream_disconnect(m_stream);
            pw_stream_destroy(m_stream);
            m_stream = nullptr;
        }
        if (m_core) {
            pw_core_disconnect(m_core);
            m_core = nullptr;
        }
        if (m_context) {
            pw_context_destroy(m_context);
            m_context = nullptr;
        }
        pw_thread_loop_unlock(m_loop);
        pw_thread_loop_stop(m_loop);
        pw_thread_loop_destroy(m_loop);
        m_loop = nullptr;
    } else {
        if (m_stream) {
            pw_stream_disconnect(m_stream);
            pw_stream_destroy(m_stream);
            m_stream = nullptr;
        }
        if (m_core) {
            pw_core_disconnect(m_core);
            m_core = nullptr;
        }
        if (m_context) {
            pw_context_destroy(m_context);
            m_context = nullptr;
        }
    }
    if (m_ownsDbusSessionHandle && !m_sessionHandle.isEmpty()) {
        QDBusInterface session(QStringLiteral("org.freedesktop.portal.Desktop"),
                               m_sessionHandle,
                               QStringLiteral("org.freedesktop.portal.Session"),
                               QDBusConnection::sessionBus());
        if (session.isValid()) {
            session.call(QStringLiteral("Close"));
        }
    }
#ifdef HAVE_LIBPORTAL
    if (m_libportalSession) {
        xdp_session_close(m_libportalSession);
        g_object_unref(m_libportalSession);
        m_libportalSession = nullptr;
    }
    if (m_libportalPortal) {
        g_object_unref(m_libportalPortal);
        m_libportalPortal = nullptr;
    }
#endif
    m_sessionHandle.clear();
    m_ownsDbusSessionHandle = false;
    markshot::debugLog("screencast", "stop session=%s frames_seen=%d",
                       m_sessionHandle.isEmpty() ? "<closed>" : "closing", m_frameCount);
    m_started = false;
    m_rawRequestedGeometry = {};
    m_rawOutputName.clear();
    m_rawBaseFrameTimeMs = -1;
    m_rawBackpressure = false;
    m_rawDmaBufDirectReadBroken = false;
    m_nodeId = 0;
    m_targetObject.clear();
    m_cursorIncluded = false;
    m_frameCount = 0;
    m_frameErrorCount = 0;
    m_droppedFrameCount = 0;
    QMutexLocker locker(&m_frameMutex);
    m_latestFrame = {};
    m_latestFrameTimeMs = 0;
    m_streamGeometry = {};
    m_negotiatedStreamGeometry = {};
}

bool PortalPipeWireScreencast::start(const CaptureRequest &request, QString *error)
{
    QString busyError;
    if (!markshot::tryAcquireInteractiveScreenCast(&busyError)) {
        if (error) {
            *error = busyError;
        }
        return false;
    }
    struct ScreenCastRelease {
        ~ScreenCastRelease()
        {
            markshot::releaseInteractiveScreenCast();
        }
    };
    [[maybe_unused]] ScreenCastRelease releaseGuard;

    // 1. 【屏幕共享】【应用身份】两种门户路径使用相同应用标识和授权存储
    registerHostPortalApplication();
    m_persistence = std::make_unique<markshot::portal::ScreenCastPersistence>(
        request, m_rawStreamMode.load(std::memory_order_acquire));

#ifdef HAVE_LIBPORTAL
    QString libportalError;
    bool requestSubmitted = false;
    if (startWithLibportal(request.includeCursor, &requestSubmitted, &libportalError)) {
        m_started = true;
        return true;
    }
    markshot::debugLog("screencast", "【屏幕共享】【会话启动】libportal-start-failed error=%s",
                       libportalError.toUtf8().constData());
    // 2. 【屏幕共享】【授权次数】门户请求失败或被取消后，不通过另一套接口再次请求授权
    if (requestSubmitted) {
        stop();
        if (error) {
            *error = libportalError;
        }
        return false;
    }
#endif

    QString dbusError;
    if (!startWithDbusPortal(request.includeCursor, &dbusError)) {
        if (error) {
#ifdef HAVE_LIBPORTAL
            *error = libportalError.isEmpty()
                ? dbusError
                : QStringLiteral("libportal: %1\nD-Bus portal: %2")
                      .arg(libportalError, dbusError);
#else
            *error = dbusError;
#endif
        }
        return false;
    }
    m_started = true;
    return true;
}

#endif  // HAVE_PIPEWIRE
