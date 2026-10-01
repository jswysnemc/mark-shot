#include "screen_capture_pipewire_screencast.h"

#ifdef HAVE_PIPEWIRE

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
                           "【屏幕共享】【会话协商】screencast create-response keys=[%s] "
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

    markshot::debugLog("capture", "【屏幕共享】【会话协商】screencast session created handle=%s",
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

    // 1. 【屏幕共享】【授权恢复】旧门户保持兼容，新门户申请持久授权并消费上次令牌
    const uint portalVersion = portalUintProperty(screenCastInterface, QStringLiteral("version"));
    QVariantMap selectOptions = m_persistence->prepare(portalVersion);
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
                       "【屏幕共享】【会话协商】screencast negotiate source_types=0x%x cursor_modes=0x%x chosen_cursor=%u",
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

    // 2. 【屏幕共享】【令牌轮换】Start 成功后立即保存新令牌，即使后续 PipeWire 建流失败
    m_persistence->accept(unwrappedVariant(startResponse.value(QStringLiteral("restore_token"))).toString());

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
                           "【屏幕共享】【会话协商】start streams=%d node=%u target_object=%s stream_pos=%s(%d,%d) "
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

#endif  // HAVE_PIPEWIRE
