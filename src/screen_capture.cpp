#include "screen_capture.h"

#include "capture_geometry.h"
#include "debug_log.h"

#ifdef MARK_SHOT_WITH_DBUS
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
#endif
#include <QDateTime>
#include <QDir>
#include <QEventLoop>
#include <QFile>
#include <QGuiApplication>
#include <QMutex>
#include <QMutexLocker>
#include <QObject>
#include <QPainter>
#include <QPoint>
#include <QPixmap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QRect>
#include <QRectF>
#include <QScreen>
#include <QStringList>
#include <QThread>
#include <QTimer>
#include <QUrl>
#include <QUuid>
#include <QVariantMap>
#include <QWaitCondition>
#include <QWidget>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <optional>
#include <utility>

#ifdef MARK_SHOT_WITH_DBUS
#include <fcntl.h>
#include <poll.h>
#include <unistd.h>
#endif

#ifdef HAVE_XCB
#include <xcb/xcb.h>
#endif

#ifdef HAVE_PIPEWIRE
#ifdef HAVE_LIBPORTAL
#ifdef signals
#define MARKSHOT_RESTORE_QT_SIGNALS_MACRO
#pragma push_macro("signals")
#undef signals
#endif
#include <libportal/portal.h>
#include <libportal/portal-helpers.h>
#ifdef MARKSHOT_RESTORE_QT_SIGNALS_MACRO
#pragma pop_macro("signals")
#undef MARKSHOT_RESTORE_QT_SIGNALS_MACRO
#endif
#endif
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

#ifdef MARK_SHOT_WITH_DBUS

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

bool isGnomeWaylandSession();
bool hasGnomeScrollHelper();
CaptureResult captureWithGnomeScrollHelper(const CaptureRequest &request);

#endif

namespace {

#ifdef MARK_SHOT_WITH_DBUS

constexpr uint kPortalSourceMonitor = 1u;
constexpr uint kPortalCursorHidden = 1u;
constexpr uint kPortalCursorEmbedded = 2u;
constexpr uint kPortalCursorMetadata = 4u;
constexpr unsigned long kScreencastFirstFrameSettleMs = 1500;

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

#endif

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

QRect virtualScreensGeometry()
{
    QRect geometry;
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        geometry = geometry.isNull() ? screen->geometry() : geometry.united(screen->geometry());
    }
    return geometry;
}

QImage normalizeCaptureImage(QImage image)
{
    if (!image.isNull()) {
        image.setDevicePixelRatio(1.0);
    }
    return image;
}

CaptureResult captureAllScreensWithQScreen(const CaptureRequest &request)
{
    QRect frameGeometry = request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()
        ? request.sourceGeometry.normalized()
        : virtualScreensGeometry();
    if (frameGeometry.isEmpty()) {
        return {{}, QStringLiteral("no virtual screen geometry available for capture"), {}, {}};
    }

    QImage combined(frameGeometry.size(), QImage::Format_ARGB32_Premultiplied);
    combined.fill(Qt::transparent);

    QPainter painter(&combined);
    painter.setCompositionMode(QPainter::CompositionMode_Source);

    int capturedScreens = 0;
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (!screen) {
            continue;
        }

        const QRect screenGeometry = screen->geometry();
        const QRect overlap = screenGeometry.intersected(frameGeometry);
        if (overlap.isEmpty()) {
            continue;
        }

        const QPixmap pixmap = screen->grabWindow(0);
        if (pixmap.isNull()) {
            markshot::debugLog("capture",
                               "qscreen-all screen=%s returned null pixmap",
                               screen->name().toUtf8().constData());
            continue;
        }

        const QImage image = pixmap.toImage().convertToFormat(QImage::Format_ARGB32_Premultiplied);
        const QRect sourceRect = markshot::capture::scaledCropRect(screenGeometry, overlap, image.size());
        if (sourceRect.isEmpty()) {
            continue;
        }

        const QRect destinationRect(overlap.topLeft() - frameGeometry.topLeft(), overlap.size());
        painter.drawImage(destinationRect, image, sourceRect);
        ++capturedScreens;
    }
    painter.end();

    if (capturedScreens == 0) {
        return {{}, QStringLiteral("QScreen::grabWindow returned no usable screen pixmaps"), {}, frameGeometry};
    }

    markshot::debugLog("capture",
                       "qscreen-all screens=%d virtual_geom=%d,%d %dx%d result=%dx%d",
                       capturedScreens,
                       frameGeometry.x(), frameGeometry.y(),
                       frameGeometry.width(), frameGeometry.height(),
                       combined.width(), combined.height());
    return {combined, {}, {}, frameGeometry};
}

