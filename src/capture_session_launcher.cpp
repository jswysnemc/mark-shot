#include "capture_session_launcher.h"

#include "annotation_launch.h"
#include "screen_capture.h"
#include "window_detection.h"
#include "windows_integration.h"

#include <QApplication>
#include <QGuiApplication>
#include <QPointer>
#include <QScreen>
#include <QTimer>

#include <memory>
#include <utility>

namespace {

/// @brief 计算全部显示器组成的虚拟桌面几何。
/// @return 虚拟桌面几何。
QRect virtualScreensGeometry()
{
    QRect geometry;
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (!screen) {
            continue;
        }
        geometry = geometry.isNull() ? screen->geometry() : geometry.united(screen->geometry());
    }
    return geometry;
}

/// @brief 捕获一个显示器并显示截图窗口。
/// @param screen 要捕获的显示器。
/// @param allOutputs 是否捕获全部输出为一张图片。
/// @param includeCursor 冻结图是否包含鼠标。
/// @param useRegularWindow 是否使用普通窗口。
/// @param fullscreenAnnotation 是否直接进入全屏标注。
/// @param defaultTools 默认工具配置。
/// @param error 输出错误信息。
/// @return 创建出的截图窗口。
ShotWindow *showCaptureWindow(QScreen *screen,
                              bool allOutputs,
                              bool includeCursor,
                              bool useRegularWindow,
                              bool fullscreenAnnotation,
                              const markshot::DefaultTools &defaultTools,
                              QString *error)
{
    const QRect captureGeometry = allOutputs ? virtualScreensGeometry() : (screen ? screen->geometry() : QRect());
    const QString outputName = (!allOutputs && screen) ? screen->name() : QString();
    const bool detectWindows = markshot::windowDetectionEnabled();
    const QVector<QRect> windowGeometries = detectWindows
        ? markshot::collectConfiguredWindowGeometries(captureGeometry, outputName, allOutputs)
        : QVector<QRect>();
    CaptureRequest request;
    request.preferredOutputName = outputName;
    request.sourceGeometry = captureGeometry;
    request.allOutputs = allOutputs;
    request.includeCursor = includeCursor;
    CaptureResult capture = captureScreenFrame(request);
    if (capture.image.isNull()) {
        if (error) {
            *error = capture.error;
        }
        return nullptr;
    }

    const QRect sourceGeometry = capture.sourceGeometry.isValid() && !capture.sourceGeometry.isEmpty()
        ? capture.sourceGeometry
        : captureGeometry;
    ShotWindow *window =
        new ShotWindow(capture.image, capture.outputName, sourceGeometry, windowGeometries, detectWindows);
    window->setDefaultTools(defaultTools.normal, defaultTools.fullscreen);
    window->setDefaultColor(defaultTools.color);
    if (screen && !allOutputs) {
        window->setScreen(screen);
    }

    const bool layerShellReady = !allOutputs && !useRegularWindow && window->configureLayerShell(screen);
    if (layerShellReady) {
        window->show();
    } else {
        if (capture.sourceGeometry.isValid() && !capture.sourceGeometry.isEmpty()) {
            window->setGeometry(capture.sourceGeometry);
        }
        if (allOutputs) {
            window->show();
        } else {
            markshot::windows::showFullScreenOnScreen(window, screen);
        }
        window->raise();
        window->activateWindow();
    }

    if (fullscreenAnnotation) {
        QTimer::singleShot(0, window, [window] {
            window->startFullscreenAnnotation();
        });
    }

    return window;
}

/// @brief 判断普通区域截图是否应冻结全部显示器。
/// @param allOutputs 是否显式捕获全部输出为一张图片。
/// @param fullscreenAnnotation 是否直接进入全屏标注。
/// @param freezeScope 配置的冻结范围。
/// @param screenCount 当前显示器数量。
/// @return 需要为每个显示器创建冻结窗口时返回 true。
bool shouldFreezeAllScreens(bool allOutputs,
                            bool fullscreenAnnotation,
                            markshot::CaptureFreezeScope freezeScope,
                            int screenCount)
{
    return !allOutputs
        && !fullscreenAnnotation
        && freezeScope == markshot::CaptureFreezeScope::AllScreens
        && screenCount > 1;
}

}  // namespace

namespace markshot {

QVector<QPointer<ShotWindow>> showCaptureSession(QApplication *app,
                                                 bool allOutputs,
                                                 CaptureFreezeScope freezeScope,
                                                 bool includeCursor,
                                                 bool useRegularWindow,
                                                 bool fullscreenAnnotation,
                                                 const DefaultTools &defaultTools,
                                                 QString *error)
{
    QVector<QPointer<ShotWindow>> windows;
    QScreen *screen = markshot::focusedScreen();
    const QList<QScreen *> screens = QGuiApplication::screens();
    if (shouldFreezeAllScreens(allOutputs, fullscreenAnnotation, freezeScope, screens.size())) {
        for (QScreen *candidate : screens) {
            if (!candidate || candidate->geometry().isEmpty()) {
                continue;
            }
            ShotWindow *window =
                showCaptureWindow(candidate, false, includeCursor, useRegularWindow, fullscreenAnnotation, defaultTools, error);
            if (!window) {
                for (const QPointer<ShotWindow> &existingWindow : std::as_const(windows)) {
                    if (existingWindow) {
                        existingWindow->close();
                    }
                }
                windows.clear();
                return windows;
            }
            windows.append(window);
        }

        if (!windows.isEmpty()) {
            auto closingSession = std::make_shared<bool>(false);
            for (const QPointer<ShotWindow> &candidateWindow : std::as_const(windows)) {
                ShotWindow *window = candidateWindow.data();
                if (!window) {
                    continue;
                }
                QObject::connect(window, &ShotWindow::selectionActivated, app, [windows, closingSession](ShotWindow *activeWindow) {
                    if (*closingSession) {
                        return;
                    }
                    *closingSession = true;
                    for (const QPointer<ShotWindow> &peerWindow : std::as_const(windows)) {
                        if (peerWindow && peerWindow.data() != activeWindow) {
                            peerWindow->close();
                        }
                    }
                    *closingSession = false;
                });
                QObject::connect(window, &ShotWindow::sessionCancelRequested, app, [windows, closingSession] {
                    if (*closingSession) {
                        return;
                    }
                    *closingSession = true;
                    for (const QPointer<ShotWindow> &peerWindow : std::as_const(windows)) {
                        if (peerWindow) {
                            peerWindow->close();
                        }
                    }
                    *closingSession = false;
                });
            }
        }
        return windows;
    }

    ShotWindow *window =
        showCaptureWindow(allOutputs ? nullptr : screen, allOutputs, includeCursor, useRegularWindow, fullscreenAnnotation, defaultTools, error);
    if (window) {
        windows.append(window);
    }
    return windows;
}

}  // namespace markshot
