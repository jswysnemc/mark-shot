#pragma once

#include "layer_shell_plugin_interface.h"

namespace markshot::layershell {

bool configureOverlay(QWidget *widget, QScreen *screen, const OverlayConfig &config);
bool configureFloatingOverlay(QWidget *widget, QScreen *screen, const FloatingOverlayConfig &config);
bool updateFloatingOverlay(QWidget *widget, QScreen *screen, const FloatingOverlayConfig &config);
bool setLayer(QWidget *widget, Layer layer);

} // namespace markshot::layershell
