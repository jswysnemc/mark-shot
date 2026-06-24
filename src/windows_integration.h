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
void showFullScreenOnScreen(QWidget *widget, QScreen *screen);
/// @brief 将窗口切换为 Windows 原生置顶或取消置顶。
/// @param widget 要处理的窗口。
/// @param alwaysOnTop 是否启用原生置顶。
void setWindowTopMost(QWidget *widget, bool alwaysOnTop);
/// @brief 使用 Windows 原生置顶层级提升窗口。
/// @param widget 要提升的窗口。
void raiseTopMostWindow(QWidget *widget);

} // namespace markshot::windows