CaptureResult captureWithQScreen(const CaptureRequest &request)
{
    if (request.allOutputs) {
        return captureAllScreensWithQScreen(request);
    }

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

QRect screenGeometryForOutputName(const QString &outputName)
{
    if (outputName.isEmpty()) {
        return {};
    }

    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (screen && screen->name() == outputName) {
            return screen->geometry();
        }
    }
    return {};
}

QRect screenGeometryForRequest(const CaptureRequest &request)
{
    if (!request.preferredOutputName.isEmpty()) {
        const QRect outputGeometry = screenGeometryForOutputName(request.preferredOutputName);
        if (!outputGeometry.isEmpty()) {
            return outputGeometry;
        }
    }

    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        if (QScreen *screen = QGuiApplication::screenAt(request.sourceGeometry.center())) {
            return screen->geometry();
        }
    }
    return {};
}

QRect fullGrimSourceGeometry(const CaptureRequest &request)
{
    if (request.allOutputs && request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        return request.sourceGeometry.normalized();
    }

    const QRect virtualGeometry = virtualScreensGeometry();
    if (!virtualGeometry.isEmpty()) {
        return virtualGeometry;
    }
    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        return request.sourceGeometry.normalized();
    }
    return {};
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

CaptureResult cropGrimFrameToRequest(CaptureResult capture, QRect frameGeometry, const CaptureRequest &request)
{
    if (capture.image.isNull()) {
        return capture;
    }

    if (frameGeometry.isValid() && !frameGeometry.isEmpty()) {
        capture.sourceGeometry = frameGeometry.normalized();
    }

    if (!request.sourceGeometry.isValid() || request.sourceGeometry.isEmpty() || request.allOutputs) {
        return capture;
    }

    if (capture.sourceGeometry.isEmpty()) {
        return {{},
                QStringLiteral("grim capture has no source geometry for local crop"),
                capture.outputName,
                request.sourceGeometry};
    }

    const QRect requested = request.sourceGeometry.normalized();
    const QRect overlap = requested.intersected(capture.sourceGeometry);
    if (overlap.isEmpty()) {
        return {{},
                QStringLiteral("grim capture does not cover requested geometry"),
                capture.outputName,
                request.sourceGeometry};
    }

    const QSize frameSize = capture.image.size();
    QImage cropped = markshot::capture::cropFrameToRequest(capture.image,
                                                           capture.sourceGeometry,
                                                           requested);
    if (cropped.isNull()) {
        return {{},
                QStringLiteral("grim local crop is empty"),
                capture.outputName,
                request.sourceGeometry};
    }

    capture.image = cropped.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const QSize croppedSize = capture.image.size();
    capture.sourceGeometry = overlap;
    markshot::debugLog("capture",
                       "grim-local-crop frame=%dx%d frame_geom=%d,%d %dx%d "
                       "requested=%d,%d %dx%d overlap=%d,%d %dx%d result=%dx%d",
                       frameSize.width(), frameSize.height(),
                       frameGeometry.x(), frameGeometry.y(),
                       frameGeometry.width(), frameGeometry.height(),
                       requested.x(), requested.y(), requested.width(), requested.height(),
                       overlap.x(), overlap.y(), overlap.width(), overlap.height(),
                       croppedSize.width(), croppedSize.height());
    return capture;
}

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

#ifdef HAVE_PIPEWIRE

#ifdef HAVE_LIBPORTAL

QString glibErrorText(const GError *error)
{
    return error && error->message
        ? QString::fromUtf8(error->message)
        : QStringLiteral("unknown libportal error");
}

