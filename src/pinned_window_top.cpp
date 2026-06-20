#include "pinned_window_top.h"

#include "debug_log.h"
#include "layer_shell_runtime.h"
#include "windows_integration.h"

#include <QGuiApplication>
#include <QMargins>
#include <QProcessEnvironment>
#include <QRect>
#include <QScreen>
#include <QString>
#include <QVariant>
#include <QWidget>
#include <QWindow>

#include <algorithm>

#ifdef MARK_SHOT_WITH_DBUS
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusMessage>
#endif

namespace {

#ifdef MARK_SHOT_WITH_DBUS
constexpr const char *kGnomeShellService = "org.gnome.Shell";
constexpr const char *kGnomeHelperPath = "/org/gnome/Shell/Extensions/MarkShotScrollHelper";
constexpr const char *kGnomeHelperInterface = "org.gnome.Shell.Extensions.MarkShotScrollHelper";

/// @brief Checks whether the current process appears to run in GNOME.
/// @return True when GNOME-specific D-Bus helpers should be attempted.
bool isGnomeSession()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString desktop = env.value(QStringLiteral("XDG_CURRENT_DESKTOP")).toLower();
    const QString sessionDesktop = env.value(QStringLiteral("XDG_SESSION_DESKTOP")).toLower();
    return desktop.contains(QStringLiteral("gnome"))
        || sessionDesktop.contains(QStringLiteral("gnome"))
        || env.contains(QStringLiteral("GNOME_DESKTOP_SESSION_ID"));
}

/// @brief Requests the GNOME Shell helper extension to set matching windows above or normal.
/// @param title Window title used to locate pinned image windows.
/// @param alwaysOnTop Whether matching windows should be kept above normal windows.
void applyGnomePinnedWindowsAbove(const QString &title, bool alwaysOnTop)
{
    if (!isGnomeSession() || title.trimmed().isEmpty()) {
        return;
    }

    QDBusInterface helper(QString::fromLatin1(kGnomeShellService),
                          QString::fromLatin1(kGnomeHelperPath),
                          QString::fromLatin1(kGnomeHelperInterface),
                          QDBusConnection::sessionBus());
    if (!helper.isValid()) {
        markshot::debugLog("pinned-window", "GNOME helper D-Bus interface is not available");
        return;
    }

    const QDBusMessage reply = helper.call(QStringLiteral("SetWindowsAbove"), title, alwaysOnTop);
    if (reply.type() == QDBusMessage::ErrorMessage) {
        markshot::debugLog("pinned-window",
                           "GNOME helper SetWindowsAbove failed: %s",
                           reply.errorMessage().toUtf8().constData());
    }
}
#endif

/// @brief Checks whether the current process is running on Wayland.
/// @return True when Wayland-specific protocols can be used.
bool isWaylandSession()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    return QGuiApplication::platformName().contains(QStringLiteral("wayland"), Qt::CaseInsensitive)
        || env.value(QStringLiteral("XDG_SESSION_TYPE")).compare(QStringLiteral("wayland"), Qt::CaseInsensitive) == 0
        || env.contains(QStringLiteral("WAYLAND_DISPLAY"));
}

/// @brief Resolves the best screen for a pinned window.
/// @param window Pinned image window.
/// @return Screen containing the window center, or the window screen.
QScreen *screenForWindow(QWidget *window)
{
    if (!window) {
        return nullptr;
    }

    const QVariant value = window->property("markShotPinnedGeometry");
    if (value.canConvert<QRect>()) {
        const QRect geometry = value.toRect();
        if (geometry.isValid() && !geometry.isEmpty()) {
            if (QScreen *screen = QGuiApplication::screenAt(geometry.center())) {
                return screen;
            }
        }
    }

    if (QScreen *screen = QGuiApplication::screenAt(window->frameGeometry().center())) {
        return screen;
    }
    return window->screen();
}

/// @brief Returns the logical pinned-window geometry tracked by the caller.
/// @param window Pinned image window.
/// @return Explicit logical geometry, or the widget geometry when absent.
QRect pinnedWindowGeometry(QWidget *window)
{
    if (!window) {
        return {};
    }

    const QVariant value = window->property("markShotPinnedGeometry");
    if (value.canConvert<QRect>()) {
        const QRect geometry = value.toRect();
        if (geometry.isValid() && !geometry.isEmpty()) {
            return geometry;
        }
    }
    return window->geometry();
}

