#pragma once

#include "screen_capture.h"

#include "capture_geometry.h"
#include "debug_log.h"

#ifdef MARK_SHOT_WITH_DBUS
#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusError>
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
#include <QImage>
#include <QMutex>
#include <QMutexLocker>
#include <QObject>
#include <QPainter>
#include <QPixmap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QRect>
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
inline constexpr int MARKSHOT_SPA_PARAM_BUFFERS_BUFFERS = 1;
inline constexpr int MARKSHOT_SPA_PARAM_BUFFERS_BLOCKS = 2;
inline constexpr int MARKSHOT_SPA_PARAM_BUFFERS_SIZE = 3;
inline constexpr int MARKSHOT_SPA_PARAM_BUFFERS_STRIDE = 4;
inline constexpr int MARKSHOT_SPA_PARAM_BUFFERS_DATA_TYPE = 6;
#endif
#endif

#ifdef MARK_SHOT_WITH_DBUS

// Minimal stream descriptor returned by the desktop portal screencast API.
// Properties carry compositor-specific geometry metadata used for cropping.
struct PortalStream {
    uint nodeId = 0;
    QVariantMap properties;
};

using PortalStreamList = QList<PortalStream>;

Q_DECLARE_METATYPE(PortalStream)
Q_DECLARE_METATYPE(PortalStreamList)

QDBusArgument &operator<<(QDBusArgument &argument, const PortalStream &stream);
const QDBusArgument &operator>>(const QDBusArgument &argument, PortalStream &stream);

inline constexpr uint kPortalSourceMonitor = 1u;
inline constexpr uint kPortalCursorHidden = 1u;
inline constexpr uint kPortalCursorEmbedded = 2u;
inline constexpr uint kPortalCursorMetadata = 4u;
inline constexpr unsigned long kScreencastFirstFrameSettleMs = 1500;

// Small synchronous bridge for portal Response signals. Capture code starts a
// nested event loop and exits it when this object emits finished().
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

#ifdef HAVE_XCB

// Atom cache used while enumerating visible X11 windows for region hints.
struct X11WindowAtoms {
    xcb_atom_t netClientListStacking = XCB_ATOM_NONE;
    xcb_atom_t netClientList = XCB_ATOM_NONE;
    xcb_atom_t netWmState = XCB_ATOM_NONE;
    xcb_atom_t netWmStateHidden = XCB_ATOM_NONE;
    xcb_atom_t netFrameExtents = XCB_ATOM_NONE;
    xcb_atom_t wmState = XCB_ATOM_NONE;
};

xcb_atom_t internX11Atom(xcb_connection_t *connection, const char *name);
X11WindowAtoms readX11WindowAtoms(xcb_connection_t *connection);
QVector<xcb_window_t> readX11WindowListProperty(xcb_connection_t *connection,
                                                xcb_window_t window,
                                                xcb_atom_t property);
QVector<xcb_atom_t> readX11AtomListProperty(xcb_connection_t *connection,
                                            xcb_window_t window,
                                            xcb_atom_t property);
QVector<std::uint32_t> readX11CardinalListProperty(xcb_connection_t *connection,
                                                   xcb_window_t window,
                                                   xcb_atom_t property,
                                                   uint32_t maxValues);
bool x11WindowHasState(xcb_connection_t *connection,
                       xcb_window_t window,
                       const X11WindowAtoms &atoms,
                       xcb_atom_t state);
bool x11WindowIsIconic(xcb_connection_t *connection,
                       xcb_window_t window,
                       const X11WindowAtoms &atoms);
bool x11WindowIsHiddenOrIconic(xcb_connection_t *connection,
                               xcb_window_t window,
                               const X11WindowAtoms &atoms);
std::optional<QRect> x11WindowFrameGeometry(xcb_connection_t *connection,
                                            xcb_window_t root,
                                            xcb_window_t window,
                                            const X11WindowAtoms &atoms);
void appendUniqueWindowRect(QVector<QRect> *results, const QRect &screenRect, QRect rect);

#endif

// Cross-backend environment and geometry helpers.
bool isWaylandSession();
QString desktopEnvironmentText();
bool prefersGrim();
bool isKdePlasma();
QRect virtualScreensGeometry();
QImage normalizeCaptureImage(QImage image);

// Qt/QScreen capture path used for X11, Windows, and non-portal fallbacks.
CaptureResult captureAllScreensWithQScreen(const CaptureRequest &request);
CaptureResult captureWithQScreen(const CaptureRequest &request);
QRect screenGeometryForOutputName(const QString &outputName);
QRect screenGeometryForRequest(const CaptureRequest &request);

#if defined(Q_OS_WIN)
// Windows Graphics Capture path. It avoids recording windows excluded with
// SetWindowDisplayAffinity and falls back to QScreen when unavailable.
CaptureResult captureWithWindowsGraphicsCapture(const CaptureRequest &request);
#endif

// grim captures full output frames and then crops them to CaptureRequest when a
// compositor supports grim but not direct rectangle capture.
QRect fullGrimSourceGeometry(const CaptureRequest &request);
CaptureResult runGrim(const QStringList &arguments, const QString &outputName, QRect sourceGeometry, bool cursorIncluded = false);
CaptureResult cropGrimFrameToRequest(CaptureResult capture, QRect frameGeometry, const CaptureRequest &request);

#ifdef MARK_SHOT_WITH_DBUS

QString portalToken();
QString portalRequestPath(const QString &handleToken);
bool connectPortalResponse(const QString &signalPath, PortalResponseReceiver *receiver);
void disconnectPortalResponse(const QString &signalPath, PortalResponseReceiver *receiver);
QString portalScreenshotParentWindow(QWidget *parentDummy);
void registerHostPortalApplication();
QVariantMap waitForPortalResponse(PortalResponseReceiver *receiver, QString *error);

// One-shot screenshot portal path. It may be interactive and is therefore not
// preferred for high-frequency scrolling capture.
CaptureResult captureWithPortalScreenshot(const CaptureRequest &request);
QVariantMap callPortalRequest(QDBusInterface *portal,
                              const QString &method,
                              const QVariantList &arguments,
                              const QString &errorPrefix,
                              QString *error);
bool readPairVariant(const QVariant &value, int *first, int *second);
QVariant unwrappedVariant(QVariant value);
uint portalUintProperty(const QString &interfaceName, const QString &propertyName);
uint preferredPortalCursorMode(uint availableModes, bool includeCursor);
QRect streamGeometryFromProperties(const QVariantMap &properties, const QSize &frameSize);

// Wayland backend cascade: compositor helpers first when available, then portal
// screencast/screenshot or command-line fallback depending on request flags.
CaptureResult captureWithGrim(const CaptureRequest &request);
CaptureResult captureWithKWinScreenShot(const CaptureRequest &request);
CaptureResult captureWaylandFrame(const CaptureRequest &request);
CaptureResult captureWithPortalScreencast(const CaptureRequest &request);
void stopPortalScreencast();
CaptureResult captureWithGnomeScrollHelper(const CaptureRequest &request);

#endif
