#include "windows_integration.h"

#include "debug_log.h"

#include <QGuiApplication>
#include <QScreen>
#include <QString>
#include <QWidget>
#include <QWindow>

#include <algorithm>
#include <string>

#if defined(Q_OS_WIN)
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <dwmapi.h>
#include <windows.h>
#endif

#if defined(Q_OS_LINUX) && defined(HAVE_XCB)
#include <QtGui/qguiapplication_platform.h>

#include <xcb/xcb.h>
#include <xcb/xproto.h>
#endif

namespace markshot::windows {
namespace {

#if defined(Q_OS_WIN)

constexpr DWORD kWdaNone = 0x00000000;
constexpr DWORD kWdaExcludeFromCapture = 0x00000011;

HMONITOR nativeMonitorForScreen(QScreen *screen)
{
    if (!screen) {
        return nullptr;
    }

#if QT_VERSION >= QT_VERSION_CHECK(6, 7, 0)
    if (auto *nativeScreen = screen->nativeInterface<QNativeInterface::QWindowsScreen>()) {
        return nativeScreen->handle();
    }
#endif
    return nullptr;
}

QString windowClassName(HWND hwnd)
{
    wchar_t buffer[256] = {};
    const int length =
        GetClassNameW(hwnd, buffer, static_cast<int>(sizeof(buffer) / sizeof(buffer[0])));
    return length > 0 ? QString::fromWCharArray(buffer, length) : QString();
}

QString windowTitle(HWND hwnd)
{
    const int length = GetWindowTextLengthW(hwnd);
    if (length <= 0) {
        return {};
    }

    std::wstring title(static_cast<std::size_t>(length + 1), L'\0');
    const int copied = GetWindowTextW(hwnd, title.data(), length + 1);
    if (copied <= 0) {
        return {};
    }
    return QString::fromWCharArray(title.data(), copied);
}

bool isCloaked(HWND hwnd)
{
    DWORD cloaked = 0;
    return SUCCEEDED(DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &cloaked, sizeof(cloaked)))
        && cloaked != 0;
}

bool readWindowFrameRect(HWND hwnd, RECT *rect)
{
    if (!rect) {
        return false;
    }

    RECT frame = {};
    if (SUCCEEDED(DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &frame, sizeof(frame)))
        && frame.right > frame.left
        && frame.bottom > frame.top) {
        *rect = frame;
        return true;
    }

    return GetWindowRect(hwnd, rect)
        && rect->right > rect->left
        && rect->bottom > rect->top;
}

QScreen *screenForNativeMonitor(HMONITOR monitor, MONITORINFOEXW *monitorInfo)
{
    if (!monitor || !monitorInfo) {
        return nullptr;
    }

    monitorInfo->cbSize = sizeof(MONITORINFOEXW);
    if (!GetMonitorInfoW(monitor, monitorInfo)) {
        return nullptr;
    }

    const QString deviceName = QString::fromWCharArray(monitorInfo->szDevice);
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (!screen) {
            continue;
        }

#if QT_VERSION >= QT_VERSION_CHECK(6, 7, 0)
        if (nativeMonitorForScreen(screen) == monitor) {
            return screen;
        }
#endif

        const QString screenName = screen->name();
        if (screenName == deviceName
            || deviceName.endsWith(screenName)
            || screenName.endsWith(deviceName)) {
            return screen;
        }
    }

    return QGuiApplication::primaryScreen();
}

QRect qtRectFromNativeRect(const RECT &rect)
{
    RECT monitorProbe = rect;
    HMONITOR monitor = MonitorFromRect(&monitorProbe, MONITOR_DEFAULTTONEAREST);
    MONITORINFOEXW monitorInfo = {};
    QScreen *screen = screenForNativeMonitor(monitor, &monitorInfo);
    if (!screen) {
        return QRect(QPoint(rect.left, rect.top), QPoint(rect.right - 1, rect.bottom - 1)).normalized();
    }

    const qreal dpr = std::max<qreal>(screen->devicePixelRatio(), 1.0);
    const QRect screenGeometry = screen->geometry();
    const int left = screenGeometry.left() + qRound((rect.left - monitorInfo.rcMonitor.left) / dpr);
    const int top = screenGeometry.top() + qRound((rect.top - monitorInfo.rcMonitor.top) / dpr);
    const int right = screenGeometry.left() + qRound((rect.right - monitorInfo.rcMonitor.left) / dpr);
    const int bottom = screenGeometry.top() + qRound((rect.bottom - monitorInfo.rcMonitor.top) / dpr);
    return QRect(QPoint(left, top), QPoint(right - 1, bottom - 1)).normalized();
}

