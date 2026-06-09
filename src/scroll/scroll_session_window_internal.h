#pragma once

#include "scroll/scroll_session_window.h"

#include "annotation_launch.h"
#include "clipboard_image.h"
#include "debug_log.h"
#include "layer_shell_runtime.h"
#include "screen_capture.h"
#include "ui/i18n.h"
#include "ui/theme.h"
#include "windows_integration.h"

#include <QCloseEvent>
#include <QCoreApplication>
#include <QDateTime>
#ifdef MARK_SHOT_WITH_DBUS
#include <QDBusConnection>
#include <QDBusMessage>
#endif
#include <QDir>
#include <QEvent>
#include <QEventLoop>
#include <QFile>
#include <QFileDialog>
#include <QFont>
#include <QGuiApplication>
#include <QHBoxLayout>
#include <QIcon>
#include <QKeyEvent>
#include <QKeySequence>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPaintEvent>
#include <QPixmap>
#include <QPushButton>
#include <QRegion>
#include <QResizeEvent>
#include <QScreen>
#include <QShortcut>
#include <QSize>
#include <QShowEvent>
#include <QStandardPaths>
#include <QThread>
#include <QTimer>
#include <QVBoxLayout>
#include <QWheelEvent>
#include <QWindow>

#include <algorithm>
#include <cmath>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <cstdint>
#include <optional>
#include <utility>

namespace markshot::scroll {

// Capture cadence and duplicate-frame signature. The stitcher is expensive
// enough that static frames are filtered before pushFrame().
inline constexpr int kCaptureIntervalMs = 45;
inline constexpr int kSignatureCols = 18;
inline constexpr int kSignatureRows = 24;
inline constexpr float kDuplicateAvgDiff = 1.1f;
inline constexpr int kDuplicateMaxDiff = 4;

// Overlay and preview chrome dimensions. The frame is drawn outside the capture
// rectangle so it can be excluded from captured pixels.
#if defined(Q_OS_WIN)
inline constexpr int kCaptureFrameWidth = 2;
#else
inline constexpr int kCaptureFrameWidth = 3;
#endif
inline constexpr int kPanelWidth = 340;
inline constexpr int kPanelPadding = 12;
inline constexpr int kControlBarHeight = 54;
inline constexpr int kControlButtonWidth = 40;
inline constexpr int kControlButtonHeight = 36;
inline constexpr int kStatusHeight = 22;
inline constexpr int kPanelMargin = 4;
inline constexpr int kFloatingDragHandleGap = 6;
inline constexpr int kAntialiasPad = 2;
inline constexpr int kCaptureFrameArtifactScanPx = 10;
inline constexpr int kCaptureFrameArtifactDistance = 70;
inline constexpr int kGnomePreviewIntervalMs = 140;
inline constexpr int kScrollIdlePauseMs = 1000;

// GNOME shell helper exposes a DBus preview surface when normal overlay windows
// cannot be kept out of captured content.
inline constexpr auto kGnomeShellService = "org.gnome.Shell";
inline constexpr auto kGnomeHelperPath = "/org/gnome/Shell/Extensions/MarkShotScrollHelper";
inline constexpr auto kGnomeHelperInterface = "org.gnome.Shell.Extensions.MarkShotScrollHelper";

enum class ControlIcon {
    AxisVertical,
    AxisHorizontal,
    Pause,
    Resume,
    Annotate,
    Save,
    Copy,
    Cancel,
};

struct PreviewPanelPlacement {
    QRect rect;
    bool fitsWithoutOverlap = true;
};

// Low-resolution frame signatures detect unchanged capture ticks before they
// reach the stitcher. They are intentionally approximate and cheap.
std::uint8_t grayPixel(const QImage &frame, int x, int y);
QVector<std::uint8_t> frameSignature(const QImage &frame, int cols, int rows);
bool isDuplicateSignature(const QVector<std::uint8_t> &previous,
                          const QVector<std::uint8_t> &current);
QString scrollSavePath();

// Shared debug names keep stitcher and session-window logs comparable.
const char *algorithmDebugName();
bool isWaylandPlatform();
const char *axisDebugName(ScrollAxis axis);
const char *statusDebugName(StitchStatus status);
const char *edgeDebugName(StitchEdge edge);
void logScrollDebug(const char *format, ...);

// Some compositors capture one-pixel remnants of the overlay frame. These
// helpers identify and replace those edge rows/columns before stitching.
bool nearColor(QRgb pixel, int red, int green, int blue);
bool isCaptureFrameArtifactPixel(QRgb pixel);
bool rowLooksLikeCaptureFrameArtifact(const QImage &frame, int y);
bool columnLooksLikeCaptureFrameArtifact(const QImage &frame, int x);
std::optional<int> replacementRowForEdgeArtifact(const QImage &frame, bool topEdge);
std::optional<int> replacementColumnForEdgeArtifact(const QImage &frame, bool leftEdge);
void copyImageRow(QImage *frame, int dstY, int srcY);
void copyImageColumn(QImage *frame, int dstX, int srcX);
int scrubCaptureFrameArtifacts(QImage *frame);
QPen iconPen(const QColor &color, qreal width = 1.8);

// Overlay geometry helpers keep painting, input masks, and preview placement in
// sync for layer-shell, regular-window, and panel-only fallback modes.
QRect captureFrameOuterRect(QRect region, int frameGap);
QRect captureFrameInnerRect(QRect region, int frameGap);
QSize previewPanelSizeForAnchor(const QRect &anchor, const QRect &bounds);
PreviewPanelPlacement choosePreviewPanelPlacement(const QRect &anchor,
                                                  const QRect &bounds,
                                                  const QSize &panelSize,
                                                  int previewGap);
QSize controlButtonSize();
QRect chooseFloatingDragHandleRect(const QRect &anchor, const QRect &bounds);
QIcon makeControlIcon(ControlIcon icon);
void configureIconButton(QPushButton *button, const QIcon &icon, const QString &label);
void applyControlButtonChrome(QPushButton *button);

}  // namespace markshot::scroll
