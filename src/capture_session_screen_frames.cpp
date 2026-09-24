#include "capture_session_screen_frames.h"

#include "debug_log.h"
#include "screen_capture.h"

#include <algorithm>
#include <utility>

namespace markshot::capture_session {

QVector<CapturedScreenFrame> captureScreensIndividually(const QList<QScreen *> &screens,
                                                        bool includeCursor,
                                                        bool hideOwnWindows,
                                                        QString *error)
{
    QVector<CapturedScreenFrame> frames;
    const bool detectWindows = markshot::windowDetectionEnabled();

    for (QScreen *screen : screens) {
        if (!screen || screen->geometry().isEmpty()) {
            continue;
        }

        const QRect captureGeometry = screen->geometry();
        const QString outputName = screen->name();

        CaptureRequest request;
        request.preferredOutputName = outputName;
        request.sourceGeometry = captureGeometry;
        request.allOutputs = false;
        request.includeCursor = includeCursor;
        request.hideOwnWindows = hideOwnWindows;

        markshot::debugLog("capture-session",
                           "【截图会话】【缩放诊断】individual-request screen=%s geom=%d,%d %dx%d "
                           "dpr=%.3f include_cursor=%d",
                           outputName.toUtf8().constData(),
                           captureGeometry.x(), captureGeometry.y(),
                           captureGeometry.width(), captureGeometry.height(),
                           screen->devicePixelRatio(),
                           includeCursor ? 1 : 0);

        // 1. 先捕获所有屏幕图像,避免已显示的截图覆盖层进入后续屏幕截图
        CaptureResult capture = captureScreenFrame(request);
        if (capture.image.isNull()) {
            if (error) {
                *error = capture.error;
            }
            return {};
        }

        CapturedScreenFrame frame;
        frame.screen = screen;
        frame.image = std::move(capture.image);
        frame.outputName = capture.outputName.isEmpty() ? outputName : capture.outputName;
        frame.sourceGeometry = capture.sourceGeometry.isValid() && !capture.sourceGeometry.isEmpty()
            ? capture.sourceGeometry
            : captureGeometry;
        frame.windowInfos = detectWindows
            ? markshot::collectConfiguredWindowInfos(frame.sourceGeometry, frame.outputName, false)
            : QVector<markshot::WindowInfo>();
        frame.detectWindows = detectWindows;
        markshot::debugLog("capture-session",
                           "【截图会话】【缩放诊断】individual-result screen=%s output=%s "
                           "source=%d,%d %dx%d image=%dx%d scale=%.6fx%.6f",
                           outputName.toUtf8().constData(),
                           frame.outputName.toUtf8().constData(),
                           frame.sourceGeometry.x(), frame.sourceGeometry.y(),
                           frame.sourceGeometry.width(), frame.sourceGeometry.height(),
                           frame.image.width(), frame.image.height(),
                           static_cast<qreal>(frame.image.width()) / std::max(1, frame.sourceGeometry.width()),
                           static_cast<qreal>(frame.image.height()) / std::max(1, frame.sourceGeometry.height()));
        frames.append(std::move(frame));
    }

    return frames;
}

}  // namespace markshot::capture_session
