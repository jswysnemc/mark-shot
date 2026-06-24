#pragma once

#include <QMargins>
#include <QSize>
#include <QString>
#include <QtPlugin>

class QScreen;
class QWidget;

namespace markshot::layershell {

enum class KeyboardInteractivity {
    Exclusive,
    OnDemand,
};

enum class Layer {
    Top,
    Overlay,
};

struct OverlayConfig {
    QString scope;
    KeyboardInteractivity keyboardInteractivity = KeyboardInteractivity::Exclusive;
    bool closeOnDismissed = true;
    bool wantsActiveScreenWhenNoScreen = true;
    bool activateOnShow = true;
};

struct FloatingOverlayConfig {
    QString scope;
    KeyboardInteractivity keyboardInteractivity = KeyboardInteractivity::OnDemand;
    bool closeOnDismissed = false;
    bool wantsActiveScreenWhenNoScreen = false;
    bool activateOnShow = true;
    QSize desiredSize;
    QMargins margins;
};

class PluginInterface {
public:
    virtual ~PluginInterface() = default;
    virtual bool configureOverlay(QWidget *widget, QScreen *screen, const OverlayConfig &config) = 0;
    virtual bool configureFloatingOverlay(QWidget *widget,
                                          QScreen *screen,
                                          const FloatingOverlayConfig &config) = 0;
    virtual bool updateFloatingOverlay(QWidget *widget,
                                       QScreen *screen,
                                       const FloatingOverlayConfig &config) = 0;
    virtual bool setLayer(QWidget *widget, Layer layer) = 0;
};

} // namespace markshot::layershell

#define MARK_SHOT_LAYER_SHELL_PLUGIN_IID "dev.mark-shot.LayerShellPlugin/1.2"

/// @brief Declares the interface for the layer shell plugin.
Q_DECLARE_INTERFACE(markshot::layershell::PluginInterface, MARK_SHOT_LAYER_SHELL_PLUGIN_IID)