bool isWindowCandidate(HWND hwnd, QRect *geometry)
{
    if (!hwnd || hwnd == GetDesktopWindow() || hwnd == GetShellWindow()) {
        return false;
    }
    if (!IsWindowVisible(hwnd) || IsIconic(hwnd) || isCloaked(hwnd)) {
        return false;
    }

    const LONG_PTR style = GetWindowLongPtrW(hwnd, GWL_STYLE);
    const LONG_PTR exStyle = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if ((style & WS_CHILD) != 0 || (exStyle & WS_EX_TOOLWINDOW) != 0) {
        return false;
    }

    const QString className = windowClassName(hwnd);
    if (className == QStringLiteral("Progman")
        || className == QStringLiteral("WorkerW")
        || className == QStringLiteral("Shell_TrayWnd")
        || className == QStringLiteral("Shell_SecondaryTrayWnd")) {
        return false;
    }

    if (windowTitle(hwnd).trimmed().isEmpty() && (exStyle & WS_EX_APPWINDOW) == 0) {
        return false;
    }

    RECT frame = {};
    if (!readWindowFrameRect(hwnd, &frame)) {
        return false;
    }

    const QRect rect = qtRectFromNativeRect(frame);
    if (rect.width() <= 1 || rect.height() <= 1) {
        return false;
    }

    if (geometry) {
        *geometry = rect;
    }
    return true;
}

BOOL CALLBACK enumWindowsCallback(HWND hwnd, LPARAM lParam)
{
    auto *windows = reinterpret_cast<QVector<QRect> *>(lParam);
    QRect geometry;
    if (isWindowCandidate(hwnd, &geometry) && !windows->contains(geometry)) {
        windows->append(geometry);
    }
    return TRUE;
}

HWND hwndForWidget(QWidget *widget)
{
    if (!widget) {
        return nullptr;
    }

    QWidget *window = widget->window();
    if (!window || !window->isWindow()) {
        return nullptr;
    }

    window->setAttribute(Qt::WA_NativeWindow);
    return reinterpret_cast<HWND>(window->winId());
}

#endif

} // namespace

/// @brief Enumerates the geometries of all visible windows.
/// @return A vector of QRect objects representing the geometries of the windows.
QVector<QRect> enumerateWindowGeometries()
{
    QVector<QRect> windows;
#if defined(Q_OS_WIN)
    EnumWindows(enumWindowsCallback, reinterpret_cast<LPARAM>(&windows));
#endif
    return windows;
}

/// @brief Enumerates the info of all visible windows with z-order.
/// @return A vector of WindowInfo with z-order based on GetTopWindow chain (bottom-to-top, higher = topmost).
QVector<markshot::WindowInfo> enumerateWindowInfos()
{
    QVector<markshot::WindowInfo> results;
#if defined(Q_OS_WIN)
    // GetTopWindow + GW_HWNDNEXT gives deterministic top-to-bottom z-order,
    // unlike EnumWindows which does not guarantee any specific order.
    int zOrder = 0;
    for (HWND hwnd = GetTopWindow(nullptr); hwnd != nullptr; hwnd = GetWindow(hwnd, GW_HWNDNEXT)) {
        QRect geometry;
        if (isWindowCandidate(hwnd, &geometry) &&
            !std::any_of(results.begin(), results.end(),
                         [&](const markshot::WindowInfo &info) { return info.rect == geometry; })) {
            results.append(markshot::WindowInfo{geometry, zOrder++});
        }
    }
    // Reverse z-order to match Linux convention (higher = topmost)
    for (auto &info : results) {
        info.zOrder = zOrder - 1 - *info.zOrder;
    }
#endif
    return results;
}

