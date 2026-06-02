#include "screen_capture.h"

#include "debug_log.h"

#include <QDBusError>
#include <QDBusConnection>
#include <QDBusArgument>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QDBusMetaType>
#include <QDBusObjectPath>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusReply>
#include <QDBusUnixFileDescriptor>
#include <QDBusVariant>
#include <QEventLoop>
#include <QFile>
#include <QGuiApplication>
#include <QMutex>
#include <QMutexLocker>
#include <QObject>
#include <QPoint>
#include <QPixmap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QRect>
#include <QRectF>
#include <QScreen>
#include <QStringList>
#include <QTimer>
#include <QUrl>
#include <QUuid>
#include <QVariantMap>
#include <QWaitCondition>

#include <algorithm>
#include <cmath>
#include <cstring>
#include <memory>

#include <fcntl.h>
#include <poll.h>
#include <unistd.h>

#ifdef HAVE_XCB
#include <xcb/xcb.h>
#endif

#ifdef HAVE_PIPEWIRE
#include <pipewire/pipewire.h>
#include <spa/buffer/buffer.h>
#include <spa/param/format-utils.h>
#include <spa/param/param.h>
#if __has_include(<spa/param/video/raw-utils.h>)
#include <spa/param/video/raw-utils.h>
#else
#include <spa/param/video/format-utils.h>
#endif
#include <spa/pod/builder.h>
#if __has_include(<spa/param/buffers.h>)
#include <spa/param/buffers.h>
#define MARKSHOT_SPA_PARAM_BUFFERS_BUFFERS SPA_PARAM_BUFFERS_buffers
#define MARKSHOT_SPA_PARAM_BUFFERS_BLOCKS SPA_PARAM_BUFFERS_blocks
#define MARKSHOT_SPA_PARAM_BUFFERS_SIZE SPA_PARAM_BUFFERS_size
#define MARKSHOT_SPA_PARAM_BUFFERS_STRIDE SPA_PARAM_BUFFERS_stride
#define MARKSHOT_SPA_PARAM_BUFFERS_DATA_TYPE SPA_PARAM_BUFFERS_dataType
#else
constexpr int MARKSHOT_SPA_PARAM_BUFFERS_BUFFERS = 1;
constexpr int MARKSHOT_SPA_PARAM_BUFFERS_BLOCKS = 2;
constexpr int MARKSHOT_SPA_PARAM_BUFFERS_SIZE = 3;
constexpr int MARKSHOT_SPA_PARAM_BUFFERS_STRIDE = 4;
constexpr int MARKSHOT_SPA_PARAM_BUFFERS_DATA_TYPE = 6;
#endif
#endif

struct PortalStream {
    uint nodeId = 0;
    QVariantMap properties;
};

using PortalStreamList = QList<PortalStream>;

Q_DECLARE_METATYPE(PortalStream)
Q_DECLARE_METATYPE(PortalStreamList)

QDBusArgument &operator<<(QDBusArgument &argument, const PortalStream &stream)
{
    argument.beginStructure();
    argument << stream.nodeId << stream.properties;
    argument.endStructure();
    return argument;
}

const QDBusArgument &operator>>(const QDBusArgument &argument, PortalStream &stream)
{
    argument.beginStructure();
    argument >> stream.nodeId >> stream.properties;
    argument.endStructure();
    return argument;
}

