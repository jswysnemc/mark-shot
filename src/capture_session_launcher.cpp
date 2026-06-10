#include "capture_session_launcher.h"

#include "annotation_launch.h"
#include "capture_geometry.h"
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

/// @brief 根据已经捕获的图像创建并显示截图窗口。
/// @param screen 要显示窗口的屏幕。
/// @param image 截图图像。
/// @param outputName 输出名称。
/// @param sourceGeometry 图像对应的全局逻辑几何。
/// @param windowGeometries 窗口检测得到的边界列表。
/// @param detectWindows 是否启用窗口检测。
/// @param allOutputs 是否显示为跨屏窗口。
/// @param useRegularWindow 是否使用普通窗口。
/// @param fullscreenAnnotation 是否直接进入全屏标注。
/// @param defaultTools 默认工具配置。
/// @return 创建出的截图窗口。
ShotWindow *showCapturedWindow(QScreen *screen,
                               QImage image,
                               QString outputName,
                               QRect sourceGeometry,
                               QVector<QRect> windowGeometries,
                               bool detectWindows,
                               bool allOutputs,
                               bool useRegularWindow,
                               bool fullscreenAnnotation,
                               const markshot::DefaultTools &defaultTools)
{
    ShotWindow *window =
        new ShotWindow(std::move(image), std::move(outputName), sourceGeometry, std::move(windowGeometries), detectWindows);
    window->setDefaultTools(defaultTools.normal, defaultTools.fullscreen);
    window->setDefaultColor(defaultTools.color);
    if (screen && !allOutputs) {
        window->setScreen(screen);
    }

    const bool layerShellReady = !allOutputs && !useRegularWindow && window->configureLayerShell(screen);
    if (layerShellReady) {
        window->show();
    } else {
        if (sourceGeometry.isValid() && !sourceGeometry.isEmpty()) {
            window->setGeometry(sourceGeometry);
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
    const QString capturedOutputName = capture.outputName.isEmpty() ? outputName : capture.outputName;
    return showCapturedWindow(screen,
                              std::move(capture.image),
                              capturedOutputName,
                              sourceGeometry,
                              windowGeometries,
                              detectWindows,
                              allOutputs,
                              useRegularWindow,
                              fullscreenAnnotation,
                              defaultTools);
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

/// @brief 关闭一组截图窗口。
/// @param windows 需要关闭的截图窗口列表。
/// @return 无返回值。
void closeCaptureWindows(const QVector<QPointer<ShotWindow>> &windows)
{
    for (const QPointer<ShotWindow> &window : windows) {
        if (window) {
            window->close();
        }
    }
}

/// @brief 为多屏冻结窗口绑定互斥关闭逻辑。
/// @param app 应用对象。
/// @param windows 多屏冻结窗口列表。
/// @return 无返回值。
void connectCaptureWindowSession(QApplication *app, const QVector<QPointer<ShotWindow>> &windows)
{
    if (!app || windows.isEmpty()) {
        return;
    }

    auto closingSession = std::make_shared<bool>(false);
    for (const QPointer<ShotWindow> &candidateWindow : windows) {
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
            closeCaptureWindows(windows);
            *closingSession = false;
        });
    }
}

/// @brief 使用一次全屏截图为每个屏幕创建冻结窗口。
/// @param screens 当前屏幕列表。
/// @param includeCursor 冻结图是否包含鼠标。
/// @param useRegularWindow 是否使用普通窗口。
/// @param fullscreenAnnotation 是否直接进入全屏标注。
/// @param defaultTools 默认工具配置。
/// @param error 输出错误信息。
/// @return 创建出的截图窗口列表。
QVector<QPointer<ShotWindow>> showCaptureWindowsFromSingleFrame(const QList<QScreen *> &screens,
                                                                bool includeCursor,
                                                                bool useRegularWindow,
                                                                bool fullscreenAnnotation,
                                                                const markshot::DefaultTools &defaultTools,
                                                                QString *error)
{
    QVector<QPointer<ShotWindow>> windows;
    const QRect virtualGeometry = virtualScreensGeometry();
    if (virtualGeometry.isEmpty()) {
        if (error) {
            *error = QStringLiteral("no virtual screen geometry available for capture");
        }
        return windows;
    }

    CaptureRequest request;
    request.sourceGeometry = virtualGeometry;
    request.allOutputs = true;
    request.includeCursor = includeCursor;
    CaptureResult capture = captureScreenFrame(request);
    if (capture.image.isNull()) {
        if (error) {
            *error = capture.error;
        }
        return windows;
    }

    const QRect frameGeometry = capture.sourceGeometry.isValid() && !capture.sourceGeometry.isEmpty()
        ? capture.sourceGeometry
        : virtualGeometry;
    const bool detectWindows = markshot::windowDetectionEnabled();
    for (QScreen *screen : screens) {
        if (!screen || screen->geometry().isEmpty()) {
            continue;
        }

        const QRect screenGeometry = screen->geometry().intersected(frameGeometry);
        if (screenGeometry.isEmpty()) {
            continue;
        }

        QImage screenImage = markshot::capture::cropFrameToRequest(capture.image, frameGeometry, screenGeometry);
        if (screenImage.isNull()) {
            if (error) {
                *error = QStringLiteral("failed to crop shared capture for screen %1").arg(screen->name());
            }
            closeCaptureWindows(windows);
            windows.clear();
            return windows;
        }

        const QVector<QRect> windowGeometries = detectWindows
            ? markshot::collectConfiguredWindowGeometries(screenGeometry, screen->name(), false)
            : QVector<QRect>();
        ShotWindow *window = showCapturedWindow(screen,
                                                std::move(screenImage),
                                                screen->name(),
                                                screenGeometry,
                                                windowGeometries,
                                                detectWindows,
                                                false,
                                                useRegularWindow,
                                                fullscreenAnnotation,
                                                defaultTools);
        windows.append(window);
    }
    return windows;
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
        windows = showCaptureWindowsFromSingleFrame(screens,
                                                    includeCursor,
                                                    useRegularWindow,
                                                    fullscreenAnnotation,
                                                    defaultTools,
                                                    error);
        connectCaptureWindowSession(app, windows);
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
