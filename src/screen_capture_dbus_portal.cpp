#include "screen_capture_internal.h"

#ifdef MARK_SHOT_WITH_DBUS


QString portalToken()
{
    QString token = QUuid::createUuid().toString(QUuid::WithoutBraces);
    token.replace(QLatin1Char('-'), QLatin1Char('_'));
    return QStringLiteral("mark_shot_%1").arg(token);
}

QString portalRequestPath(const QString &handleToken)
{
    const QString connectionName = QDBusConnection::sessionBus().baseService().mid(1).replace(QLatin1Char('.'), QLatin1Char('_'));
    return QStringLiteral("/org/freedesktop/portal/desktop/request/%1/%2").arg(connectionName, handleToken);
}

bool connectPortalResponse(const QString &signalPath, PortalResponseReceiver *receiver)
{
    return QDBusConnection::sessionBus().connect(QStringLiteral("org.freedesktop.portal.Desktop"),
                                                 signalPath,
                                                 QStringLiteral("org.freedesktop.portal.Request"),
                                                 QStringLiteral("Response"),
                                                 receiver,
                                                 SLOT(handleResponse(uint,QVariantMap)));
}

void disconnectPortalResponse(const QString &signalPath, PortalResponseReceiver *receiver)
{
    QDBusConnection::sessionBus().disconnect(QStringLiteral("org.freedesktop.portal.Desktop"),
                                             signalPath,
                                             QStringLiteral("org.freedesktop.portal.Request"),
                                             QStringLiteral("Response"),
                                             receiver,
                                             SLOT(handleResponse(uint,QVariantMap)));
}

QString portalScreenshotParentWindow(QWidget *parentDummy)
{
    if (QGuiApplication::platformName().compare(QStringLiteral("wayland"), Qt::CaseInsensitive) == 0) {
        return QStringLiteral("wayland:");
    }

    parentDummy->setAttribute(Qt::WA_DontShowOnScreen, true);
    parentDummy->resize(1, 1);
    parentDummy->show();
    return QStringLiteral("x11:0x%1").arg(parentDummy->winId(), 0, 16);
}

void registerHostPortalApplication()
{
    static QMutex mutex;
    static bool attempted = false;

    QMutexLocker locker(&mutex);
    if (attempted) {
        return;
    }
    attempted = true;

    if (QFile::exists(QStringLiteral("/.flatpak-info")) || qEnvironmentVariableIsSet("SNAP")) {
        return;
    }

    const QString desktopFileName = QGuiApplication::desktopFileName();
    if (desktopFileName.isEmpty()) {
        return;
    }

    QDBusMessage message = QDBusMessage::createMethodCall(QStringLiteral("org.freedesktop.portal.Desktop"),
                                                          QStringLiteral("/org/freedesktop/portal/desktop"),
                                                          QStringLiteral("org.freedesktop.host.portal.Registry"),
                                                          QStringLiteral("Register"));
    message << desktopFileName << QVariantMap();

    const QDBusMessage reply = QDBusConnection::sessionBus().call(message, QDBus::Block, 3000);
    if (reply.type() == QDBusMessage::ErrorMessage) {
        const QDBusError error(reply);
        if (error.type() == QDBusError::UnknownInterface || error.type() == QDBusError::UnknownMethod) {
            return;
        }
        if (error.name() == QStringLiteral("org.freedesktop.portal.Error.Failed")
            && error.message().contains(QStringLiteral("Connection already associated"))) {
            return;
        }
    }
}

QVariantMap waitForPortalResponse(PortalResponseReceiver *receiver, QString *error)
{
    if (receiver->received) {
        return receiver->response == 0 ? receiver->results : QVariantMap();
    }

    QEventLoop loop;
    QTimer timeout;
    timeout.setSingleShot(true);
    QObject::connect(receiver, &PortalResponseReceiver::finished, &loop, &QEventLoop::quit);
    QObject::connect(&timeout, &QTimer::timeout, &loop, &QEventLoop::quit);
    timeout.start(120000);
    loop.exec();

    if (!receiver->received) {
        if (error) {
            *error = QStringLiteral("xdg-desktop-portal screenshot request timed out");
        }
        return {};
    }

    if (receiver->response != 0) {
        if (error) {
            *error = receiver->response == 1
                ? QStringLiteral("screenshot request was cancelled")
                : QStringLiteral("screenshot request failed with response code %1").arg(receiver->response);
        }
        return {};
    }

    return receiver->results;
}