void setExcludedFromCapture(QWidget *widget, bool excluded)
{
#if defined(Q_OS_WIN)
    HWND hwnd = hwndForWidget(widget);
    if (!hwnd) {
        return;
    }

    const LONG_PTR style = GetWindowLongPtrW(hwnd, GWL_STYLE);
    const LONG_PTR exStyle = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    const bool topLevel = GetAncestor(hwnd, GA_ROOT) == hwnd;
    BOOL compositionEnabled = FALSE;
    DwmIsCompositionEnabled(&compositionEnabled);

    const DWORD affinity = excluded ? kWdaExcludeFromCapture : kWdaNone;
    if (!SetWindowDisplayAffinity(hwnd, affinity)) {
        markshot::debugLog("windows",
                           "SetWindowDisplayAffinity failed hwnd=%p error=%lu excluded=%d top_level=%d visible=%d layered=%d composition=%d style=0x%llx exstyle=0x%llx",
                           static_cast<void *>(hwnd),
                           static_cast<unsigned long>(GetLastError()),
                           excluded ? 1 : 0,
                           topLevel ? 1 : 0,
                           IsWindowVisible(hwnd) ? 1 : 0,
                           (exStyle & WS_EX_LAYERED) != 0 ? 1 : 0,
                           compositionEnabled ? 1 : 0,
                           static_cast<unsigned long long>(style),
                           static_cast<unsigned long long>(exStyle));
        return;
    }

    DWORD appliedAffinity = 0;
    const bool readBack = GetWindowDisplayAffinity(hwnd, &appliedAffinity);
    markshot::debugLog("windows",
                       "SetWindowDisplayAffinity ok hwnd=%p excluded=%d affinity=0x%lx readback=%d applied=0x%lx top_level=%d visible=%d layered=%d composition=%d style=0x%llx exstyle=0x%llx",
                       static_cast<void *>(hwnd),
                       excluded ? 1 : 0,
                       static_cast<unsigned long>(affinity),
                       readBack ? 1 : 0,
                       static_cast<unsigned long>(appliedAffinity),
                       topLevel ? 1 : 0,
                       IsWindowVisible(hwnd) ? 1 : 0,
                       (exStyle & WS_EX_LAYERED) != 0 ? 1 : 0,
                       compositionEnabled ? 1 : 0,
                       static_cast<unsigned long long>(style),
                       static_cast<unsigned long long>(exStyle));
#else
    Q_UNUSED(widget);
    Q_UNUSED(excluded);
#endif
}

void showFullScreenOnScreen(QWidget *widget, QScreen *screen)
{
    if (!widget) {
        return;
    }

    if (screen) {
        widget->setScreen(screen);
        widget->setGeometry(screen->geometry());
    }
#if defined(Q_OS_WIN)
    widget->setWindowState(widget->windowState() & ~Qt::WindowFullScreen);
    widget->show();
#else
    widget->showFullScreen();
#endif

#if defined(Q_OS_WIN)
    HWND hwnd = hwndForWidget(widget);
    HMONITOR monitor = nativeMonitorForScreen(screen);
    MONITORINFOEXW monitorInfo = {};
    monitorInfo.cbSize = sizeof(monitorInfo);
    if (hwnd && monitor && GetMonitorInfoW(monitor, &monitorInfo)) {
        const RECT rect = monitorInfo.rcMonitor;
        constexpr UINT flags = SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_FRAMECHANGED;
        if (!SetWindowPos(hwnd,
                          HWND_TOPMOST,
                          rect.left,
                          rect.top,
                          rect.right - rect.left,
                          rect.bottom - rect.top,
                          flags)) {
            markshot::debugLog("windows",
                               "fullscreen placement failed screen=%s hwnd=%p monitor=%p error=%lu",
                               screen ? screen->name().toUtf8().constData() : "(none)",
                               static_cast<void *>(hwnd),
                               static_cast<void *>(monitor),
                               static_cast<unsigned long>(GetLastError()));
        }
    }

    QScreen *actualScreen = widget->windowHandle() ? widget->windowHandle()->screen() : widget->screen();
    markshot::debugLog("windows",
                       "fullscreen placement target=%s target_geom=%d,%d %dx%d monitor=%p "
                       "native_geom=%ld,%ld %ldx%ld "
                       "window_geom=%d,%d %dx%d actual=%s",
                       screen ? screen->name().toUtf8().constData() : "(none)",
                       screen ? screen->geometry().x() : 0,
                       screen ? screen->geometry().y() : 0,
                       screen ? screen->geometry().width() : 0,
                       screen ? screen->geometry().height() : 0,
                       static_cast<void *>(monitor),
                       static_cast<long>(monitorInfo.rcMonitor.left),
                       static_cast<long>(monitorInfo.rcMonitor.top),
                       static_cast<long>(monitorInfo.rcMonitor.right - monitorInfo.rcMonitor.left),
                       static_cast<long>(monitorInfo.rcMonitor.bottom - monitorInfo.rcMonitor.top),
                       widget->geometry().x(),
                       widget->geometry().y(),
                       widget->geometry().width(),
                       widget->geometry().height(),
                       actualScreen ? actualScreen->name().toUtf8().constData() : "(none)");
#endif
}