namespace {

constexpr uint kPortalSourceMonitor = 1u;
constexpr uint kPortalCursorHidden = 1u;
constexpr uint kPortalCursorEmbedded = 2u;
constexpr uint kPortalCursorMetadata = 4u;

class PortalResponseReceiver : public QObject {
    Q_OBJECT

public:
    bool received = false;
    uint response = 2;
    QVariantMap results;

public slots:
    void handleResponse(uint responseCode, const QVariantMap &responseResults)
    {
        received = true;
        response = responseCode;
        results = responseResults;
        emit finished();
    }

signals:
    void finished();
};

bool isWaylandSession()
{
    const QString sessionType = QProcessEnvironment::systemEnvironment().value(QStringLiteral("XDG_SESSION_TYPE")).toLower();
    return sessionType == QStringLiteral("wayland");
}

QString desktopEnvironmentText()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    return (env.value(QStringLiteral("XDG_CURRENT_DESKTOP")) + QLatin1Char(':')
            + env.value(QStringLiteral("XDG_SESSION_DESKTOP")) + QLatin1Char(':')
            + env.value(QStringLiteral("DESKTOP_SESSION")) + QLatin1Char(':')
            + env.value(QStringLiteral("WAYLAND_DISPLAY")))
        .toLower();
}

bool prefersGrim()
{
    const QString desktop = desktopEnvironmentText();
    return desktop.contains(QStringLiteral("sway"))
        || desktop.contains(QStringLiteral("hyprland"))
        || desktop.contains(QStringLiteral("niri"))
        || desktop.contains(QStringLiteral("river"))
        || desktop.contains(QStringLiteral("wayfire"))
        || desktop.contains(QStringLiteral("labwc"))
        || desktop.contains(QStringLiteral("wlroots"));
}

// KWin exposes org.kde.KWin.ScreenShot2, which can capture an exact pixel
// region server-side with no portal dialog. That is the right backend for live
// scrolling capture on Plasma; the portal ScreenCast path forces a source
// picker and reports a virtual-desktop geometry that does not match KWin's real
// outputs, so its crop math is unreliable here.
bool isKdePlasma()
{
    const QString desktop = desktopEnvironmentText();
    return desktop.contains(QStringLiteral("kde")) || desktop.contains(QStringLiteral("plasma"));
}

QImage normalizeCaptureImage(QImage image)
{
    if (!image.isNull()) {
        image.setDevicePixelRatio(1.0);
    }
    return image;
}

CaptureResult captureWithQScreen(const CaptureRequest &request)
{
    QScreen *screen = nullptr;
    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        screen = QGuiApplication::screenAt(request.sourceGeometry.center());
    }
    if (!screen) {
        screen = QGuiApplication::primaryScreen();
    }
    if (!screen) {
        return {{}, QStringLiteral("no screen available for capture"), {}, {}};
    }

    QPixmap pixmap = screen->grabWindow(0);
    if (pixmap.isNull()) {
        return {{}, QStringLiteral("QScreen::grabWindow returned null pixmap"), {}, {}};
    }

    const QImage rawImage = pixmap.toImage();
    QImage image = rawImage.convertToFormat(QImage::Format_ARGB32_Premultiplied);

    QRect cropRect;
    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        const QRect requested = request.sourceGeometry.normalized();
        const QRect screenGeometry = screen->geometry();
        const QRect overlap = requested.intersected(screenGeometry);
        if (!overlap.isEmpty()) {
            const qreal scaleX = static_cast<qreal>(image.width()) / std::max(1, screenGeometry.width());
            const qreal scaleY = static_cast<qreal>(image.height()) / std::max(1, screenGeometry.height());
            const int left = static_cast<int>(std::lround((overlap.left() - screenGeometry.left()) * scaleX));
            const int top = static_cast<int>(std::lround((overlap.top() - screenGeometry.top()) * scaleY));
            const int right = static_cast<int>(std::lround((overlap.right() + 1 - screenGeometry.left()) * scaleX));
            const int bottom = static_cast<int>(std::lround((overlap.bottom() + 1 - screenGeometry.top()) * scaleY));
            cropRect = QRect(QPoint(left, top), QPoint(right - 1, bottom - 1)).intersected(image.rect());
        }
        if (cropRect.isEmpty()) {
            return {{}, QStringLiteral("QScreen capture does not cover requested geometry"), {}, request.sourceGeometry};
        }
        image = image.copy(cropRect);
    }

    markshot::debugLog("capture",
                       "qscreen frame screen=%s screen_geom=%d,%d %dx%d "
                       "pixmap=%dx%d pixmap_dpr=%.3f image=%dx%d requested=%d,%d %dx%d "
                       "crop=%d,%d %dx%d result=%dx%d",
                       screen->name().toUtf8().constData(),
                       screen->geometry().x(), screen->geometry().y(),
                       screen->geometry().width(), screen->geometry().height(),
                       pixmap.width(), pixmap.height(), pixmap.devicePixelRatioF(),
                       rawImage.width(), rawImage.height(),
                       request.sourceGeometry.x(), request.sourceGeometry.y(),
                       request.sourceGeometry.width(), request.sourceGeometry.height(),
                       cropRect.x(), cropRect.y(), cropRect.width(), cropRect.height(),
                       image.width(), image.height());
    return {image, {}, {}, request.sourceGeometry};
}