QVariant gVariantToQVariant(GVariant *value)
{
    if (!value) {
        return {};
    }

    if (g_variant_is_of_type(value, G_VARIANT_TYPE_VARIANT)) {
        GVariant *nested = g_variant_get_variant(value);
        const QVariant converted = gVariantToQVariant(nested);
        g_variant_unref(nested);
        return converted;
    }

    switch (g_variant_classify(value)) {
    case G_VARIANT_CLASS_BOOLEAN:
        return static_cast<bool>(g_variant_get_boolean(value));
    case G_VARIANT_CLASS_BYTE:
        return static_cast<int>(g_variant_get_byte(value));
    case G_VARIANT_CLASS_INT16:
        return static_cast<int>(g_variant_get_int16(value));
    case G_VARIANT_CLASS_UINT16:
        return static_cast<uint>(g_variant_get_uint16(value));
    case G_VARIANT_CLASS_INT32:
        return g_variant_get_int32(value);
    case G_VARIANT_CLASS_UINT32:
        return g_variant_get_uint32(value);
    case G_VARIANT_CLASS_INT64:
        return static_cast<qlonglong>(g_variant_get_int64(value));
    case G_VARIANT_CLASS_UINT64:
        return static_cast<qulonglong>(g_variant_get_uint64(value));
    case G_VARIANT_CLASS_HANDLE:
        return g_variant_get_handle(value);
    case G_VARIANT_CLASS_DOUBLE:
        return g_variant_get_double(value);
    case G_VARIANT_CLASS_STRING:
    case G_VARIANT_CLASS_OBJECT_PATH:
    case G_VARIANT_CLASS_SIGNATURE:
        return QString::fromUtf8(g_variant_get_string(value, nullptr));
    case G_VARIANT_CLASS_ARRAY:
    case G_VARIANT_CLASS_TUPLE: {
        QVariantList list;
        const gsize childCount = g_variant_n_children(value);
        list.reserve(static_cast<int>(childCount));
        for (gsize i = 0; i < childCount; ++i) {
            GVariant *child = g_variant_get_child_value(value, i);
            list.push_back(gVariantToQVariant(child));
            g_variant_unref(child);
        }
        return list;
    }
    default:
        break;
    }

    gchar *printed = g_variant_print(value, TRUE);
    const QString fallback = printed
        ? QString::fromUtf8(printed)
        : QStringLiteral("<unprintable>");
    g_free(printed);
    return fallback;
}

QVariantMap gVariantDictionaryToVariantMap(GVariant *dictionary)
{
    QVariantMap map;
    if (!dictionary) {
        return map;
    }

    const gsize childCount = g_variant_n_children(dictionary);
    for (gsize i = 0; i < childCount; ++i) {
        GVariant *entry = g_variant_get_child_value(dictionary, i);
        if (!entry || g_variant_n_children(entry) < 2) {
            if (entry) {
                g_variant_unref(entry);
            }
            continue;
        }

        GVariant *key = g_variant_get_child_value(entry, 0);
        GVariant *value = g_variant_get_child_value(entry, 1);
        if (key && g_variant_is_of_type(key, G_VARIANT_TYPE_STRING)) {
            map.insert(QString::fromUtf8(g_variant_get_string(key, nullptr)),
                       gVariantToQVariant(value));
        }
        if (value) {
            g_variant_unref(value);
        }
        if (key) {
            g_variant_unref(key);
        }
        g_variant_unref(entry);
    }
    return map;
}

bool readLibportalStream(GVariant *streams,
                         uint *nodeId,
                         QVariantMap *properties,
                         QString *error)
{
    if (!streams || g_variant_n_children(streams) == 0) {
        if (error) {
            *error = QStringLiteral("libportal ScreenCast Start returned no PipeWire stream");
        }
        return false;
    }

    GVariant *stream = g_variant_get_child_value(streams, 0);
    if (!stream || g_variant_n_children(stream) < 2) {
        if (stream) {
            g_variant_unref(stream);
        }
        if (error) {
            *error = QStringLiteral("libportal returned an invalid stream descriptor");
        }
        return false;
    }

    GVariant *nodeValue = g_variant_get_child_value(stream, 0);
    GVariant *propertiesValue = g_variant_get_child_value(stream, 1);
    const bool nodeOk = nodeValue && g_variant_is_of_type(nodeValue, G_VARIANT_TYPE_UINT32);
    if (nodeOk && nodeId) {
        *nodeId = g_variant_get_uint32(nodeValue);
    }
    if (propertiesValue && properties) {
        *properties = gVariantDictionaryToVariantMap(propertiesValue);
    }

    if (propertiesValue) {
        g_variant_unref(propertiesValue);
    }
    if (nodeValue) {
        g_variant_unref(nodeValue);
    }
    g_variant_unref(stream);

    if (!nodeOk || !nodeId || *nodeId == 0) {
        if (error) {
            *error = QStringLiteral("libportal returned a PipeWire stream without a node id");
        }
        return false;
    }
    return true;
}