void setWindowTopMost(QWidget *widget, bool alwaysOnTop)
{
#if defined(Q_OS_WIN)
    HWND hwnd = hwndForWidget(widget);
    if (!hwnd) {
        return;
    }

    const HWND insertAfter = alwaysOnTop ? HWND_TOPMOST : HWND_NOTOPMOST;
    constexpr UINT flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER;
    if (!SetWindowPos(hwnd, insertAfter, 0, 0, 0, 0, flags)) {
        markshot::debugLog("windows",
                           "【Windows】【置顶窗口】SetWindowPos failed hwnd=%p error=%lu topmost=%d",
                           static_cast<void *>(hwnd),
                           static_cast<unsigned long>(GetLastError()),
                           alwaysOnTop ? 1 : 0);
    }
#else
    Q_UNUSED(widget);
    Q_UNUSED(alwaysOnTop);
#endif
}

void raiseTopMostWindow(QWidget *widget)
{
#if defined(Q_OS_WIN)
    HWND hwnd = hwndForWidget(widget);
    if (!hwnd) {
        return;
    }

    constexpr UINT flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER;
    if (!SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags)) {
        markshot::debugLog("windows",
                           "【Windows】【置顶窗口】raise SetWindowPos failed hwnd=%p error=%lu",
                           static_cast<void *>(hwnd),
                           static_cast<unsigned long>(GetLastError()));
    }
#else
    Q_UNUSED(widget);
#endif
}

bool isX11QtPlatform()
{
#if defined(Q_OS_LINUX) && defined(HAVE_XCB)
    return QGuiApplication::platformName().compare(QStringLiteral("xcb"), Qt::CaseInsensitive) == 0;
#else
    return false;
#endif
}

bool isNativeX11Session()
{
#if defined(Q_OS_LINUX) && defined(HAVE_XCB)
    // XWayland 会话下 Qt 平台名也是 "xcb"，但部分原生 X11 能力（如 XGrabKey
    // 抓取全局按键）在 XWayland 下不可靠，因此要求会话确为 X11/Xorg。
    return isX11QtPlatform() && !qEnvironmentVariableIsSet("WAYLAND_DISPLAY");
#else
    return false;
#endif
}