QString grimGeometry(QRect geometry)
{
    geometry = geometry.normalized();
    return QStringLiteral("%1,%2 %3x%4")
        .arg(geometry.x())
        .arg(geometry.y())
        .arg(geometry.width())
        .arg(geometry.height());
}

CaptureResult runGrim(const QStringList &arguments, const QString &outputName, QRect sourceGeometry)
{
    QProcess grim;
    grim.setProgram(QStringLiteral("grim"));
    grim.setArguments(arguments);
    grim.start(QIODevice::ReadOnly);

    if (!grim.waitForStarted(3000)) {
        return {{}, QStringLiteral("failed to start grim; install grim and run under a Wayland compositor that supports screencopy"), outputName, sourceGeometry};
    }

    if (!grim.waitForFinished(8000)) {
        grim.kill();
        grim.waitForFinished(1000);
        return {{}, QStringLiteral("grim timed out while capturing the screen"), outputName, sourceGeometry};
    }

    const QByteArray png = grim.readAllStandardOutput();
    const QByteArray stderrText = grim.readAllStandardError();

    if (grim.exitStatus() != QProcess::NormalExit || grim.exitCode() != 0) {
        QString error = QString::fromLocal8Bit(stderrText).trimmed();
        if (error.isEmpty()) {
            error = QStringLiteral("grim failed with exit code %1").arg(grim.exitCode());
        }
        return {{}, error, outputName, sourceGeometry};
    }

    QImage image;
    if (!image.loadFromData(png, "PPM") || image.isNull()) {
        return {{}, QStringLiteral("grim returned invalid PPM data"), outputName, sourceGeometry};
    }

    return {image.convertToFormat(QImage::Format_ARGB32_Premultiplied), {}, outputName, sourceGeometry};
}

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

QRect virtualScreensGeometry()
{
    QRect geometry;
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        geometry = geometry.isNull() ? screen->geometry() : geometry.united(screen->geometry());
    }
    return geometry;
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

        QDBusPendingReply<QDBusObjectPath> pending = portal.asyncCall(QStringLiteral("Screenshot"), QString(), options);
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
        const QRect imageRect(QPoint(0, 0), image.size());
        const QRect requested = request.sourceGeometry.normalized();
        QRect cropRect(QPoint(0, 0), requested.size());
        if (image.size() != requested.size()) {
            const QRect virtualGeometry = virtualScreensGeometry();
            cropRect = requested.translated(-virtualGeometry.topLeft());
        }
        cropRect = cropRect.intersected(imageRect);
        if (!cropRect.isEmpty() && cropRect.size() != image.size()) {
            image = image.copy(cropRect);
            sourceGeometry = requested;
        }
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