/// @brief Builds the floating overlay config from the current window geometry.
/// @param window Pinned image window.
/// @param screen Target screen for local layer-shell coordinates.
/// @return Floating layer-shell configuration.
markshot::layershell::FloatingOverlayConfig floatingOverlayConfig(QWidget *window, QScreen *screen)
{
    const QRect screenGeometry = screen ? screen->geometry() : QRect();
    const QRect geometry = pinnedWindowGeometry(window);
    const int left = geometry.left() - screenGeometry.left();
    const int top = geometry.top() - screenGeometry.top();

    markshot::layershell::FloatingOverlayConfig config;
    config.scope = QStringLiteral("dock");
    config.keyboardInteractivity = markshot::layershell::KeyboardInteractivity::OnDemand;
    config.activateOnShow = true;
    config.closeOnDismissed = false;
    config.wantsActiveScreenWhenNoScreen = false;
    config.desiredSize = geometry.size();
    config.margins = QMargins(std::max(0, left), std::max(0, top), 0, 0);
    return config;
}

/// @brief Checks whether protocol-level topmost should be attempted.
/// @return True for non-GNOME Wayland sessions.
bool shouldUseLayerShellTop()
{
    if (!isWaylandSession()) {
        return false;
    }
#ifdef MARK_SHOT_WITH_DBUS
    if (isGnomeSession()) {
        return false;
    }
#endif
    return true;
}

/// @brief Configures the pinned window as a layer-shell overlay where supported.
/// @param window Pinned image window to configure.
/// @return True when layer-shell configuration succeeded.
bool configurePinnedLayerShell(QWidget *window)
{
    if (!window || !shouldUseLayerShellTop()) {
        return false;
    }

    QScreen *screen = screenForWindow(window);
    const markshot::layershell::FloatingOverlayConfig config = floatingOverlayConfig(window, screen);
    const bool configured = markshot::layershell::configureFloatingOverlay(window, screen, config);
    window->setProperty("markShotPinnedLayerShellActive", configured);
    return configured;
}

/// @brief Updates layer-shell placement for a pinned overlay.
/// @param window Pinned image window to update.
void updatePinnedLayerShell(QWidget *window)
{
    if (!window || !shouldUseLayerShellTop()) {
        return;
    }

    QScreen *screen = screenForWindow(window);
    const markshot::layershell::FloatingOverlayConfig config = floatingOverlayConfig(window, screen);
    markshot::layershell::updateFloatingOverlay(window, screen, config);
}

/// @brief Applies the Qt topmost hint while preserving visible geometry.
/// @param window Window to update.
/// @param alwaysOnTop Whether Qt should request the topmost state.
/// @param showAfterChange Whether a visible window should be shown immediately after flag changes.
void applyQtTopHint(QWidget *window, bool alwaysOnTop, bool showAfterChange = true)
{
    Qt::WindowFlags flags = window->windowFlags();
    if (alwaysOnTop) {
        flags |= Qt::WindowStaysOnTopHint;
    } else {
        flags &= ~Qt::WindowFlags(Qt::WindowStaysOnTopHint);
    }

    if (flags == window->windowFlags()) {
        return;
    }

    const bool visible = window->isVisible();
    const QRect geometry = window->geometry();
    window->setWindowFlags(flags);
    window->setGeometry(geometry);
    if (visible && showAfterChange) {
        window->show();
    }
}

}  // namespace

namespace markshot::shot {

void applyPinnedWindowTopState(QWidget *window, bool alwaysOnTop)
{
    if (!window) {
        return;
    }

    const bool layerShellTop = alwaysOnTop && shouldUseLayerShellTop();
    const bool wasVisible = window->isVisible();
    if (!alwaysOnTop) {
        window->setProperty("markShotPinnedLayerShellActive", false);
    }
    applyQtTopHint(window, alwaysOnTop, !layerShellTop);
    markshot::windows::setWindowTopMost(window, alwaysOnTop);
    if (alwaysOnTop) {
        configurePinnedLayerShell(window);
        if (layerShellTop && wasVisible && !window->isVisible()) {
            window->show();
        }
        raisePinnedWindowOnPlatform(window);
    }

#ifdef MARK_SHOT_WITH_DBUS
    applyGnomePinnedWindowsAbove(window->windowTitle(), alwaysOnTop);
#endif
}

void raisePinnedWindowOnPlatform(QWidget *window)
{
    if (!window) {
        return;
    }

    window->raise();
    if (QWindow *nativeWindow = window->windowHandle()) {
        nativeWindow->raise();
    }
    markshot::windows::raiseTopMostWindow(window);
    updatePinnedLayerShell(window);

#ifdef MARK_SHOT_WITH_DBUS
    applyGnomePinnedWindowsAbove(window->windowTitle(), true);
#endif
}

void syncPinnedWindowTopGeometry(QWidget *window)
{
    updatePinnedLayerShell(window);
}

void syncPinnedWindowTopGeometry(QWidget *window, const QRect &geometry)
{
    if (!window) {
        return;
    }
    window->setProperty("markShotPinnedGeometry", geometry);
    updatePinnedLayerShell(window);
}

bool pinnedWindowUsesLayerShellTop()
{
    return shouldUseLayerShellTop();
}

bool pinnedWindowHasLayerShellTop(QWidget *window)
{
    return window && window->property("markShotPinnedLayerShellActive").toBool();
}

}  // namespace markshot::shot
