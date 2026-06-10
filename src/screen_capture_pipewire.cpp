#include "screen_capture_internal.h"
#include "screen_capture_pipewire_libportal.h"

#ifdef HAVE_PIPEWIRE

class PortalPipeWireScreencast {
public:
    ~PortalPipeWireScreencast()
    {
        stop();
    }

    CaptureResult capture(const CaptureRequest &request)
    {
        const bool firstStart = !m_started;
        if (!m_started) {
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

        const QImage frame = m_latestFrame.copy();
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
        return {image.convertToFormat(QImage::Format_ARGB32_Premultiplied),
                {},
                request.preferredOutputName,
                request.sourceGeometry,
                m_cursorIncluded};
    }

    void stop()
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
        QMutexLocker locker(&m_frameMutex);
        m_latestFrame = {};
        m_latestFrameTimeMs = 0;
        m_streamGeometry = {};
    }

private:
    bool start(bool includeCursor, QString *error)
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
    bool startWithLibportal(bool includeCursor, QString *error)
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

    bool startWithDbusPortal(bool includeCursor, QString *error)
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

        // The session handle lives in an a{sv} map, so it arrives wrapped in a
        // QDBusVariant. Per the xdg-desktop-portal spec session_handle is a
        // string ('s'), but some backends have historically returned an object
        // path ('o'); unwrap the variant and accept either type instead of
        // casting straight to QDBusObjectPath (which yields an empty path for the
        // string form, and silently breaks the whole ScreenCast session).
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

        // SelectSources/Start/OpenPipeWireRemote take the session as an object
        // path ('o'), so rebuild one from the (possibly string-typed) handle.
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

    bool startPipeWire(int fd, QString *error)
    {
        pw_init(nullptr, nullptr);

        m_loop = pw_thread_loop_new("mark-shot-screencast", nullptr);
        if (!m_loop) {
            ::close(fd);
            if (error) {
                *error = QStringLiteral("failed to create PipeWire loop");
            }
            return false;
        }

        if (pw_thread_loop_start(m_loop) < 0) {
            ::close(fd);
            if (error) {
                *error = QStringLiteral("failed to start PipeWire loop");
            }
            return false;
        }

        pw_thread_loop_lock(m_loop);
        m_context = pw_context_new(pw_thread_loop_get_loop(m_loop), nullptr, 0);
        if (!m_context) {
            pw_thread_loop_unlock(m_loop);
            ::close(fd);
            if (error) {
                *error = QStringLiteral("failed to create PipeWire context");
            }
            return false;
        }

        m_core = pw_context_connect_fd(m_context, fd, nullptr, 0);
        if (!m_core) {
            pw_thread_loop_unlock(m_loop);
            ::close(fd);
            if (error) {
                *error = QStringLiteral("failed to connect to PipeWire remote");
            }
            return false;
        }

        pw_properties *properties =
            pw_properties_new(PW_KEY_MEDIA_TYPE, "Video",
                              PW_KEY_MEDIA_CATEGORY, "Capture",
                              PW_KEY_MEDIA_ROLE, "Screen",
                              nullptr);
        if (!m_targetObject.isEmpty()) {
            pw_properties_set(properties, PW_KEY_TARGET_OBJECT, m_targetObject.toUtf8().constData());
        }

        m_stream = pw_stream_new(m_core, "mark-shot-screencast", properties);
        if (!m_stream) {
            pw_thread_loop_unlock(m_loop);
            if (error) {
                *error = QStringLiteral("failed to create PipeWire stream");
            }
            return false;
        }

        m_streamEvents = {};
        m_streamEvents.version = PW_VERSION_STREAM_EVENTS;
        m_streamEvents.state_changed = &PortalPipeWireScreencast::onStreamStateChanged;
        m_streamEvents.param_changed = &PortalPipeWireScreencast::onStreamParamChanged;
        m_streamEvents.process = &PortalPipeWireScreencast::onStreamProcess;
        pw_stream_add_listener(m_stream, &m_streamListener, &m_streamEvents, this);

        uint8_t buffer[2048];
        spa_pod_builder builder = SPA_POD_BUILDER_INIT(buffer, sizeof(buffer));
        const spa_pod *params[1];
        params[0] = static_cast<const spa_pod *>(
            spa_pod_builder_add_object(&builder,
                                       SPA_TYPE_OBJECT_Format, SPA_PARAM_EnumFormat,
                                       SPA_FORMAT_mediaType, SPA_POD_Id(SPA_MEDIA_TYPE_video),
                                       SPA_FORMAT_mediaSubtype, SPA_POD_Id(SPA_MEDIA_SUBTYPE_raw),
                                       SPA_FORMAT_VIDEO_format,
                                       SPA_POD_CHOICE_ENUM_Id(6,
                                                              SPA_VIDEO_FORMAT_BGRA,
                                                              SPA_VIDEO_FORMAT_BGRA,
                                                              SPA_VIDEO_FORMAT_BGRx,
                                                              SPA_VIDEO_FORMAT_RGBA,
                                                              SPA_VIDEO_FORMAT_RGBx,
                                                              SPA_VIDEO_FORMAT_ARGB,
                                                              SPA_VIDEO_FORMAT_xRGB)));

        const pw_stream_flags flags = static_cast<pw_stream_flags>(
            PW_STREAM_FLAG_AUTOCONNECT | PW_STREAM_FLAG_MAP_BUFFERS);
        const int targetId = m_targetObject.isEmpty() ? static_cast<int>(m_nodeId) : PW_ID_ANY;
        const int result = pw_stream_connect(m_stream,
                                             PW_DIRECTION_INPUT,
                                             targetId,
                                             flags,
                                             params,
                                             1);
        pw_thread_loop_unlock(m_loop);
        if (result < 0) {
            if (error) {
                *error = QStringLiteral("failed to connect PipeWire stream");
            }
            markshot::debugLog("screencast",
                               "pw-connect-failed result=%d node=%u target_id=%d target_object=%s",
                               result, m_nodeId, targetId,
                               m_targetObject.isEmpty() ? "<none>" : m_targetObject.toUtf8().constData());
            return false;
        }
        markshot::debugLog("screencast",
                           "pw-connect ok node=%u target_id=%d target_object=%s flags=AUTOCONNECT|MAP_BUFFERS",
                           m_nodeId, targetId,
                           m_targetObject.isEmpty() ? "<none>" : m_targetObject.toUtf8().constData());
        return true;
    }