uint preferredPortalCursorMode(uint availableModes)
{
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

QImage cropFrameToRequest(const QImage &frame, QRect streamGeometry, QRect requestedGeometry)
{
    if (frame.isNull() || requestedGeometry.isEmpty()) {
        return frame;
    }

    if (streamGeometry.isNull() || streamGeometry.isEmpty()) {
        streamGeometry = QRect(QPoint(0, 0), frame.size());
    }

    const QRect requested = requestedGeometry.normalized();
    const QRect overlap = requested.intersected(streamGeometry);
    if (overlap.isEmpty()) {
        return {};
    }

    const qreal scaleX = static_cast<qreal>(frame.width()) / streamGeometry.width();
    const qreal scaleY = static_cast<qreal>(frame.height()) / streamGeometry.height();
    const int left = qRound((overlap.left() - streamGeometry.left()) * scaleX);
    const int top = qRound((overlap.top() - streamGeometry.top()) * scaleY);
    const int right = qRound((overlap.right() + 1 - streamGeometry.left()) * scaleX);
    const int bottom = qRound((overlap.bottom() + 1 - streamGeometry.top()) * scaleY);
    const QRect crop(left, top, std::max(1, right - left), std::max(1, bottom - top));
    const QRect boundedCrop = crop.intersected(frame.rect());
    return boundedCrop.isEmpty() ? QImage() : frame.copy(boundedCrop);
}

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
            if (!start(&error)) {
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

        QMutexLocker locker(&m_frameMutex);
        bool waited = false;
        if (m_latestFrame.isNull()) {
            waited = true;
            m_frameReady.wait(&m_frameMutex, 2500);
        }
        if (m_latestFrame.isNull()) {
            const QString error = m_lastError.isEmpty()
                ? QStringLiteral("portal screencast did not produce a frame")
                : QStringLiteral("portal screencast did not produce a frame: %1").arg(m_lastError);
            markshot::debugLog("screencast",
                               "no-frame first_start=%d waited=%d last_error=%s",
                               firstStart ? 1 : 0, waited ? 1 : 0,
                               m_lastError.isEmpty() ? "(none)"
                                                     : m_lastError.toUtf8().constData());
            return {{}, error, {}, request.sourceGeometry};
        }

        const QImage frame = m_latestFrame.copy();
        const QRect streamGeometry = m_streamGeometry;
        locker.unlock();

        const QRect requested = request.sourceGeometry;
        const bool wantCrop = requested.isValid() && !requested.isEmpty();
        QImage image = wantCrop
            ? cropFrameToRequest(frame, streamGeometry, requested)
            : frame;
        markshot::debugLog("screencast",
                           "frame raw=%dx%d stream_geom=%d,%d %dx%d requested=%d,%d %dx%d "
                           "want_crop=%d cropped=%dx%d",
                           frame.width(), frame.height(),
                           streamGeometry.x(), streamGeometry.y(),
                           streamGeometry.width(), streamGeometry.height(),
                           requested.x(), requested.y(), requested.width(), requested.height(),
                           wantCrop ? 1 : 0, image.width(), image.height());
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
        return {image.convertToFormat(QImage::Format_ARGB32_Premultiplied), {}, request.preferredOutputName, request.sourceGeometry};
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
        if (!m_sessionHandle.isEmpty()) {
            QDBusInterface session(QStringLiteral("org.freedesktop.portal.Desktop"),
                                   m_sessionHandle,
                                   QStringLiteral("org.freedesktop.portal.Session"),
                                   QDBusConnection::sessionBus());
            if (session.isValid()) {
                session.call(QStringLiteral("Close"));
            }
            m_sessionHandle.clear();
        }
        markshot::debugLog("screencast", "stop session=%s frames_seen=%d",
                           m_sessionHandle.isEmpty() ? "<closed>" : "closing", m_frameCount);
        m_started = false;
        m_nodeId = 0;
        m_targetObject.clear();
        m_frameCount = 0;
        QMutexLocker locker(&m_frameMutex);
        m_latestFrame = {};
        m_streamGeometry = {};
    }

private:
    bool start(QString *error)
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
        const uint cursorMode = preferredPortalCursorMode(availableCursorModes);
        if (cursorMode != 0) {
            selectOptions.insert(QStringLiteral("cursor_mode"), cursorMode);
        }
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
            QMutexLocker locker(&self->m_frameMutex);
            self->m_latestFrame = std::move(image);
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
    QString m_lastError;
    QVariantMap m_streamProperties;
    QMutex m_frameMutex;
    QWaitCondition m_frameReady;
    QImage m_latestFrame;
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

CaptureResult captureWithPortalScreencast(const CaptureRequest &request)
{
    return {{}, QStringLiteral("PipeWire support was not enabled at build time"), {}, request.sourceGeometry};
}

void stopPortalScreencast()
{
}

#endif

CaptureResult captureWithGrim(const CaptureRequest &request)
{
    const QStringList baseArguments{QStringLiteral("-t"), QStringLiteral("ppm")};

    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        QStringList arguments = baseArguments;
        arguments << QStringLiteral("-g") << grimGeometry(request.sourceGeometry) << QStringLiteral("-");
        CaptureResult geometryCapture = runGrim(arguments, request.allOutputs ? QString() : request.preferredOutputName, request.sourceGeometry);
        if (!geometryCapture.image.isNull() || request.allOutputs || request.preferredOutputName.isEmpty()) {
            return geometryCapture;
        }
    }

    if (!request.allOutputs && !request.preferredOutputName.isEmpty()) {
        QStringList arguments = baseArguments;
        arguments << QStringLiteral("-o") << request.preferredOutputName << QStringLiteral("-");
        return runGrim(arguments, request.preferredOutputName, {});
    }

    QStringList arguments = baseArguments;
    arguments << QStringLiteral("-");
    return runGrim(arguments, {}, {});
}

