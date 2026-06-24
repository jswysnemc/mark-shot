#include "layer_shell_plugin_interface.h"

#include "debug_log.h"

#include <LayerShellQt/Window>

#include <QObject>
#include <QGuiApplication>
#include <QProcessEnvironment>
#include <QScreen>
#include <QWidget>
#include <QWindow>

namespace {

/// @brief Checks whether the current desktop environment is GNOME.
/// @return True if GNOME desktop environment is detected, false otherwise.
bool isGnomeDesktop()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString desktop =
        (env.value(QStringLiteral("XDG_CURRENT_DESKTOP")) + QLatin1Char(':')
         + env.value(QStringLiteral("XDG_SESSION_DESKTOP")) + QLatin1Char(':')
         + env.value(QStringLiteral("DESKTOP_SESSION")))
            .toLower();
    return desktop.contains(QStringLiteral("gnome"));
}

/// @brief Forces updated layer-shell state to be committed during interactive movement.
/// @param widget Widget whose backing store should commit the pending protocol state.
/// @param nativeWindow Native window used to request a platform update.
/// @param createIfNeeded Whether this is the initial role setup path.
void commitInteractiveLayerState(QWidget *widget, QWindow *nativeWindow, bool createIfNeeded)
{
    if (createIfNeeded || !widget || !widget->isVisible()) {
        return;
    }

    widget->update();
    if (nativeWindow) {
        nativeWindow->requestUpdate();
    }
}

}  // namespace