    static const char *streamStateName(pw_stream_state state)
    {
        switch (state) {
        case PW_STREAM_STATE_ERROR:
            return "error";
        case PW_STREAM_STATE_UNCONNECTED:
            return "unconnected";
        case PW_STREAM_STATE_CONNECTING:
            return "connecting";
        case PW_STREAM_STATE_PAUSED:
            return "paused";
        case PW_STREAM_STATE_STREAMING:
            return "streaming";
        }
        return "unknown";
    }

    static void onStreamStateChanged(void *data,
                                     pw_stream_state old,
                                     pw_stream_state state,
                                     const char *error)
    {
        auto *self = static_cast<PortalPipeWireScreencast *>(data);
        if (!self) {
            return;
        }
        markshot::debugLog("screencast", "pw-state %s -> %s%s%s",
                           streamStateName(old), streamStateName(state),
                           error ? " error=" : "", error ? error : "");
        if (state == PW_STREAM_STATE_ERROR && error) {
            QMutexLocker locker(&self->m_frameMutex);
            self->m_lastError = QString::fromUtf8(error);
            self->m_frameReady.wakeAll();
        }
    }

    static void onStreamParamChanged(void *data, uint32_t id, const spa_pod *param)
    {
        auto *self = static_cast<PortalPipeWireScreencast *>(data);
        if (!self || id != SPA_PARAM_Format || !param) {
            return;
        }

        spa_video_info_raw info = {};
        if (spa_format_video_raw_parse(param, &info) < 0
            || info.size.width == 0 || info.size.height == 0) {
            markshot::debugLog("screencast",
                               "pw-param-changed rejected (parse failed or zero size)");
            return;
        }

        self->m_videoInfo = info;
        const uint32_t stride = info.size.width * 4;
        const uint32_t size = stride * info.size.height;

        markshot::debugLog("screencast",
                           "pw-param-changed format=%d size=%ux%u stride=%u "
                           "framerate=%u/%u",
                           static_cast<int>(info.format), info.size.width, info.size.height,
                           stride, info.max_framerate.num, info.max_framerate.denom);

        uint8_t buffer[1024];
        spa_pod_builder builder = SPA_POD_BUILDER_INIT(buffer, sizeof(buffer));
        const spa_pod *params[1];
        params[0] = static_cast<const spa_pod *>(
            spa_pod_builder_add_object(&builder,
                                       SPA_TYPE_OBJECT_ParamBuffers, SPA_PARAM_Buffers,
                                       MARKSHOT_SPA_PARAM_BUFFERS_BUFFERS, SPA_POD_CHOICE_RANGE_Int(8, 2, 16),
                                       MARKSHOT_SPA_PARAM_BUFFERS_BLOCKS, SPA_POD_Int(1),
                                       MARKSHOT_SPA_PARAM_BUFFERS_SIZE, SPA_POD_Int(size),
                                       MARKSHOT_SPA_PARAM_BUFFERS_STRIDE, SPA_POD_Int(stride),
                                       MARKSHOT_SPA_PARAM_BUFFERS_DATA_TYPE,
                                       SPA_POD_CHOICE_FLAGS_Int((1u << SPA_DATA_MemPtr)
                                                                | (1u << SPA_DATA_MemFd))));
        pw_stream_update_params(self->m_stream, params, 1);
    }