class LibportalOperation final {
public:
    LibportalOperation(GMainContext *context, GCancellable *cancellable)
        : m_loop(g_main_loop_new(context, FALSE))
        , m_cancellable(cancellable)
    {
        m_timeoutSource = g_timeout_source_new_seconds(120);
        g_source_set_callback(m_timeoutSource, &LibportalOperation::timeoutCallback, this, nullptr);
        g_source_attach(m_timeoutSource, context);
    }

    ~LibportalOperation()
    {
        if (m_timeoutSource) {
            if (!g_source_is_destroyed(m_timeoutSource)) {
                g_source_destroy(m_timeoutSource);
            }
            g_source_unref(m_timeoutSource);
        }
        if (m_loop) {
            g_main_loop_unref(m_loop);
        }
    }

    GMainLoop *loop() const
    {
        return m_loop;
    }

    void finish()
    {
        m_done = true;
        if (m_loop) {
            g_main_loop_quit(m_loop);
        }
    }

    bool wait(const QString &operation, QString *error)
    {
        if (!m_done && m_loop) {
            g_main_loop_run(m_loop);
        }
        if (m_timedOut) {
            if (error) {
                *error = QStringLiteral("%1 timed out").arg(operation);
            }
            return false;
        }
        return true;
    }

private:
    static gboolean timeoutCallback(gpointer data)
    {
        auto *self = static_cast<LibportalOperation *>(data);
        if (!self) {
            return G_SOURCE_REMOVE;
        }
        self->m_timedOut = true;
        if (self->m_cancellable) {
            g_cancellable_cancel(self->m_cancellable);
        }
        return G_SOURCE_REMOVE;
    }

    GMainLoop *m_loop = nullptr;
    GCancellable *m_cancellable = nullptr;
    GSource *m_timeoutSource = nullptr;
    bool m_timedOut = false;
    bool m_done = false;
};

struct LibportalCreateCall {
    LibportalOperation *operation = nullptr;
    XdpSession *session = nullptr;
    GError *error = nullptr;
};

void onLibportalCreateScreencastFinished(GObject *object,
                                         GAsyncResult *result,
                                         gpointer data)
{
    auto *call = static_cast<LibportalCreateCall *>(data);
    if (!call || !call->operation) {
        return;
    }
    call->session = xdp_portal_create_screencast_session_finish(XDP_PORTAL(object),
                                                                result,
                                                                &call->error);
    call->operation->finish();
}

struct LibportalStartCall {
    LibportalOperation *operation = nullptr;
    gboolean ok = FALSE;
    GError *error = nullptr;
};

void onLibportalSessionStartFinished(GObject *object,
                                     GAsyncResult *result,
                                     gpointer data)
{
    auto *call = static_cast<LibportalStartCall *>(data);
    if (!call || !call->operation) {
        return;
    }
    call->ok = xdp_session_start_finish(XDP_SESSION(object), result, &call->error);
    call->operation->finish();
}

#endif  // HAVE_LIBPORTAL

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
        m_frameCount = 0;
        QMutexLocker locker(&m_frameMutex);
        m_latestFrame = {};
        m_latestFrameTimeMs = 0;
        m_streamGeometry = {};
    }

