#pragma once

#include "window_detection.h"

#include <QRect>
#include <QVector>

class QWidget;
class QScreen;

namespace markshot::windows {

QVector<QRect> enumerateWindowGeometries();
QVector<markshot::WindowInfo> enumerateWindowInfos();
void setExcludedFromCapture(QWidget *widget, bool excluded = true);
/// @brief 将窗口从系统任务栏/坞中排除（Windows: WS_EX_TOOLWINDOW；
/// X11: _NET_WM_STATE_SKIP_TASKBAR）。截图/滚动覆盖层不应出现在任务栏中。
/// @param widget 要处理的窗口。
/// @param excluded 是否从任务栏排除。
void setExcludedFromTaskbar(QWidget *widget, bool excluded = true);
void showFullScreenOnScreen(QWidget *widget, QScreen *screen);
/// @brief 当前 Qt 平台是否为 xcb（原生 X11 或 XWayland）。
/// @return 是 xcb 平台时返回 true。
bool isX11QtPlatform();
/// @brief 当前是否运行在真正的 X11/Xorg 会话（非 XWayland）。
/// @return 原生 X11 会话时返回 true。
bool isNativeX11Session();
/// @brief 将窗口切换为 Windows 原生置顶或取消置顶。
/// @param widget 要处理的窗口。
/// @param alwaysOnTop 是否启用原生置顶。
void setWindowTopMost(QWidget *widget, bool alwaysOnTop);
/// @brief 使用 Windows 原生置顶层级提升窗口。
/// @param widget 要提升的窗口。
void raiseTopMostWindow(QWidget *widget);

} // namespace markshot::windows
