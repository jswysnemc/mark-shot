#pragma once

#include "window_detection.h"

#include <QImage>
#include <QRect>
#include <QString>
#include <QVector>

// Result returned by any capture backend. sourceGeometry describes the global
// compositor coordinates represented by image, which can differ from the
// requested geometry when a backend captures a full screen and crops later.
struct CaptureResult {
    QImage image;
    QString error;
    QString outputName;
    QRect sourceGeometry;
    // 后端已经把鼠标写入 image 时为 true
    bool cursorIncluded = false;
};

// Backend-independent capture request. Callers can ask for all outputs, one
// named output, or a source rectangle; platform code chooses the safest backend
// available for the current session and capability flags.
struct CaptureRequest {
    QString preferredOutputName; // Best-effort monitor name when a single output is desired.
    QRect sourceGeometry;        // Global capture rectangle before backend-specific cropping.
    bool allOutputs = false;     // Capture the virtual desktop instead of one output/rectangle.
    bool preferScreencast = false; // Prefer reusable stream capture for repeated scroll frames.
    bool allowInteractivePortal = true; // Permit user-facing portal prompts for one-shot capture.
    bool allowPortalScreenshotFallback = true; // Allow slower portal screenshots if streaming fails.
    qint64 minimumFrameTimeMs = 0; // Ignore stale stream frames captured before this timestamp.
    // 捕获冻结图时是否包含鼠标
    bool includeCursor = false;
};

// Captures one frame and normalizes the image for downstream painting/stitching.
CaptureResult captureScreenFrame(const CaptureRequest &request);
// Stops a reusable screencast session when scrolling capture pauses or fails.
void stopActiveScreencastCapture();
// Returns visible X11 window frames for selection snapping/highlighting.
QVector<QRect> enumerateX11WindowGeometries();
QVector<markshot::WindowInfo> enumerateX11WindowInfos();
bool isGnomeWaylandSession();
bool hasGnomeScrollHelper();
bool hasGnomeScrollPreviewHelper();
bool hasGnomeScrollOverlayHelper();
