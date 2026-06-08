#pragma once

class QRect;
class QWidget;

namespace markshot::shot {

/// @brief Applies the configured pinned-window topmost state to a window.
/// @param window Pinned image window to update.
/// @param alwaysOnTop Whether the window should stay above normal windows.
void applyPinnedWindowTopState(QWidget *window, bool alwaysOnTop);

/// @brief Raises a pinned window using the platform-specific topmost path when available.
/// @param window Pinned image window to raise.
void raisePinnedWindowOnPlatform(QWidget *window);

/// @brief Synchronizes protocol-level geometry for pinned topmost windows.
/// @param window Pinned image window to synchronize.
void syncPinnedWindowTopGeometry(QWidget *window);

/// @brief Synchronizes protocol-level geometry using an explicit global rectangle.
/// @param window Pinned image window to synchronize.
/// @param geometry Global logical window geometry.
void syncPinnedWindowTopGeometry(QWidget *window, const QRect &geometry);

/// @brief Checks whether pinned windows should use layer-shell for topmost behavior.
/// @return True when non-GNOME Wayland layer-shell should be used.
bool pinnedWindowUsesLayerShellTop();

/// @brief Checks whether a pinned window is currently configured as layer-shell.
/// @param window Pinned image window.
/// @return True when the window has an active layer-shell topmost role.
bool pinnedWindowHasLayerShellTop(QWidget *window);

}  // namespace markshot::shot