    static void onStreamProcess(void *data)
    {
        auto *self = static_cast<PortalPipeWireScreencast *>(data);
        if (!self || !self->m_stream) {
            return;
        }

        pw_buffer *buffer = nullptr;
        while (pw_buffer *next = pw_stream_dequeue_buffer(self->m_stream)) {
            if (buffer) {
                pw_stream_queue_buffer(self->m_stream, buffer);
            }
            buffer = next;
        }
        if (!buffer) {
            return;
        }

        QString imageError;
        QImage image = self->imageFromBuffer(buffer, &imageError);
        if (!image.isNull()) {
            const qint64 frameTimeMs = QDateTime::currentMSecsSinceEpoch();
            QMutexLocker locker(&self->m_frameMutex);
            self->m_latestFrame = std::move(image);
            self->m_latestFrameTimeMs = frameTimeMs;
            self->m_streamGeometry = streamGeometryFromProperties(self->m_streamProperties, self->m_latestFrame.size());
            self->m_frameReady.wakeAll();
            self->m_frameCount += 1;
            // Per-frame logging would flood the log at the 45ms capture cadence;
            // record only the first frame and then every 100th so the stream stays
            // observable without drowning the stitch trace.
            if (self->m_frameCount == 1 || self->m_frameCount % 100 == 0) {
                markshot::debugLog("screencast",
                                   "pw-frame #%d image=%dx%d stream_geom=%d,%d %dx%d",
                                   self->m_frameCount,
                                   self->m_latestFrame.width(), self->m_latestFrame.height(),
                                   self->m_streamGeometry.x(), self->m_streamGeometry.y(),
                                   self->m_streamGeometry.width(), self->m_streamGeometry.height());
            }
        } else if (!imageError.isEmpty()) {
            QMutexLocker locker(&self->m_frameMutex);
            self->m_lastError = imageError;
            self->m_frameReady.wakeAll();
            markshot::debugLog("screencast", "pw-frame-error %s",
                               imageError.toUtf8().constData());
        }
        pw_stream_queue_buffer(self->m_stream, buffer);
    }