CaptureResult captureWithPortalScreenshot(const CaptureRequest &request)
{
    registerHostPortalApplication();

    QDBusInterface portal(QStringLiteral("org.freedesktop.portal.Desktop"),
                          QStringLiteral("/org/freedesktop/portal/desktop"),
                          QStringLiteral("org.freedesktop.portal.Screenshot"),
                          QDBusConnection::sessionBus());
    if (!portal.isValid()) {
        return {{}, QStringLiteral("xdg-desktop-portal Screenshot interface is not available"), {}, request.sourceGeometry};
    }

    auto requestScreenshot = [&portal](bool interactive, bool *userCancelled, QString *error) -> QVariantMap {
        if (userCancelled) {
            *userCancelled = false;
        }
        const QString handleToken = portalToken();
        const QString expectedSignalPath = portalRequestPath(handleToken);
        PortalResponseReceiver receiver;
        if (!connectPortalResponse(expectedSignalPath, &receiver)) {
            if (error) {
                *error = QStringLiteral("failed to connect to xdg-desktop-portal response signal");
            }
            return {};
        }

        QVariantMap options;
        options.insert(QStringLiteral("handle_token"), handleToken);
        options.insert(QStringLiteral("interactive"), interactive);
        options.insert(QStringLiteral("modal"), true);

        QWidget parentDummy;
        const QString parentWindow = portalScreenshotParentWindow(&parentDummy);
        QDBusPendingReply<QDBusObjectPath> pending = portal.asyncCall(QStringLiteral("Screenshot"), parentWindow, options);
        QDBusPendingCallWatcher watcher(pending);
        QEventLoop callLoop;
        QObject::connect(&watcher, &QDBusPendingCallWatcher::finished, &callLoop, &QEventLoop::quit);
        callLoop.exec();

        pending = watcher;
        if (pending.isError()) {
            disconnectPortalResponse(expectedSignalPath, &receiver);
            if (error) {
                *error = pending.error().message();
            }
            return {};
        }

        const QString returnedSignalPath = pending.value().path();
        if (returnedSignalPath != expectedSignalPath && !receiver.received) {
            connectPortalResponse(returnedSignalPath, &receiver);
        }

        QString waitError;
        const QVariantMap result = waitForPortalResponse(&receiver, &waitError);
        disconnectPortalResponse(expectedSignalPath, &receiver);
        if (returnedSignalPath != expectedSignalPath) {
            disconnectPortalResponse(returnedSignalPath, &receiver);
        }
        if (userCancelled && receiver.received && receiver.response == 1) {
            *userCancelled = true;
        }
        if (error) {
            *error = waitError;
        }
        return result;
    };

    // GNOME refuses non-interactive screenshots when the app is launched from
    // a .desktop entry (the portal sees a confined app_id without a stored
    // permission and returns response code 2), while a shell-launched process
    // runs as the trusted host and succeeds silently. Try the quiet path first,
    // then fall back to an interactive request so the portal can prompt for
    // access. A genuine user cancellation is not retried.
    bool userCancelled = false;
    QString error;
    QVariantMap response = requestScreenshot(false, &userCancelled, &error);
    if (response.isEmpty() && !userCancelled && request.allowInteractivePortal) {
        QString interactiveError;
        bool interactiveCancelled = false;
        const QVariantMap interactiveResponse = requestScreenshot(true, &interactiveCancelled, &interactiveError);
        if (!interactiveResponse.isEmpty()) {
            response = interactiveResponse;
            error.clear();
        } else if (!interactiveCancelled && !interactiveError.isEmpty()) {
            error = interactiveError;
        }
    }
    if (response.isEmpty()) {
        return {{}, error.isEmpty() ? QStringLiteral("screenshot request failed") : error, {}, request.sourceGeometry};
    }

    const QUrl screenshotUrl(response.value(QStringLiteral("uri")).toString());
    if (!screenshotUrl.isLocalFile()) {
        return {{}, QStringLiteral("xdg-desktop-portal returned a non-local screenshot URI"), {}, request.sourceGeometry};
    }

    const QString screenshotPath = screenshotUrl.toLocalFile();
    QImage image;
    if (!image.load(screenshotPath) || image.isNull()) {
        return {{}, QStringLiteral("failed to load portal screenshot: %1").arg(screenshotPath), {}, request.sourceGeometry};
    }
    QFile::remove(screenshotPath);

    image = image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    QRect sourceGeometry = request.sourceGeometry;
    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty() && !request.allOutputs) {
        const QRect requested = request.sourceGeometry.normalized();
        const QRect virtualGeometry = virtualScreensGeometry();
        const QRect coverage = virtualGeometry.isEmpty() ? requested : virtualGeometry;
        const QRect overlap = requested.intersected(coverage);
        if (overlap.isEmpty()) {
            return {{}, QStringLiteral("portal screenshot does not cover requested geometry"), {}, request.sourceGeometry};
        }

        const QSize rawSize = image.size();
        QRect cropRect(QPoint(0, 0), image.size());
        if (rawSize != overlap.size()) {
            cropRect = markshot::capture::scaledCropRect(coverage, overlap, rawSize);
        }
        if (cropRect.isEmpty()) {
            return {{}, QStringLiteral("portal screenshot physical crop is empty"), {}, request.sourceGeometry};
        }

        if (cropRect != QRect(QPoint(0, 0), image.size())) {
            image = image.copy(cropRect);
        }
        const QSize croppedSize = image.size();
        sourceGeometry = overlap;
        markshot::debugLog("capture",
                           "portal-screenshot raw=%dx%d coverage=%d,%d %dx%d requested=%d,%d %dx%d "
                           "crop=%d,%d %dx%d result=%dx%d display_logical=%dx%d",
                           rawSize.width(), rawSize.height(),
                           coverage.x(), coverage.y(), coverage.width(), coverage.height(),
                           requested.x(), requested.y(), requested.width(), requested.height(),
                           cropRect.x(), cropRect.y(), cropRect.width(), cropRect.height(),
                           croppedSize.width(), croppedSize.height(),
                           overlap.width(), overlap.height());
    }

    return {image, {}, request.allOutputs ? QString() : request.preferredOutputName, sourceGeometry};
}

