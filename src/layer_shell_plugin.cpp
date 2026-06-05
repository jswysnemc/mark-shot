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

}  // namespace

class MarkShotLayerShellPlugin final
    : public QObject
    , public markshot::layershell::PluginInterface
{
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

private:
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