// KWin's own org.kde.KWin.ScreenShot2.CaptureArea renders the exact requested
// rectangle server-side and streams the raw pixels back over a pipe fd. Unlike
// the xdg-desktop-portal ScreenCast path it needs no source-selection dialog,
// keeps no session, and returns a frame whose size already matches the request
// (so there is no virtual-geometry scaling to get wrong). It requires the
// caller's .desktop file to list org.kde.KWin.ScreenShot2 in
// X-KDE-DBUS-Restricted-Interfaces; without that KWin replies NoAuthorized and
// this returns a null image so the caller can fall back to the portal path.
CaptureResult captureWithKWinScreenShot(const CaptureRequest &request)
{
    const QRect geometry = request.sourceGeometry.normalized();
    if (geometry.isEmpty()) {
        return {{}, QStringLiteral("KWin ScreenShot2 requires a non-empty geometry"), {}, request.sourceGeometry};
    }

    QDBusInterface kwin(QStringLiteral("org.kde.KWin.ScreenShot2"),
                        QStringLiteral("/org/kde/KWin/ScreenShot2"),
                        QStringLiteral("org.kde.KWin.ScreenShot2"),
                        QDBusConnection::sessionBus());
    if (!kwin.isValid()) {
        return {{}, QStringLiteral("org.kde.KWin.ScreenShot2 interface is not available"), {}, request.sourceGeometry};
    }

    int fds[2];
    if (::pipe2(fds, O_CLOEXEC) != 0) {
        return {{}, QStringLiteral("failed to create pipe for KWin ScreenShot2"), {}, request.sourceGeometry};
    }

    QVariantMap options;
    options.insert(QStringLiteral("include-cursor"), false);
    // native-resolution keeps device pixels on HiDPI instead of downscaling to
    // logical size, so the stitched result stays sharp.
    options.insert(QStringLiteral("native-resolution"), true);

    // KWin sends the D-Bus reply with the buffer metadata first, then writes the
    // pixels to the pipe, so this synchronous call does not deadlock even when
    // the image is larger than the pipe buffer.
    QDBusReply<QVariantMap> reply =
        kwin.call(QStringLiteral("CaptureArea"),
                  geometry.x(), geometry.y(),
                  static_cast<uint>(geometry.width()), static_cast<uint>(geometry.height()),
                  options,
                  QVariant::fromValue(QDBusUnixFileDescriptor(fds[1])));
    ::close(fds[1]);

    if (!reply.isValid()) {
        ::close(fds[0]);
        markshot::debugLog("kwin", "capture-area-error geom=%d,%d %dx%d name=%s msg=%s",
                           geometry.x(), geometry.y(), geometry.width(), geometry.height(),
                           reply.error().name().toUtf8().constData(),
                           reply.error().message().toUtf8().constData());
        return {{},
                QStringLiteral("KWin ScreenShot2 CaptureArea failed: %1: %2")
                    .arg(reply.error().name(), reply.error().message()),
                {},
                request.sourceGeometry};
    }

    const QVariantMap results = reply.value();
    const int width = results.value(QStringLiteral("width")).toInt();
    const int height = results.value(QStringLiteral("height")).toInt();
    const int stride = results.value(QStringLiteral("stride")).toInt();
    const uint format = results.value(QStringLiteral("format")).toUInt();
    if (width <= 0 || height <= 0 || stride < width * 4) {
        ::close(fds[0]);
        return {{},
                QStringLiteral("KWin ScreenShot2 returned invalid buffer metadata (%1x%2 stride=%3)")
                    .arg(width).arg(height).arg(stride),
                {},
                request.sourceGeometry};
    }

    const qulonglong total = static_cast<qulonglong>(stride) * static_cast<qulonglong>(height);
    QByteArray buffer(static_cast<int>(total), Qt::Uninitialized);
    qulonglong received = 0;
    while (received < total) {
        struct pollfd pfd { fds[0], POLLIN, 0 };
        const int polled = ::poll(&pfd, 1, 2000);
        if (polled <= 0) {
            break;  // timeout or poll error
        }
        const ssize_t bytes = ::read(fds[0], buffer.data() + received, total - received);
        if (bytes <= 0) {
            break;  // EOF or read error
        }
        received += static_cast<qulonglong>(bytes);
    }
    ::close(fds[0]);

    if (received < total) {
        markshot::debugLog("kwin", "short-read got=%llu want=%llu %dx%d stride=%d",
                           received, total, width, height, stride);
        return {{},
                QStringLiteral("KWin ScreenShot2 delivered a truncated frame (%1/%2 bytes)")
                    .arg(received).arg(total),
                {},
                request.sourceGeometry};
    }

    const QImage::Format imageFormat =
        format != 0 ? static_cast<QImage::Format>(format) : QImage::Format_ARGB32_Premultiplied;
    const QImage view(reinterpret_cast<const uchar *>(buffer.constData()),
                      width, height, stride, imageFormat);
    if (view.isNull()) {
        return {{}, QStringLiteral("KWin ScreenShot2 frame could not be wrapped as an image"), {}, request.sourceGeometry};
    }

    markshot::debugLog("kwin", "capture-area-ok geom=%d,%d %dx%d -> frame=%dx%d stride=%d format=%u",
                       geometry.x(), geometry.y(), geometry.width(), geometry.height(),
                       width, height, stride, format);
    // Detach from the soon-to-be-freed buffer and normalize the format.
    return {view.copy().convertToFormat(QImage::Format_ARGB32_Premultiplied),
            {},
            request.allOutputs ? QString() : request.preferredOutputName,
            request.sourceGeometry};
}

