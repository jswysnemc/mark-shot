#pragma once

#include <QImage>
#include <QRect>
#include <QSize>
#include <QString>

class ShotWindow;
class QScreen;

namespace markshot {

/**
 * 【截图会话】【屏幕选择】优先使用 GNOME 合成器指针或 niri 聚焦输出，再回退到 Qt 指针和主屏。
 * @return 新窗口应使用的屏幕，无参数。
 */
QScreen *focusedScreen();

QRect centeredImageWindowGeometry(const QSize &imageSize, QScreen *screen);

// Opens an existing/loaded image in a fullscreen-annotation ShotWindow,
// matching the behaviour of main.cpp's file mode. Used both by file mode and
// by the scrolling capture session when sending its stitched image back for
// annotation. Returns the created window (already shown).
ShotWindow *openImageForAnnotation(const QImage &image, const QString &name);

}  // namespace markshot