private:
    bool start(QString *error)
    {
#ifdef HAVE_LIBPORTAL
        QString libportalError;
        if (startWithLibportal(&libportalError)) {
            m_started = true;
            return true;
        }
        markshot::debugLog("screencast", "libportal-start-failed error=%s",
                           libportalError.toUtf8().constData());
        stop();
#endif

        QString dbusError;
        if (!startWithDbusPortal(&dbusError)) {
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
    bool startWithLibportal(QString *error)
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
        uint cursorMode = preferredPortalCursorMode(availableCursorModes);
        if (cursorMode == 0) {
            cursorMode = XDP_CURSOR_MODE_HIDDEN;
        }
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

    bool startWithDbusPortal(QString *error)
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

CaptureResult captureWithPortalScreencast(const CaptureRequest &request)
{
    return {{}, QStringLiteral("PipeWire support was not enabled at build time"), {}, request.sourceGeometry};
}

void stopPortalScreencast()
{
}

#endif

#endif  // MARK_SHOT_WITH_DBUS

CaptureResult captureWithGrim(const CaptureRequest &request)
{
    const QStringList baseArguments{QStringLiteral("-t"), QStringLiteral("ppm")};

    if (!request.allOutputs && !request.preferredOutputName.isEmpty()) {
        const QRect outputGeometry = screenGeometryForRequest(request);
        QStringList arguments = baseArguments;
        arguments << QStringLiteral("-o") << request.preferredOutputName << QStringLiteral("-");
        CaptureResult outputCapture = runGrim(arguments, request.preferredOutputName, outputGeometry);
        if (!outputCapture.image.isNull()) {
            if (!outputGeometry.isEmpty()) {
                return cropGrimFrameToRequest(std::move(outputCapture), outputGeometry, request);
            }
            if (!request.sourceGeometry.isValid() || request.sourceGeometry.isEmpty()) {
                return outputCapture;
            }
        }

        const QString outputError = outputCapture.error;
        const QRect fullGeometry = fullGrimSourceGeometry(request);
        QStringList fullArguments = baseArguments;
        fullArguments << QStringLiteral("-");
        CaptureResult fullCapture = runGrim(fullArguments, {}, fullGeometry);
        if (!fullCapture.image.isNull()) {
            fullCapture.outputName = request.preferredOutputName;
            return cropGrimFrameToRequest(std::move(fullCapture), fullGeometry, request);
        }

        if (!outputError.isEmpty() && !fullCapture.error.isEmpty()) {
            return {{},
                    QStringLiteral("%1\nFull-desktop grim fallback: %2")
                        .arg(outputError, fullCapture.error),
                    request.preferredOutputName,
                    request.sourceGeometry};
        }
        if (!outputGeometry.isValid() || outputGeometry.isEmpty()) {
            const QString fallbackError = fullCapture.error.isEmpty()
                ? QStringLiteral("full-desktop grim fallback was not usable")
                : fullCapture.error;
            return {{},
                    QStringLiteral("grim output capture has no Qt output geometry for local crop\nFull-desktop grim fallback: %1")
                        .arg(fallbackError),
                    request.preferredOutputName,
                    request.sourceGeometry};
        }
        return outputCapture.image.isNull() ? fullCapture : outputCapture;
    }

    const QRect frameGeometry = fullGrimSourceGeometry(request);
    QStringList arguments = baseArguments;
    arguments << QStringLiteral("-");
    CaptureResult fullCapture = runGrim(arguments, {}, frameGeometry);
    return cropGrimFrameToRequest(std::move(fullCapture), frameGeometry, request);
}

#ifdef MARK_SHOT_WITH_DBUS

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

    if (isGnomeWaylandSession() && hasGnomeScrollHelper() && request.sourceGeometry.isValid()
        && !request.sourceGeometry.isEmpty() && !request.allOutputs) {
        markshot::debugLog("capture", "route=gnome-scroll-helper");
        CaptureResult gnomeCapture = captureWithGnomeScrollHelper(request);
        if (!gnomeCapture.image.isNull()) {
            markshot::debugLog("capture", "gnome-scroll-helper-ok frame=%dx%d",
                               gnomeCapture.image.width(), gnomeCapture.image.height());
            return gnomeCapture;
        }
        markshot::debugLog("capture", "gnome-scroll-helper-failed (falling back) error=%s",
                           gnomeCapture.error.toUtf8().constData());
    }

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

#endif  // MARK_SHOT_WITH_DBUS

} // namespace

#ifdef MARK_SHOT_WITH_DBUS
#include "screen_capture.moc"
#endif

CaptureResult captureScreenFrame(const CaptureRequest &request)
{
#ifdef MARK_SHOT_WITH_DBUS
    CaptureResult result = isWaylandSession()
        ? captureWaylandFrame(request)
        : captureWithQScreen(request);
#else
    CaptureResult result = captureWithQScreen(request);
#endif
    result.image = normalizeCaptureImage(result.image);
    return result;
}

void stopActiveScreencastCapture()
{
#ifdef MARK_SHOT_WITH_DBUS
    stopPortalScreencast();
#endif
}

namespace {

#ifdef HAVE_XCB

constexpr std::uint32_t kWmStateIconic = 3;

struct X11WindowAtoms {
    xcb_atom_t netClientListStacking = XCB_ATOM_NONE;
    xcb_atom_t netClientList = XCB_ATOM_NONE;
    xcb_atom_t netWmState = XCB_ATOM_NONE;
    xcb_atom_t netWmStateHidden = XCB_ATOM_NONE;
    xcb_atom_t netFrameExtents = XCB_ATOM_NONE;
    xcb_atom_t wmState = XCB_ATOM_NONE;
};

xcb_atom_t internX11Atom(xcb_connection_t *connection, const char *name)
{
    if (!connection || !name) {
        return XCB_ATOM_NONE;
    }

    xcb_intern_atom_cookie_t cookie =
        xcb_intern_atom(connection, 0, static_cast<uint16_t>(std::strlen(name)), name);
    xcb_intern_atom_reply_t *reply = xcb_intern_atom_reply(connection, cookie, nullptr);
    const xcb_atom_t atom = reply ? reply->atom : XCB_ATOM_NONE;
    std::free(reply);
    return atom;
}

X11WindowAtoms readX11WindowAtoms(xcb_connection_t *connection)
{
    X11WindowAtoms atoms;
    atoms.netClientListStacking = internX11Atom(connection, "_NET_CLIENT_LIST_STACKING");
    atoms.netClientList = internX11Atom(connection, "_NET_CLIENT_LIST");
    atoms.netWmState = internX11Atom(connection, "_NET_WM_STATE");
    atoms.netWmStateHidden = internX11Atom(connection, "_NET_WM_STATE_HIDDEN");
    atoms.netFrameExtents = internX11Atom(connection, "_NET_FRAME_EXTENTS");
    atoms.wmState = internX11Atom(connection, "WM_STATE");
    return atoms;
}

QVector<xcb_window_t> readX11WindowListProperty(xcb_connection_t *connection,
                                                xcb_window_t window,
                                                xcb_atom_t property)
{
    QVector<xcb_window_t> windows;
    if (!connection || property == XCB_ATOM_NONE) {
        return windows;
    }

    xcb_get_property_cookie_t cookie =
        xcb_get_property(connection, 0, window, property, XCB_ATOM_WINDOW, 0, 4096);
    xcb_get_property_reply_t *reply = xcb_get_property_reply(connection, cookie, nullptr);
    if (!reply) {
        return windows;
    }

    if (reply->format == 32 && reply->type == XCB_ATOM_WINDOW) {
        const int count = xcb_get_property_value_length(reply) / static_cast<int>(sizeof(xcb_window_t));
        const xcb_window_t *values =
            static_cast<const xcb_window_t *>(xcb_get_property_value(reply));
        windows.reserve(count);
        for (int i = 0; i < count; ++i) {
            if (values[i] != XCB_WINDOW_NONE) {
                windows.append(values[i]);
            }
        }
    }

    std::free(reply);
    return windows;
}

QVector<xcb_atom_t> readX11AtomListProperty(xcb_connection_t *connection,
                                            xcb_window_t window,
                                            xcb_atom_t property)
{
    QVector<xcb_atom_t> atoms;
    if (!connection || property == XCB_ATOM_NONE) {
        return atoms;
    }

    xcb_get_property_cookie_t cookie =
        xcb_get_property(connection, 0, window, property, XCB_ATOM_ATOM, 0, 4096);
    xcb_get_property_reply_t *reply = xcb_get_property_reply(connection, cookie, nullptr);
    if (!reply) {
        return atoms;
    }

    if (reply->format == 32 && reply->type == XCB_ATOM_ATOM) {
        const int count = xcb_get_property_value_length(reply) / static_cast<int>(sizeof(xcb_atom_t));
        const xcb_atom_t *values =
            static_cast<const xcb_atom_t *>(xcb_get_property_value(reply));
        atoms.reserve(count);
        for (int i = 0; i < count; ++i) {
            atoms.append(values[i]);
        }
    }

    std::free(reply);
    return atoms;
}

QVector<std::uint32_t> readX11CardinalListProperty(xcb_connection_t *connection,
                                                   xcb_window_t window,
                                                   xcb_atom_t property,
                                                   uint32_t maxValues)
{
    QVector<std::uint32_t> values;
    if (!connection || property == XCB_ATOM_NONE || maxValues == 0) {
        return values;
    }

    xcb_get_property_cookie_t cookie =
        xcb_get_property(connection, 0, window, property, XCB_ATOM_CARDINAL, 0, maxValues);
    xcb_get_property_reply_t *reply = xcb_get_property_reply(connection, cookie, nullptr);
    if (!reply) {
        return values;
    }

    if (reply->format == 32 && reply->type == XCB_ATOM_CARDINAL) {
        const int count = xcb_get_property_value_length(reply) / static_cast<int>(sizeof(std::uint32_t));
        const auto *raw = static_cast<const std::uint32_t *>(xcb_get_property_value(reply));
        values.reserve(count);
        for (int i = 0; i < count; ++i) {
            values.append(raw[i]);
        }
    }

    std::free(reply);
    return values;
}

bool x11WindowHasState(xcb_connection_t *connection,
                       xcb_window_t window,
                       const X11WindowAtoms &atoms,
                       xcb_atom_t state)
{
    if (state == XCB_ATOM_NONE) {
        return false;
    }
    const QVector<xcb_atom_t> states =
        readX11AtomListProperty(connection, window, atoms.netWmState);
    return states.contains(state);
}

bool x11WindowIsIconic(xcb_connection_t *connection,
                       xcb_window_t window,
                       const X11WindowAtoms &atoms)
{
    const QVector<std::uint32_t> values =
        readX11CardinalListProperty(connection, window, atoms.wmState, 2);
    return !values.isEmpty() && values.first() == kWmStateIconic;
}

bool x11WindowIsHiddenOrIconic(xcb_connection_t *connection,
                               xcb_window_t window,
                               const X11WindowAtoms &atoms)
{
    return x11WindowHasState(connection, window, atoms, atoms.netWmStateHidden)
        || x11WindowIsIconic(connection, window, atoms);
}

std::optional<QRect> x11WindowFrameGeometry(xcb_connection_t *connection,
                                            xcb_window_t root,
                                            xcb_window_t window,
                                            const X11WindowAtoms &atoms)
{
    if (!connection || window == XCB_WINDOW_NONE || x11WindowIsHiddenOrIconic(connection, window, atoms)) {
        return std::nullopt;
    }

    xcb_get_window_attributes_cookie_t attrCookie = xcb_get_window_attributes(connection, window);
    xcb_get_window_attributes_reply_t *attrReply =
        xcb_get_window_attributes_reply(connection, attrCookie, nullptr);
    if (!attrReply) {
        return std::nullopt;
    }
    const bool isViewable = attrReply->map_state == XCB_MAP_STATE_VIEWABLE;
    const bool isOverrideRedirect = attrReply->override_redirect != 0;
    std::free(attrReply);
    if (!isViewable || isOverrideRedirect) {
        return std::nullopt;
    }

    xcb_get_geometry_cookie_t geoCookie = xcb_get_geometry(connection, window);
    xcb_get_geometry_reply_t *geoReply = xcb_get_geometry_reply(connection, geoCookie, nullptr);
    if (!geoReply) {
        return std::nullopt;
    }

    xcb_translate_coordinates_cookie_t transCookie =
        xcb_translate_coordinates(connection, window, root, 0, 0);
    xcb_translate_coordinates_reply_t *transReply =
        xcb_translate_coordinates_reply(connection, transCookie, nullptr);
    if (!transReply) {
        std::free(geoReply);
        return std::nullopt;
    }

    QRect rect(transReply->dst_x, transReply->dst_y, geoReply->width, geoReply->height);
    std::free(transReply);
    std::free(geoReply);

    const QVector<std::uint32_t> extents =
        readX11CardinalListProperty(connection, window, atoms.netFrameExtents, 4);
    if (extents.size() >= 4) {
        const int left = static_cast<int>(std::min<std::uint32_t>(extents.at(0), 16384));
        const int right = static_cast<int>(std::min<std::uint32_t>(extents.at(1), 16384));
        const int top = static_cast<int>(std::min<std::uint32_t>(extents.at(2), 16384));
        const int bottom = static_cast<int>(std::min<std::uint32_t>(extents.at(3), 16384));
        rect.adjust(-left, -top, right, bottom);
    }

    return rect.normalized();
}

void appendUniqueWindowRect(QVector<QRect> *results, const QRect &screenRect, QRect rect)
{
    if (!results) {
        return;
    }
    rect = rect.normalized();
    if (rect.width() <= 1 || rect.height() <= 1 || !rect.intersects(screenRect)) {
        return;
    }
    if (!results->contains(rect)) {
        results->append(rect);
    }
}

#endif

} // namespace

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
    const QRect rootRect(0, 0, screenIter.data->width_in_pixels, screenIter.data->height_in_pixels);
    const X11WindowAtoms atoms = readX11WindowAtoms(connection);

    QVector<xcb_window_t> managedWindows =
        readX11WindowListProperty(connection, root, atoms.netClientListStacking);
    if (managedWindows.isEmpty()) {
        managedWindows = readX11WindowListProperty(connection, root, atoms.netClientList);
    }
    if (!managedWindows.isEmpty()) {
        for (xcb_window_t window : std::as_const(managedWindows)) {
            if (const std::optional<QRect> rect =
                    x11WindowFrameGeometry(connection, root, window, atoms)) {
                appendUniqueWindowRect(&results, rootRect, *rect);
            }
        }
        xcb_disconnect(connection);
        return results;
    }

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
            const bool isOverrideRedirect = attrReply->override_redirect != 0;
            std::free(attrReply);

            if (!isViewable || isOverrideRedirect || x11WindowIsHiddenOrIconic(connection, child, atoms)) {
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
                std::free(transReply);

                appendUniqueWindowRect(&results, rootRect, QRect(x, y, w, h));
            }

            std::free(geoReply);
            stack.append(child);
        }

        std::free(treeReply);
    }

    xcb_disconnect(connection);