CaptureResult captureWaylandFrame(const CaptureRequest &request)
{
    const bool grimPreferred = prefersGrim();
    markshot::debugLog("capture",
                       "wayland-frame geom=%d,%d %dx%d output=%s all_outputs=%d "
                       "prefer_screencast=%d allow_interactive=%d allow_screenshot_fallback=%d "
                       "prefers_grim=%d desktop=%s",
                       request.sourceGeometry.x(), request.sourceGeometry.y(),
                       request.sourceGeometry.width(), request.sourceGeometry.height(),
                       request.preferredOutputName.toUtf8().constData(),
                       request.allOutputs ? 1 : 0, request.preferScreencast ? 1 : 0,
                       request.allowInteractivePortal ? 1 : 0,
                       request.allowPortalScreenshotFallback ? 1 : 0, grimPreferred ? 1 : 0,
                       desktopEnvironmentText().toUtf8().constData());

    // On KDE, ScreenShot2.CaptureArea can capture the exact requested region
    // without an interactive portal dialog. Prefer it for both the initial
    // single-output screenshot and live scrolling frames; failures fall back to
    // the existing portal/grim routes.
    if (isKdePlasma() && request.sourceGeometry.isValid()
        && !request.sourceGeometry.isEmpty() && !request.allOutputs) {
        markshot::debugLog("capture", "route=kwin-screenshot (KDE Plasma)");
        CaptureResult kwinCapture = captureWithKWinScreenShot(request);
        if (!kwinCapture.image.isNull()) {
            markshot::debugLog("capture", "kwin-screenshot-ok frame=%dx%d",
                               kwinCapture.image.width(), kwinCapture.image.height());
            return kwinCapture;
        }
        markshot::debugLog("capture", "kwin-screenshot-failed (falling back) error=%s",
                           kwinCapture.error.toUtf8().constData());
    }

    if (request.preferScreencast && !grimPreferred) {
        markshot::debugLog("capture", "route=screencast (preferScreencast && !prefersGrim)");
        CaptureResult screencastCapture = captureWithPortalScreencast(request);
        if (!screencastCapture.image.isNull()) {
            markshot::debugLog("capture", "screencast-ok frame=%dx%d",
                               screencastCapture.image.width(), screencastCapture.image.height());
            return screencastCapture;
        }
        markshot::debugLog("capture", "screencast-failed error=%s",
                           screencastCapture.error.toUtf8().constData());
        stopPortalScreencast();

        CaptureResult portalCapture;
        if (request.allowPortalScreenshotFallback) {
            markshot::debugLog("capture", "fallback=portal-screenshot");
            portalCapture = captureWithPortalScreenshot(request);
            if (!portalCapture.image.isNull()) {
                markshot::debugLog("capture", "portal-screenshot-ok frame=%dx%d",
                                   portalCapture.image.width(), portalCapture.image.height());
                return portalCapture;
            }
            markshot::debugLog("capture", "portal-screenshot-failed error=%s",
                               portalCapture.error.toUtf8().constData());
        } else {
            markshot::debugLog("capture", "portal-screenshot-fallback disabled");
        }

        markshot::debugLog("capture", "fallback=grim");
        CaptureResult grimCapture = captureWithGrim(request);
        if (!grimCapture.image.isNull()) {
            markshot::debugLog("capture", "grim-ok frame=%dx%d",
                               grimCapture.image.width(), grimCapture.image.height());
            return grimCapture;
        }
        markshot::debugLog("capture", "grim-failed error=%s all-routes-exhausted",
                           grimCapture.error.toUtf8().constData());
        return {{},
                QStringLiteral("%1\nPortal screenshot fallback: %2\nGrim fallback: %3")
                    .arg(screencastCapture.error,
                         request.allowPortalScreenshotFallback
                             ? portalCapture.error
                             : QStringLiteral("disabled for live scrolling capture"),
                         grimCapture.error),
                {},
                request.sourceGeometry};
    }

    if (prefersGrim()) {
        CaptureResult grimCapture = captureWithGrim(request);
        if (!grimCapture.image.isNull()) {
            return grimCapture;
        }

        CaptureResult portalCapture = captureWithPortalScreenshot(request);
        if (!portalCapture.image.isNull()) {
            return portalCapture;
        }

        return {{}, QStringLiteral("%1\nPortal fallback: %2").arg(grimCapture.error, portalCapture.error), {}, request.sourceGeometry};
    }

    CaptureResult portalCapture = captureWithPortalScreenshot(request);
    if (!portalCapture.image.isNull()) {
        return portalCapture;
    }

    CaptureResult grimCapture = captureWithGrim(request);
    if (!grimCapture.image.isNull()) {
        return grimCapture;
    }

    return {{}, QStringLiteral("%1\nGrim fallback: %2").arg(portalCapture.error, grimCapture.error), {}, request.sourceGeometry};
}

} // namespace