    QImage imageFromBuffer(pw_buffer *pipewireBuffer, QString *error) const
    {
        if (!pipewireBuffer || !pipewireBuffer->buffer || pipewireBuffer->buffer->n_datas == 0) {
            if (error) {
                *error = QStringLiteral("PipeWire delivered an empty buffer");
            }
            return {};
        }
        const spa_buffer *spaBuffer = pipewireBuffer->buffer;
        const spa_data &data = spaBuffer->datas[0];
        if (!data.data || !data.chunk) {
            if (error) {
                *error = QStringLiteral("PipeWire buffer is not CPU-mappable (data type %1)")
                             .arg(static_cast<uint>(data.type));
            }
            return {};
        }

        const int width = static_cast<int>(m_videoInfo.size.width);
        const int height = static_cast<int>(m_videoInfo.size.height);
        if (width <= 0 || height <= 0) {
            if (error) {
                *error = QStringLiteral("PipeWire frame size is invalid");
            }
            return {};
        }

        const int stride = data.chunk->stride != 0
            ? std::abs(static_cast<int>(data.chunk->stride))
            : width * 4;
        const uchar *source = static_cast<const uchar *>(data.data) + data.chunk->offset;
        if (!source || stride < width * 4) {
            if (error) {
                *error = QStringLiteral("PipeWire frame stride is invalid");
            }
            return {};
        }

        switch (m_videoInfo.format) {
        case SPA_VIDEO_FORMAT_BGRA:
            return QImage(source, width, height, stride, QImage::Format_ARGB32).copy()
                .convertToFormat(QImage::Format_ARGB32_Premultiplied);
        case SPA_VIDEO_FORMAT_BGRx:
        case SPA_VIDEO_FORMAT_xRGB:
            return QImage(source, width, height, stride, QImage::Format_RGB32).copy()
                .convertToFormat(QImage::Format_ARGB32_Premultiplied);
        case SPA_VIDEO_FORMAT_RGBA:
        case SPA_VIDEO_FORMAT_RGBx:
        case SPA_VIDEO_FORMAT_ARGB:
            return convertRgbaLikeFrame(source, width, height, stride, m_videoInfo.format);
        default:
            if (error) {
                *error = QStringLiteral("unsupported PipeWire video format %1")
                             .arg(static_cast<int>(m_videoInfo.format));
            }
            return {};
        }
    }

    static QImage convertRgbaLikeFrame(const uchar *source,
                                       int width,
                                       int height,
                                       int stride,
                                       spa_video_format format)
    {
        QImage image(width, height, QImage::Format_ARGB32_Premultiplied);
        for (int y = 0; y < height; ++y) {
            const uchar *src = source + y * stride;
            QRgb *dst = reinterpret_cast<QRgb *>(image.scanLine(y));
            for (int x = 0; x < width; ++x) {
                const uchar *px = src + x * 4;
                int r = 0;
                int g = 0;
                int b = 0;
                int a = 255;
                if (format == SPA_VIDEO_FORMAT_ARGB) {
                    a = px[0];
                    r = px[1];
                    g = px[2];
                    b = px[3];
                } else {
                    r = px[0];
                    g = px[1];
                    b = px[2];
                    if (format == SPA_VIDEO_FORMAT_RGBA) {
                        a = px[3];
                    }
                }
                dst[x] = qRgba(std::min(r, a), std::min(g, a), std::min(b, a), a);
            }
        }
        return image;
    }

    bool m_started = false;
    uint m_nodeId = 0;
    QString m_targetObject;
    QString m_sessionHandle;
    bool m_ownsDbusSessionHandle = false;
    bool m_cursorIncluded = false;
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
    int m_frameCount = 0;  // process-callback frames seen; gates first-frame logging
};

std::unique_ptr<PortalPipeWireScreencast> &activeScreencast()
{
    static std::unique_ptr<PortalPipeWireScreencast> screencast;
    return screencast;
}

CaptureResult captureWithPortalScreencast(const CaptureRequest &request)
{
    std::unique_ptr<PortalPipeWireScreencast> &screencast = activeScreencast();
    if (!screencast) {
        screencast = std::make_unique<PortalPipeWireScreencast>();
    }
    return screencast->capture(request);
}

void stopPortalScreencast()
{
    activeScreencast().reset();
}

#else

/// @brief Captures the screen using the portal screencast service.
/// @param request The capture request details.
/// @return The result of the capture operation.
CaptureResult captureWithPortalScreencast(const CaptureRequest &request)
{
    return {{}, QStringLiteral("PipeWire support was not enabled at build time"), {}, request.sourceGeometry};
}

/// @brief Stops the active portal screen cast session.
void stopPortalScreencast()
{
}

#endif
