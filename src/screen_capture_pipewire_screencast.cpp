#include "screen_capture_pipewire_screencast.h"

#include "screen_capture_pipewire_libportal.h"

#ifdef HAVE_PIPEWIRE

PortalPipeWireScreencast::~PortalPipeWireScreencast()
{
    stop();
}

CaptureResult PortalPipeWireScreencast::capture(const CaptureRequest &request)
{
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
        if (!start(request.includeCursor, &error)) {
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
                           "settle-first-frame delay_ms=%lu",
                           kScreencastFirstFrameSettleMs);
        QThread::msleep(kScreencastFirstFrameSettleMs);
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
        const qint64 deadlineMs = QDateTime::currentMSecsSinceEpoch() + 2500;
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

void PortalPipeWireScreencast::stop()
{
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
    m_nodeId = 0;
    m_targetObject.clear();
    m_cursorIncluded = false;
    m_frameCount = 0;
    m_droppedFrameCount = 0;
    QMutexLocker locker(&m_frameMutex);
    m_latestFrame = {};
    m_latestFrameTimeMs = 0;
    m_streamGeometry = {};
}

bool PortalPipeWireScreencast::start(bool includeCursor, QString *error)
{
#ifdef HAVE_LIBPORTAL
    QString libportalError;
    if (startWithLibportal(includeCursor, &libportalError)) {
        m_started = true;
        return true;
    }
    markshot::debugLog("screencast", "libportal-start-failed error=%s",
                       libportalError.toUtf8().constData());
    stop();
#endif

    QString dbusError;
    if (!startWithDbusPortal(includeCursor, &dbusError)) {
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

#ifdef HAVE_LIBPORTAL
bool PortalPipeWireScreencast::startWithLibportal(bool includeCursor, QString *error)
{
    registerHostPortalApplication();

    GMainContext *context = g_main_context_new();
    GCancellable *cancellable = g_cancellable_new();
    g_main_context_push_thread_default(context);

    GError *portalError = nullptr;
    XdpPortal *portal = xdp_portal_initable_new(&portalError);
    if (!portal) {
        if (error) {
            *error = QStringLiteral("failed to initialize libportal: %1")
                         .arg(glibErrorText(portalError));
        }
        if (portalError) {
            g_error_free(portalError);
        }
        g_main_context_pop_thread_default(context);
        g_object_unref(cancellable);
        g_main_context_unref(context);
        return false;
    }

    XdpSession *session = nullptr;
    bool ok = false;
    QString failure;

    const uint availableCursorModes =
        portalUintProperty(QStringLiteral("org.freedesktop.portal.ScreenCast"),
                           QStringLiteral("AvailableCursorModes"));
    uint cursorMode = preferredPortalCursorMode(availableCursorModes, includeCursor);
    if (cursorMode == 0) {
        cursorMode = XDP_CURSOR_MODE_HIDDEN;
    }
    m_cursorIncluded = includeCursor && cursorMode == kPortalCursorEmbedded;
    markshot::debugLog("screencast",
                       "libportal-start source=monitor cursor_modes=0x%x chosen_cursor=%u",
                       availableCursorModes, cursorMode);

    {
        LibportalOperation operation(context, cancellable);
        LibportalCreateCall call;
        call.operation = &operation;
        markshot::debugLog("screencast", "libportal-create-session");
        xdp_portal_create_screencast_session(portal,
                                             XDP_OUTPUT_MONITOR,
                                             XDP_SCREENCAST_FLAG_NONE,
                                             static_cast<XdpCursorMode>(cursorMode),
                                             XDP_PERSIST_MODE_NONE,
                                             nullptr,
                                             cancellable,
                                             onLibportalCreateScreencastFinished,
                                             &call);
        if (!operation.wait(QStringLiteral("libportal CreateScreencast"), &failure)) {
            if (call.error) {
                g_error_free(call.error);
            }
            goto cleanup;
        }
        if (!call.session) {
            failure = QStringLiteral("libportal CreateScreencast failed: %1")
                          .arg(glibErrorText(call.error));
            if (call.error) {
                g_error_free(call.error);
            }
            goto cleanup;
        }
        session = call.session;
        if (call.error) {
            g_error_free(call.error);
        }
    }

    {
        LibportalOperation operation(context, cancellable);
        LibportalStartCall call;
        call.operation = &operation;
        markshot::debugLog("screencast", "libportal-start-session");
        xdp_session_start(session,
                          nullptr,
                          cancellable,
                          onLibportalSessionStartFinished,
                          &call);
        if (!operation.wait(QStringLiteral("libportal Start"), &failure)) {
            if (call.error) {
                g_error_free(call.error);
            }
            goto cleanup;
        }
        if (!call.ok) {
            failure = QStringLiteral("libportal Start failed: %1")
                          .arg(glibErrorText(call.error));
            if (call.error) {
                g_error_free(call.error);
            }
            goto cleanup;
        }
        if (call.error) {
            g_error_free(call.error);
        }
    }

    {
        GVariant *streams = xdp_session_get_streams(session);
        if (!readLibportalStream(streams, &m_nodeId, &m_streamProperties, &failure)) {
            goto cleanup;
        }
        m_targetObject =
            unwrappedVariant(m_streamProperties.value(QStringLiteral("pipewire-serial"))).toString();
        if (m_targetObject.isEmpty()) {
            m_targetObject =
                unwrappedVariant(m_streamProperties.value(QStringLiteral("pipewire.node.serial"))).toString();
        }

        if (markshot::debugEnabled()) {
            gchar *streamsText = streams ? g_variant_print(streams, TRUE) : nullptr;
            QStringList propKeys = m_streamProperties.keys();
            markshot::debugLog("screencast",
                               "libportal streams=%s node=%u target_object=%s prop_keys=[%s]",
                               streamsText ? streamsText : "<null>",
                               m_nodeId,
                               m_targetObject.isEmpty()
                                   ? "<none>"
                                   : m_targetObject.toUtf8().constData(),
                               propKeys.join(QLatin1Char(',')).toUtf8().constData());
            g_free(streamsText);
        }
    }

    {
        const int pipewireFd = xdp_session_open_pipewire_remote(session);
        if (pipewireFd < 0) {
            failure = QStringLiteral("libportal OpenPipeWireRemote returned an invalid fd");
            goto cleanup;
        }

        m_libportalPortal = portal;
        m_libportalSession = session;
        portal = nullptr;
        session = nullptr;
        ok = startPipeWire(pipewireFd, &failure);
        if (!ok) {
            goto cleanup;
        }
    }

cleanup:
    g_main_context_pop_thread_default(context);
    g_object_unref(cancellable);
    g_main_context_unref(context);
    if (!ok) {
        if (session) {
            xdp_session_close(session);
            g_object_unref(session);
        }
        if (portal) {
            g_object_unref(portal);
        }
        if (error) {
            *error = failure.isEmpty()
                ? QStringLiteral("libportal screencast start failed")
                : failure;
        }
        return false;
    }
    return true;
}
#endif

bool PortalPipeWireScreencast::startWithDbusPortal(bool includeCursor, QString *error)
{
    static bool dbusTypesRegistered = [] {
        qRegisterMetaType<PortalStream>("PortalStream");
        qRegisterMetaType<PortalStreamList>("PortalStreamList");
        qDBusRegisterMetaType<PortalStream>();
        qDBusRegisterMetaType<PortalStreamList>();
        return true;
    }();
    Q_UNUSED(dbusTypesRegistered);

    QDBusInterface portal(QStringLiteral("org.freedesktop.portal.Desktop"),
                          QStringLiteral("/org/freedesktop/portal/desktop"),
                          QStringLiteral("org.freedesktop.portal.ScreenCast"),
                          QDBusConnection::sessionBus());
    if (!portal.isValid()) {
        if (error) {
            *error = QStringLiteral("xdg-desktop-portal ScreenCast interface is not available");
        }
        return false;
    }

    QVariantMap options;
    options.insert(QStringLiteral("handle_token"), portalToken());
    options.insert(QStringLiteral("session_handle_token"), portalToken());

    QString requestError;
    const QVariantMap createResponse =
        callPortalRequest(&portal, QStringLiteral("CreateSession"), {options},
                          QStringLiteral("ScreenCast CreateSession"), &requestError);
    if (!requestError.isEmpty()) {
        if (error) {
            *error = requestError;
        }
        return false;
    }

    const QVariant rawSessionHandle =
        createResponse.value(QStringLiteral("session_handle"));
    const QVariant sessionHandleValue = unwrappedVariant(rawSessionHandle);
    if (sessionHandleValue.metaType() == QMetaType::fromType<QDBusObjectPath>()) {
        m_sessionHandle = qvariant_cast<QDBusObjectPath>(sessionHandleValue).path();
    } else {
        m_sessionHandle = sessionHandleValue.toString();
    }
    if (markshot::debugEnabled()) {
        markshot::debugLog("capture",
                           "screencast create-response keys=[%s] "
                           "session_handle type=%s value=%s",
                           createResponse.keys().join(QLatin1Char(',')).toUtf8().constData(),
                           sessionHandleValue.typeName()
                               ? sessionHandleValue.typeName()
                               : "<null>",
                           m_sessionHandle.isEmpty()
                               ? "<empty>"
                               : m_sessionHandle.toUtf8().constData());
    }
    if (m_sessionHandle.isEmpty()) {
        if (error) {
            *error = QStringLiteral("ScreenCast CreateSession returned no session handle");
        }
        return false;
    }
    m_ownsDbusSessionHandle = true;

    markshot::debugLog("capture", "screencast session created handle=%s",
                       m_sessionHandle.toUtf8().constData());

    const QDBusObjectPath sessionPath{m_sessionHandle};
    const QString screenCastInterface = QStringLiteral("org.freedesktop.portal.ScreenCast");
    const uint sourceTypes =
        portalUintProperty(screenCastInterface, QStringLiteral("AvailableSourceTypes"));
    if (sourceTypes != 0 && !(sourceTypes & kPortalSourceMonitor)) {
        if (error) {
            *error = QStringLiteral("ScreenCast portal does not advertise monitor capture");
        }
        return false;
    }

    QVariantMap selectOptions;
    selectOptions.insert(QStringLiteral("handle_token"), portalToken());
    selectOptions.insert(QStringLiteral("types"), kPortalSourceMonitor);
    selectOptions.insert(QStringLiteral("multiple"), false);
    const uint availableCursorModes =
        portalUintProperty(screenCastInterface, QStringLiteral("AvailableCursorModes"));
    const uint cursorMode = preferredPortalCursorMode(availableCursorModes, includeCursor);
    if (cursorMode != 0) {
        selectOptions.insert(QStringLiteral("cursor_mode"), cursorMode);
    }
    m_cursorIncluded = includeCursor && cursorMode == kPortalCursorEmbedded;
    markshot::debugLog("capture",
                       "screencast negotiate source_types=0x%x cursor_modes=0x%x chosen_cursor=%u",
                       sourceTypes, availableCursorModes, cursorMode);
    callPortalRequest(&portal, QStringLiteral("SelectSources"),
                      {QVariant::fromValue(sessionPath), selectOptions},
                      QStringLiteral("ScreenCast SelectSources"), &requestError);
    if (!requestError.isEmpty()) {
        if (error) {
            *error = requestError;
        }
        return false;
    }

    QVariantMap startOptions;
    startOptions.insert(QStringLiteral("handle_token"), portalToken());
    const QVariantMap startResponse =
        callPortalRequest(&portal, QStringLiteral("Start"),
                          {QVariant::fromValue(sessionPath), QString(), startOptions},
                          QStringLiteral("ScreenCast Start"), &requestError);
    if (!requestError.isEmpty()) {
        if (error) {
            *error = requestError;
        }
        return false;
    }

    const PortalStreamList streams =
        qdbus_cast<PortalStreamList>(startResponse.value(QStringLiteral("streams")));
    if (streams.isEmpty() || streams.first().nodeId == 0) {
        if (error) {
            *error = QStringLiteral("ScreenCast Start returned no PipeWire stream");
        }
        return false;
    }
    m_nodeId = streams.first().nodeId;
    m_streamProperties = streams.first().properties;
    m_targetObject =
        unwrappedVariant(m_streamProperties.value(QStringLiteral("pipewire-serial"))).toString();
    if (m_targetObject.isEmpty()) {
        m_targetObject =
            unwrappedVariant(m_streamProperties.value(QStringLiteral("pipewire.node.serial"))).toString();
    }

    if (markshot::debugEnabled()) {
        int sx = 0;
        int sy = 0;
        int sw = 0;
        int sh = 0;
        const bool hasPos =
            readPairVariant(m_streamProperties.value(QStringLiteral("position")), &sx, &sy);
        const bool hasSize =
            readPairVariant(m_streamProperties.value(QStringLiteral("size")), &sw, &sh);
        QStringList propKeys = m_streamProperties.keys();
        markshot::debugLog("screencast",
                           "start streams=%d node=%u target_object=%s stream_pos=%s(%d,%d) "
                           "stream_size=%s(%dx%d) prop_keys=[%s]",
                           static_cast<int>(streams.size()), m_nodeId,
                           m_targetObject.isEmpty() ? "<none>" : m_targetObject.toUtf8().constData(),
                           hasPos ? "yes" : "no", sx, sy,
                           hasSize ? "yes" : "no", sw, sh,
                           propKeys.join(QLatin1Char(',')).toUtf8().constData());
    }

    QDBusPendingReply<QDBusUnixFileDescriptor> pending =
        portal.asyncCall(QStringLiteral("OpenPipeWireRemote"),
                         QVariant::fromValue(sessionPath),
                         QVariantMap());
    QDBusPendingCallWatcher watcher(pending);
    QEventLoop fdLoop;
    QObject::connect(&watcher, &QDBusPendingCallWatcher::finished, &fdLoop, &QEventLoop::quit);
    fdLoop.exec();
    pending = watcher;
    if (pending.isError()) {
        if (error) {
            *error = QStringLiteral("ScreenCast OpenPipeWireRemote: %1").arg(pending.error().message());
        }
        return false;
    }

    const QDBusUnixFileDescriptor descriptor = pending.value();
    if (!descriptor.isValid()) {
        if (error) {
            *error = QStringLiteral("ScreenCast OpenPipeWireRemote returned an invalid fd");
        }
        return false;
    }

    const int pipewireFd = ::dup(descriptor.fileDescriptor());
    if (pipewireFd < 0) {
        if (error) {
            *error = QStringLiteral("failed to duplicate PipeWire fd");
        }
        return false;
    }
    if (!startPipeWire(pipewireFd, error)) {
        return false;
    }

    m_started = true;
    return true;
}

#endif
