#include "screen_capture.h"

#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusObjectPath>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QEventLoop>
#include <QFile>
#include <QGuiApplication>
#include <QObject>
#include <QPixmap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QRect>
#include <QScreen>
#include <QStringList>
#include <QTimer>
#include <QUrl>
#include <QUuid>
#include <QVariantMap>

#ifdef HAVE_XCB
#include <xcb/xcb.h>
#endif

namespace {

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

CaptureResult captureWithQScreen(const CaptureRequest &request)
{
    QScreen *screen = QGuiApplication::primaryScreen();
    if (!screen) {
        return {{}, QStringLiteral("no screen available for capture"), {}, {}};
    }

    QPixmap pixmap = screen->grabWindow(0);
    if (pixmap.isNull()) {
        return {{}, QStringLiteral("QScreen::grabWindow returned null pixmap"), {}, {}};
    }

    QImage image = pixmap.toImage().convertToFormat(QImage::Format_ARGB32_Premultiplied);

    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        const QRect geo = request.sourceGeometry.normalized().intersected(image.rect());
        if (!geo.isEmpty()) {
            image = image.copy(geo);
        }
    }

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
    QDBusInterface portal(QStringLiteral("org.freedesktop.portal.Desktop"),
                          QStringLiteral("/org/freedesktop/portal/desktop"),
                          QStringLiteral("org.freedesktop.portal.Screenshot"),
                          QDBusConnection::sessionBus());
    if (!portal.isValid()) {
        return {{}, QStringLiteral("xdg-desktop-portal Screenshot interface is not available"), {}, request.sourceGeometry};
    }

    const QString handleToken = portalToken();
    const QString expectedSignalPath = portalRequestPath(handleToken);
    PortalResponseReceiver receiver;
    if (!connectPortalResponse(expectedSignalPath, &receiver)) {
        return {{}, QStringLiteral("failed to connect to xdg-desktop-portal response signal"), {}, request.sourceGeometry};
    }

    QVariantMap options;
    options.insert(QStringLiteral("handle_token"), handleToken);
    options.insert(QStringLiteral("interactive"), false);
    options.insert(QStringLiteral("modal"), true);

    QDBusPendingReply<QDBusObjectPath> pending = portal.asyncCall(QStringLiteral("Screenshot"), QString(), options);
    QDBusPendingCallWatcher watcher(pending);
    QEventLoop callLoop;
    QObject::connect(&watcher, &QDBusPendingCallWatcher::finished, &callLoop, &QEventLoop::quit);
    callLoop.exec();

    pending = watcher;
    if (pending.isError()) {
        disconnectPortalResponse(expectedSignalPath, &receiver);
        return {{}, pending.error().message(), {}, request.sourceGeometry};
    }

    const QString returnedSignalPath = pending.value().path();
    if (returnedSignalPath != expectedSignalPath && !receiver.received) {
        connectPortalResponse(returnedSignalPath, &receiver);
    }

    QString error;
    const QVariantMap response = waitForPortalResponse(&receiver, &error);
    disconnectPortalResponse(expectedSignalPath, &receiver);
    if (returnedSignalPath != expectedSignalPath) {
        disconnectPortalResponse(returnedSignalPath, &receiver);
    }
    if (!error.isEmpty()) {
        return {{}, error, {}, request.sourceGeometry};
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

CaptureResult captureWithGrim(const CaptureRequest &request)
{
    const QStringList baseArguments{QStringLiteral("-t"), QStringLiteral("ppm"), QStringLiteral("-s"), QStringLiteral("1")};

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

CaptureResult captureWaylandFrame(const CaptureRequest &request)
{
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
    if (!isWaylandSession()) {
        return captureWithQScreen(request);
    }

    return captureWaylandFrame(request);
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
