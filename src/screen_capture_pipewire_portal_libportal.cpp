#include "screen_capture_pipewire_screencast.h"

#include "screen_capture_pipewire_libportal.h"

#ifdef HAVE_PIPEWIRE

#ifdef HAVE_LIBPORTAL
bool PortalPipeWireScreencast::startWithLibportal(bool includeCursor, bool *requestSubmitted, QString *error)
{
    *requestSubmitted = false;

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
                       "【屏幕共享】【会话协商】libportal-start source=monitor cursor_modes=0x%x chosen_cursor=%u",
                       availableCursorModes, cursorMode);

    // 1. 【屏幕共享】【授权恢复】与直接 D-Bus 路径共享版本兼容和令牌消费规则
    const uint portalVersion = portalUintProperty(QStringLiteral("org.freedesktop.portal.ScreenCast"),
                                                  QStringLiteral("version"));
    const QVariantMap persistenceOptions = m_persistence->prepare(portalVersion);
    const QByteArray restoreToken = persistenceOptions.value(QStringLiteral("restore_token")).toString().toUtf8();
    const auto persistMode = static_cast<XdpPersistMode>(
        persistenceOptions.value(QStringLiteral("persist_mode"), uint(0)).toUInt());

    {
        LibportalOperation operation(context, cancellable);
        LibportalCreateCall call;
        call.operation = &operation;
        markshot::debugLog("screencast", "【屏幕共享】【会话协商】libportal-create-session");
        *requestSubmitted = true;
        xdp_portal_create_screencast_session(portal,
                                             XDP_OUTPUT_MONITOR,
                                             XDP_SCREENCAST_FLAG_NONE,
                                             static_cast<XdpCursorMode>(cursorMode),
                                             persistMode,
                                             restoreToken.isEmpty() ? nullptr : restoreToken.constData(),
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
        markshot::debugLog("screencast", "【屏幕共享】【会话协商】libportal-start-session");
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
        // 2. 【屏幕共享】【令牌轮换】成功授权后保存替换令牌，不在日志中输出令牌内容
        char *token = xdp_session_get_restore_token(session);
        m_persistence->accept(token ? QString::fromUtf8(token) : QString());
        g_free(token);
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
                               "【屏幕共享】【会话协商】libportal streams=%s node=%u target_object=%s prop_keys=[%s]",
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

#endif  // HAVE_PIPEWIRE