QVariantMap callPortalRequest(QDBusInterface *portal,
                              const QString &method,
                              const QVariantList &arguments,
                              const QString &errorPrefix,
                              QString *error)
{
    if (!portal || !portal->isValid()) {
        if (error) {
            *error = QStringLiteral("%1 interface is not available").arg(errorPrefix);
        }
        return {};
    }

    QString expectedSignalPath;
    for (auto it = arguments.crbegin(); it != arguments.crend(); ++it) {
        const QVariantMap options = it->toMap();
        const QString handleToken = options.value(QStringLiteral("handle_token")).toString();
        if (!handleToken.isEmpty()) {
            expectedSignalPath = portalRequestPath(handleToken);
            break;
        }
    }

    PortalResponseReceiver receiver;
    if (!expectedSignalPath.isEmpty()
        && !connectPortalResponse(expectedSignalPath, &receiver)) {
        if (error) {
            *error = QStringLiteral("%1: failed to connect to xdg-desktop-portal response signal").arg(errorPrefix);
        }
        return {};
    }

    QDBusPendingReply<QDBusObjectPath> pending = portal->asyncCallWithArgumentList(method, arguments);
    QDBusPendingCallWatcher watcher(pending);
    QEventLoop callLoop;
    QObject::connect(&watcher, &QDBusPendingCallWatcher::finished, &callLoop, &QEventLoop::quit);
    callLoop.exec();

    pending = watcher;
    if (pending.isError()) {
        if (!expectedSignalPath.isEmpty()) {
            disconnectPortalResponse(expectedSignalPath, &receiver);
        }
        if (error) {
            *error = QStringLiteral("%1: %2").arg(errorPrefix, pending.error().message());
        }
        return {};
    }

    const QString returnedSignalPath = pending.value().path();
    if (expectedSignalPath.isEmpty()) {
        // No handle_token was present in the arguments, so the request object
        // path could not be predicted up front; subscribe now to the path the
        // portal actually returned.
        expectedSignalPath = returnedSignalPath;
        if (!receiver.received
            && !connectPortalResponse(expectedSignalPath, &receiver)) {
            if (error) {
                *error = QStringLiteral("%1: failed to connect to xdg-desktop-portal response signal").arg(errorPrefix);
            }
            return {};
        }
    } else if (returnedSignalPath != expectedSignalPath && !receiver.received) {
        // The portal used a different request path than predicted; also listen
        // on the real one so the response is not missed. Re-subscribing the
        // already-connected predicted path here would make QDBusConnection
        // reject the duplicate and report a spurious connection failure.
        connectPortalResponse(returnedSignalPath, &receiver);
    }

    QString responseError;
    const QVariantMap response = waitForPortalResponse(&receiver, &responseError);
    disconnectPortalResponse(expectedSignalPath, &receiver);
    if (returnedSignalPath != expectedSignalPath) {
        disconnectPortalResponse(returnedSignalPath, &receiver);
    }
    if (!responseError.isEmpty()) {
        if (error) {
            *error = QStringLiteral("%1: %2").arg(errorPrefix, responseError);
        }
        return {};
    }
    return response;
}