#endif

    return results;
}

bool isGnomeWaylandSession()
{
#ifdef MARK_SHOT_WITH_DBUS
    if (!isWaylandSession()) {
        return false;
    }
    return desktopEnvironmentText().toLower().contains(QStringLiteral("gnome"));
#else
    return false;
#endif
}

bool hasGnomeScrollHelper()
{
#ifdef MARK_SHOT_WITH_DBUS
    QDBusInterface helper(QStringLiteral("org.gnome.Shell"),
                          QStringLiteral("/org/gnome/Shell/Extensions/MarkShotScrollHelper"),
                          QStringLiteral("org.gnome.Shell.Extensions.MarkShotScrollHelper"),
                          QDBusConnection::sessionBus());
    if (!helper.isValid()) {
        return false;
    }

    QDBusMessage reply = helper.call(QStringLiteral("Version"));
    return reply.type() != QDBusMessage::ErrorMessage && !reply.arguments().isEmpty();
#else
    return false;
#endif
}

bool hasGnomeScrollPreviewHelper()
{
#ifdef MARK_SHOT_WITH_DBUS
    QDBusInterface helper(QStringLiteral("org.gnome.Shell"),
                          QStringLiteral("/org/gnome/Shell/Extensions/MarkShotScrollHelper"),
                          QStringLiteral("org.gnome.Shell.Extensions.MarkShotScrollHelper"),
                          QDBusConnection::sessionBus());
    if (!helper.isValid()) {
        return false;
    }

    QDBusMessage reply = helper.call(QStringLiteral("Version"));
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
        return false;
    }

    bool ok = false;
    const int version = reply.arguments().first().toString().toInt(&ok);
    return ok && version >= 3;