void setExcludedFromTaskbar(QWidget *widget, bool excluded)
{
    if (!widget) {
        return;
    }

#if defined(Q_OS_WIN)
    // WS_EX_TOOLWINDOW 让窗口既不出现在任务栏也不出现在 Alt-Tab 列表，
    // 与截图覆盖层的临时性一致。
    HWND hwnd = hwndForWidget(widget);
    if (!hwnd) {
        return;
    }
    const LONG_PTR exStyle = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    const LONG_PTR updated = excluded ? (exStyle | WS_EX_TOOLWINDOW) : (exStyle & ~WS_EX_TOOLWINDOW);
    if (updated == exStyle) {
        return;
    }
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, updated);
    SetWindowPos(hwnd,
                 nullptr,
                 0, 0, 0, 0,
                 SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
#elif defined(Q_OS_LINUX) && defined(HAVE_XCB)
    // 原生 Wayland (xdg-shell) 没有“跳过任务栏”的标准机制，只有 layer-shell
    // 覆盖层天然没有任务栏项；XWayland 无法控制宿主合成器。因此这里只在
    // xcb 平台（含 XWayland）生效。
    if (!isX11QtPlatform()) {
        return;
    }
    const auto *appInstance = qobject_cast<const QGuiApplication *>(QGuiApplication::instance());
    auto *x11Application = appInstance
        ? appInstance->nativeInterface<QNativeInterface::QX11Application>()
        : nullptr;
    xcb_connection_t *connection = x11Application ? x11Application->connection() : nullptr;
    if (!connection) {
        return;
    }
    const xcb_window_t window = static_cast<xcb_window_t>(widget->winId());
    if (!window) {
        return;
    }

    // 原子名与根窗口按连接缓存，避免每次调用都做两次阻塞 intern_atom 往返。
    static xcb_connection_t *s_cachedConnection = nullptr;
    static xcb_atom_t s_stateAtom = XCB_ATOM_NONE;
    static xcb_atom_t s_skipTaskbarAtom = XCB_ATOM_NONE;
    static xcb_window_t s_root = XCB_WINDOW_NONE;
    if (s_cachedConnection != connection || s_stateAtom == XCB_ATOM_NONE) {
        static constexpr char kNetWmStateName[] = "_NET_WM_STATE";
        static constexpr char kNetWmStateSkipTaskbarName[] = "_NET_WM_STATE_SKIP_TASKBAR";
        const xcb_intern_atom_cookie_t stateCookie =
            xcb_intern_atom(connection, 0, sizeof(kNetWmStateName) - 1, kNetWmStateName);
        const xcb_intern_atom_cookie_t skipCookie =
            xcb_intern_atom(connection, 0, sizeof(kNetWmStateSkipTaskbarName) - 1, kNetWmStateSkipTaskbarName);
        xcb_intern_atom_reply_t *stateReply = xcb_intern_atom_reply(connection, stateCookie, nullptr);
        xcb_intern_atom_reply_t *skipReply = xcb_intern_atom_reply(connection, skipCookie, nullptr);
        if (!stateReply || !skipReply) {
            free(stateReply);
            free(skipReply);
            return;
        }
        s_stateAtom = stateReply->atom;
        s_skipTaskbarAtom = skipReply->atom;
        free(stateReply);
        free(skipReply);

        const xcb_setup_t *setup = xcb_get_setup(connection);
        const xcb_screen_iterator_t iterator = setup ? xcb_setup_roots_iterator(setup) : xcb_screen_iterator_t{};
        s_root = iterator.data ? iterator.data->root : XCB_WINDOW_NONE;
        s_cachedConnection = connection;
    }
    if (s_stateAtom == XCB_ATOM_NONE || s_skipTaskbarAtom == XCB_ATOM_NONE || s_root == XCB_WINDOW_NONE) {
        return;
    }

    xcb_client_message_event_t event = {};
    event.response_type = XCB_CLIENT_MESSAGE;
    event.format = 32;
    event.window = window;
    event.type = s_stateAtom;
    event.data.data32[0] = excluded ? 1 : 0; // _NET_WM_STATE_ADD / _NET_WM_STATE_REMOVE
    event.data.data32[1] = s_skipTaskbarAtom;
    event.data.data32[2] = 0;
    event.data.data32[3] = 1; // source indication: 1 = regular application
    xcb_send_event(connection,
                   false,
                   s_root,
                   XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY | XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT,
                   reinterpret_cast<const char *>(&event));
    xcb_flush(connection);
#else
    Q_UNUSED(widget);
    Q_UNUSED(excluded);
#endif
}

} // namespace markshot::windows