#include "screen_capture.moc"

CaptureResult captureScreenFrame(const CaptureRequest &request)
{
    CaptureResult result = isWaylandSession()
        ? captureWaylandFrame(request)
        : captureWithQScreen(request);
    result.image = normalizeCaptureImage(result.image);
    return result;
}

void stopActiveScreencastCapture()
{
    stopPortalScreencast();
}

QVector<QRect> enumerateX11WindowGeometries()
{
    QVector<QRect> results;

#ifdef HAVE_XCB
    xcb_connection_t *connection = xcb_connect(nullptr, nullptr);
    if (!connection || xcb_connection_has_error(connection)) {
        if (connection) {
            xcb_disconnect(connection);
        }
        return results;
    }

    const xcb_setup_t *setup = xcb_get_setup(connection);
    if (!setup) {
        xcb_disconnect(connection);
        return results;
    }

    xcb_screen_iterator_t screenIter = xcb_setup_roots_iterator(setup);
    if (!screenIter.data) {
        xcb_disconnect(connection);
        return results;
    }

    xcb_window_t root = screenIter.data->root;

    struct WindowInfo {
        xcb_window_t window;
    };

    QVector<xcb_window_t> stack;
    stack.append(root);

    while (!stack.isEmpty()) {
        xcb_window_t parent = stack.takeLast();
        xcb_query_tree_cookie_t treeCookie = xcb_query_tree(connection, parent);
        xcb_query_tree_reply_t *treeReply = xcb_query_tree_reply(connection, treeCookie, nullptr);
        if (!treeReply) {
            continue;
        }

        int childCount = xcb_query_tree_children_length(treeReply);
        xcb_window_t *children = xcb_query_tree_children(treeReply);

        for (int i = 0; i < childCount; ++i) {
            xcb_window_t child = children[i];

            xcb_get_window_attributes_cookie_t attrCookie = xcb_get_window_attributes(connection, child);
            xcb_get_window_attributes_reply_t *attrReply = xcb_get_window_attributes_reply(connection, attrCookie, nullptr);
            if (!attrReply) {
                continue;
            }

            const bool isViewable = (attrReply->map_state == XCB_MAP_STATE_VIEWABLE);
            free(attrReply);

            if (!isViewable) {
                continue;
            }

            xcb_get_geometry_cookie_t geoCookie = xcb_get_geometry(connection, child);
            xcb_get_geometry_reply_t *geoReply = xcb_get_geometry_reply(connection, geoCookie, nullptr);
            if (!geoReply) {
                continue;
            }

            xcb_translate_coordinates_cookie_t transCookie = xcb_translate_coordinates(connection, child, root, 0, 0);
            xcb_translate_coordinates_reply_t *transReply = xcb_translate_coordinates_reply(connection, transCookie, nullptr);

            if (transReply) {
                int x = transReply->dst_x;
                int y = transReply->dst_y;
                int w = geoReply->width;
                int h = geoReply->height;
                free(transReply);

                if (w > 1 && h > 1) {
                    results.append(QRect(x, y, w, h));
                }
            }

            free(geoReply);
            stack.append(child);
        }

        free(treeReply);
    }

    xcb_disconnect(connection);
#endif

    return results;
}
