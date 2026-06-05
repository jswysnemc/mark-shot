#pragma once

#include <QString>
#include <QtPlugin>

class QScreen;
class QWidget;

namespace markshot::layershell {

enum class KeyboardInteractivity {
    Exclusive,
    OnDemand,
};

struct OverlayConfig {
    QString scope;
    KeyboardInteractivity keyboardInteractivity = KeyboardInteractivity::Exclusive;
    bool closeOnDismissed = true;
    bool wantsActiveScreenWhenNoScreen = true;
    bool activateOnShow = true;
};

class PluginInterface {
public:
    virtual ~PluginInterface() = default;
    virtual bool configureOverlay(QWidget *widget, QScreen *screen, const OverlayConfig &config) = 0;
};

} // namespace markshot::layershell

#define MARK_SHOT_LAYER_SHELL_PLUGIN_IID "dev.mark-shot.LayerShellPlugin/1.0"

Q_DECLARE_INTERFACE(markshot::layershell::PluginInterface, MARK_SHOT_LAYER_SHELL_PLUGIN_IID)
