#include "screen_capture_internal.h"

CaptureResult captureScreenFrame(const CaptureRequest &request)
{
    // The public API hides platform selection from callers. Wayland needs portal,
    // PipeWire, grim, or compositor helpers; non-Wayland sessions can use Qt's
    // QScreen grab path directly. Windows prefers Windows Graphics Capture so
    // excluded overlay windows stay out of repeated scroll frames.
#if defined(Q_OS_WIN)
    CaptureResult result = captureWithWindowsGraphicsCapture(request);
    if (result.image.isNull()) {
        markshot::debugLog("capture",
                           "windows-graphics-capture-failed (falling back) error=%s",
                           result.error.toUtf8().constData());
        result = captureWithQScreen(request);
    }
#elif defined(MARK_SHOT_WITH_DBUS)
    CaptureResult result = isWaylandSession()
        ? captureWaylandFrame(request)
        : captureWithQScreen(request);
#else
    CaptureResult result = captureWithQScreen(request);
#endif
    // Downstream selection, annotation, and stitching code works in raw image
    // pixels, so normalize format/device-pixel-ratio at the backend boundary.
    result.image = normalizeCaptureImage(result.image);
    return result;
}

void stopActiveScreencastCapture()
{
#ifdef MARK_SHOT_WITH_DBUS
    stopPortalScreencast();
#endif
}