/// @brief Plugin implementation for Wayland layer-shell integration.
class MarkShotLayerShellPlugin final
    : public QObject
    , public markshot::layershell::PluginInterface
{
    /// @brief Qt meta-object declaration for this class.
    Q_OBJECT
    Q_PLUGIN_METADATA(IID MARK_SHOT_LAYER_SHELL_PLUGIN_IID)
    Q_INTERFACES(markshot::layershell::PluginInterface)

public:
    bool configureOverlay(QWidget *widget,
                          QScreen *screen,
                          const markshot::layershell::OverlayConfig &config) override
    {
        if (!widget) {
            markshot::debugLog("layershell", "configure failed: widget is null");
            return false;
        }
        if (isGnomeDesktop()) {
            markshot::debugLog("layershell",
                               "configure skipped: GNOME does not support the layer-shell protocol");
            return false;
        }

        if (screen) {
            widget->setScreen(screen);
        }

        widget->setAttribute(Qt::WA_NativeWindow);
        widget->winId();

        QWindow *nativeWindow = widget->windowHandle();
        if (!nativeWindow) {
            markshot::debugLog("layershell", "configure failed: no native QWindow");
            return false;
        }
        if (screen) {
            nativeWindow->setScreen(screen);
        }

        LayerShellQt::Window *layerWindow = LayerShellQt::Window::get(nativeWindow);
        if (!layerWindow) {
            markshot::debugLog("layershell",
                               "configure failed: LayerShellQt::Window::get returned null "
                               "platform=%s screen=%s",
                               QGuiApplication::platformName().toUtf8().constData(),
                               screen ? screen->name().toUtf8().constData() : "(none)");
            return false;
        }

        LayerShellQt::Window::Anchors anchors = LayerShellQt::Window::AnchorTop;
        anchors |= LayerShellQt::Window::AnchorBottom;
        anchors |= LayerShellQt::Window::AnchorLeft;
        anchors |= LayerShellQt::Window::AnchorRight;

        layerWindow->setScope(config.scope);
        layerWindow->setLayer(LayerShellQt::Window::LayerOverlay);
        layerWindow->setAnchors(anchors);
        layerWindow->setMargins({});
        layerWindow->setExclusiveZone(-1);
        layerWindow->setKeyboardInteractivity(keyboardInteractivity(config.keyboardInteractivity));
        layerWindow->setActivateOnShow(config.activateOnShow);
        layerWindow->setCloseOnDismissed(config.closeOnDismissed);
        if (screen) {
            layerWindow->setScreen(screen);
        } else if (config.wantsActiveScreenWhenNoScreen) {
            layerWindow->setWantsToBeOnActiveScreen(true);
        }
        const QSize desiredSize = widget->size();
        layerWindow->setDesiredSize(desiredSize);
        markshot::debugLog("layershell",
                           "configured overlay scope=%s screen=%s widget=%dx%d desired=%dx%d",
                           config.scope.toUtf8().constData(),
                           screen ? screen->name().toUtf8().constData() : "(none)",
                           widget->width(), widget->height(),
                           desiredSize.width(), desiredSize.height());
        return true;
    }

    bool configureFloatingOverlay(QWidget *widget,
                                  QScreen *screen,
                                  const markshot::layershell::FloatingOverlayConfig &config) override
    {
        return configureFloatingOverlayInternal(widget, screen, config, true);
    }

    bool updateFloatingOverlay(QWidget *widget,
                               QScreen *screen,
                               const markshot::layershell::FloatingOverlayConfig &config) override
    {
        return configureFloatingOverlayInternal(widget, screen, config, false);
    }

    bool setLayer(QWidget *widget, markshot::layershell::Layer layer) override
    {
        if (!widget) {
            markshot::debugLog("layershell", "setLayer failed: widget is null");
            return false;
        }
        QWindow *nativeWindow = widget->windowHandle();
        if (!nativeWindow) {
            markshot::debugLog("layershell", "setLayer failed: no native QWindow");
            return false;
        }
        LayerShellQt::Window *layerWindow = LayerShellQt::Window::get(nativeWindow);
        if (!layerWindow) {
            markshot::debugLog("layershell", "setLayer failed: no layer window");
            return false;
        }
        layerWindow->setLayer(layer == markshot::layershell::Layer::Overlay
                                  ? LayerShellQt::Window::LayerOverlay
                                  : LayerShellQt::Window::LayerTop);
        markshot::debugLog("layershell", "setLayer to %s",
                           layer == markshot::layershell::Layer::Overlay ? "overlay" : "top");
        return true;
    }

private:
    static bool configureFloatingOverlayInternal(
        QWidget *widget,
        QScreen *screen,
        const markshot::layershell::FloatingOverlayConfig &config,
        bool createIfNeeded)
    {
        if (!widget) {
            markshot::debugLog("layershell", "floating configure failed: widget is null");
            return false;
        }
        if (isGnomeDesktop()) {
            markshot::debugLog("layershell",
                               "floating configure skipped: GNOME does not support layer-shell");
            return false;
        }

        if (screen && widget->screen() != screen) {
            widget->setScreen(screen);
        }
        if (createIfNeeded) {
            widget->setAttribute(Qt::WA_NativeWindow);
            widget->winId();
        }

        QWindow *nativeWindow = widget->windowHandle();
        if (!nativeWindow) {
            markshot::debugLog("layershell", "floating configure failed: no native QWindow");
            return false;
        }
        if (screen && nativeWindow->screen() != screen) {
            nativeWindow->setScreen(screen);
        }

        LayerShellQt::Window *layerWindow = LayerShellQt::Window::get(nativeWindow);
        if (!layerWindow) {
            markshot::debugLog("layershell", "floating configure failed: no layer window");
            return false;
        }

        // 1. 静态 layer-shell 属性只在首次配置时写入,避免拖动时触发 KWin 重新管理窗口
        if (createIfNeeded) {
            layerWindow->setScope(config.scope);
            layerWindow->setLayer(LayerShellQt::Window::LayerOverlay);
            LayerShellQt::Window::Anchors anchors = LayerShellQt::Window::AnchorTop;
            anchors |= LayerShellQt::Window::AnchorLeft;
            layerWindow->setAnchors(anchors);
            layerWindow->setExclusiveZone(-1);
            layerWindow->setKeyboardInteractivity(keyboardInteractivity(config.keyboardInteractivity));
            layerWindow->setActivateOnShow(config.activateOnShow);
            layerWindow->setCloseOnDismissed(config.closeOnDismissed);
        }

        // 2. 拖动和缩放阶段只同步动态位置与尺寸
        layerWindow->setMargins(config.margins);
        if (screen && layerWindow->screen() != screen) {
            layerWindow->setScreen(screen);
        } else if (createIfNeeded && config.wantsActiveScreenWhenNoScreen) {
            layerWindow->setWantsToBeOnActiveScreen(true);
        }
        layerWindow->setDesiredSize(config.desiredSize.isEmpty() ? widget->size() : config.desiredSize);
        commitInteractiveLayerState(widget, nativeWindow, createIfNeeded);
        return true;
    }

    static LayerShellQt::Window::KeyboardInteractivity keyboardInteractivity(
        markshot::layershell::KeyboardInteractivity value)
    {
        switch (value) {
        case markshot::layershell::KeyboardInteractivity::Exclusive:
            return LayerShellQt::Window::KeyboardInteractivityExclusive;
        case markshot::layershell::KeyboardInteractivity::OnDemand:
            return LayerShellQt::Window::KeyboardInteractivityOnDemand;
        }
        return LayerShellQt::Window::KeyboardInteractivityExclusive;
    }
};

#include "layer_shell_plugin.moc"
