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
    // 流式后端产生该帧的时间，0 表示后端没有提供
    qint64 frameTimeMs = 0;
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
    // 流式后端可按该帧率限制捕获节奏，0 表示保持后端默认行为
    int targetFps = 0;
    // 捕获冻结图时是否包含鼠标
    bool includeCursor = false;
    // 截屏时是否让后端隐藏调用者自身窗口（KWin hide-caller-windows 选项）
    bool hideOwnWindows = true;
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
