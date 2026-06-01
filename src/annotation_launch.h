#pragma once

#include <QImage>
#include <QString>

class ShotWindow;
class QScreen;

namespace markshot {

// Resolves the screen that should host new windows (niri focused output,
// else the screen under the cursor, else primary). Shared by the capture
// entrypoint and the file/scroll annotation launchers.
QScreen *focusedScreen();

// Opens an existing/loaded image in a fullscreen-annotation ShotWindow,
// matching the behaviour of main.cpp's file mode. Used both by file mode and
// by the scrolling capture session when sending its stitched image back for
// annotation. Returns the created window (already shown).
ShotWindow *openImageForAnnotation(const QImage &image, const QString &name);

}  // namespace markshot