bool readPairVariant(const QVariant &value, int *first, int *second)
{
    if (!first || !second || !value.isValid()) {
        return false;
    }

    if (value.canConvert<QDBusVariant>()) {
        const QVariant nested = qvariant_cast<QDBusVariant>(value).variant();
        if (nested.isValid() && nested != value) {
            return readPairVariant(nested, first, second);
        }
    }

    if (value.metaType() == QMetaType::fromType<QDBusArgument>()) {
        const QDBusArgument argument = qvariant_cast<QDBusArgument>(value);
        argument.beginStructure();
        argument >> *first >> *second;
        argument.endStructure();
        return true;
    }

    const QVariantList list = value.toList();
    if (list.size() >= 2) {
        bool okFirst = false;
        bool okSecond = false;
        const int parsedFirst = list.at(0).toInt(&okFirst);
        const int parsedSecond = list.at(1).toInt(&okSecond);
        if (okFirst && okSecond) {
            *first = parsedFirst;
            *second = parsedSecond;
            return true;
        }
    }
    return false;
}

QVariant unwrappedVariant(QVariant value)
{
    while (value.canConvert<QDBusVariant>()) {
        const QVariant nested = qvariant_cast<QDBusVariant>(value).variant();
        if (!nested.isValid() || nested == value) {
            break;
        }
        value = nested;
    }
    return value;
}

uint portalUintProperty(const QString &interfaceName, const QString &propertyName)
{
    QDBusMessage message = QDBusMessage::createMethodCall(QStringLiteral("org.freedesktop.portal.Desktop"),
                                                          QStringLiteral("/org/freedesktop/portal/desktop"),
                                                          QStringLiteral("org.freedesktop.DBus.Properties"),
                                                          QStringLiteral("Get"));
    message << interfaceName << propertyName;

    const QDBusMessage reply = QDBusConnection::sessionBus().call(message, QDBus::Block, 3000);
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
        return 0;
    }

    bool ok = false;
    const uint value = unwrappedVariant(reply.arguments().first()).toUInt(&ok);
    return ok ? value : 0;
}

uint preferredPortalCursorMode(uint availableModes, bool includeCursor)
{
    if (includeCursor) {
        if (availableModes & kPortalCursorEmbedded) {
            return kPortalCursorEmbedded;
        }
        if (availableModes & kPortalCursorHidden) {
            return kPortalCursorHidden;
        }
        return 0;
    }
    if (availableModes & kPortalCursorHidden) {
        return kPortalCursorHidden;
    }
    if (availableModes & kPortalCursorEmbedded) {
        return kPortalCursorEmbedded;
    }
    if (availableModes & kPortalCursorMetadata) {
        return kPortalCursorMetadata;
    }
    return 0;
}

QRect streamGeometryFromProperties(const QVariantMap &properties, const QSize &frameSize)
{
    int x = 0;
    int y = 0;
    int w = frameSize.width();
    int h = frameSize.height();
    const bool hasPosition = readPairVariant(properties.value(QStringLiteral("position")), &x, &y);
    const bool hasSize = readPairVariant(properties.value(QStringLiteral("size")), &w, &h);
    if (hasPosition && hasSize && w > 0 && h > 0) {
        return QRect(x, y, w, h);
    }
    return virtualScreensGeometry();
}

#endif