#else
    return false;
#endif
}

CaptureResult captureWithGnomeScrollHelper(const CaptureRequest &request)
{
#ifdef MARK_SHOT_WITH_DBUS
    const QString tempDir = QFile::exists(QStringLiteral("/dev/shm"))
        ? QStringLiteral("/dev/shm")
        : QDir::tempPath();
    const QString tempPath = QStringLiteral("%1/mark-shot-scroll-frame-%2.png")
        .arg(tempDir, QUuid::createUuid().toString(QUuid::Id128));

    QDBusMessage message = QDBusMessage::createMethodCall(
        QStringLiteral("org.gnome.Shell"),
        QStringLiteral("/org/gnome/Shell/Extensions/MarkShotScrollHelper"),
        QStringLiteral("org.gnome.Shell.Extensions.MarkShotScrollHelper"),
        QStringLiteral("ScreenshotArea")
    );
    message << request.sourceGeometry.x()
            << request.sourceGeometry.y()
            << request.sourceGeometry.width()
            << request.sourceGeometry.height()
            << tempPath;

    QDBusMessage reply = QDBusConnection::sessionBus().call(message);
    if (reply.type() == QDBusMessage::ErrorMessage) {
        QFile::remove(tempPath);
        return {{}, reply.errorMessage(), {}, request.sourceGeometry};
    }

    QList<QVariant> args = reply.arguments();
    if (args.size() < 2 || !args.at(0).toBool()) {
        QFile::remove(tempPath);
        return {{}, QStringLiteral("Failed to capture area via GNOME Shell extension"), {}, request.sourceGeometry};
    }

    QString actualPath = args.at(1).toString();
    QImage img(actualPath);
    if (img.isNull()) {
        return {{}, QStringLiteral("Failed to load captured frame from %1").arg(actualPath), {}, request.sourceGeometry};
    }

    QFile::remove(actualPath);
    return {img, {}, {}, request.sourceGeometry};
#else
    return {{}, QStringLiteral("GNOME scroll helper support was not enabled at build time"), {}, request.sourceGeometry};
#endif
}
