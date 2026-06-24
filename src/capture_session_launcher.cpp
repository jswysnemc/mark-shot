#include "capture_session_launcher.h"

#include "annotation_launch.h"
#include "capture_geometry.h"
#include "debug_log.h"
#include "display_capture/display_capture_snapshot.h"
#include "screen_capture.h"
#include "window_detection.h"
#include "windows_integration.h"

#include <QApplication>
#include <QEventLoop>
#include <QGuiApplication>
#include <QImage>
#include <QMessageBox>
#include <QPointer>
#include <QScreen>
#include <QTimer>

#include <cmath>
#include <memory>
#include <optional>
#include <utility>

namespace {

struct CapturedScreenFrame {
    QPointer<QScreen> screen;
    QImage image;
    QString outputName;
    QRect sourceGeometry;
    QVector<markshot::WindowInfo> windowInfos;
    bool detectWindows = false;
};

/// @brief 返回冻结范围日志名称。
/// @param scope 冻结范围枚举值。
/// @return 日志使用的固定字符串。
const char *freezeScopeDebugName(markshot::CaptureFreezeScope scope)
{
    switch (scope) {
    case markshot::CaptureFreezeScope::CursorScreen:
        return "cursor-screen";
    case markshot::CaptureFreezeScope::AllScreens:
        return "all-screens";
    }
    return "unknown";
}

/// @brief 判断当前窗口系统是否为 Wayland。
/// @return 当前 Qt 平台为 Wayland 时返回 true。
bool isWaylandPlatform()
{
    return QGuiApplication::platformName().compare(QStringLiteral("wayland"),
                                                   Qt::CaseInsensitive) == 0;
}

/// @brief 判断屏幕列表是否包含不同的设备像素比例。
/// @param screens 当前屏幕列表。
/// @return 存在两个以上不同设备像素比例时返回 true。
bool hasMixedDevicePixelRatios(const QList<QScreen *> &screens)
{
    std::optional<qreal> firstRatio;
    for (QScreen *screen : screens) {
        if (!screen || screen->geometry().isEmpty()) {
            continue;
        }

        const qreal ratio = screen->devicePixelRatio();
        if (!firstRatio.has_value()) {
            firstRatio = ratio;
            continue;
        }
        if (std::abs(*firstRatio - ratio) > 0.01) {
            return true;
        }
    }
    return false;
}

/// @brief 判断多屏冻结是否需要逐屏捕获。
/// @param screens 当前屏幕列表。
/// @return Wayland 多屏场景返回 true。
bool shouldCaptureScreensIndividually(const QList<QScreen *> &screens)
{
    return isWaylandPlatform() && screens.size() > 1;
}

/// @brief 记录截图会话中的屏幕缩放诊断信息。
/// @param screens 当前屏幕列表。
/// @return 无返回值。
void logCaptureSessionScreens(const QList<QScreen *> &screens)
{
    int index = 0;
    for (QScreen *screen : screens) {
        if (!screen) {
            markshot::debugLog("capture-session",
                               "【截图会话】【缩放诊断】screen index=%d null=1",
                               index++);
            continue;
        }

        const QRect geometry = screen->geometry();
        const QRect available = screen->availableGeometry();
        markshot::debugLog("capture-session",
                           "【截图会话】【缩放诊断】screen index=%d name=%s geom=%d,%d %dx%d "
                           "available=%d,%d %dx%d dpr=%.3f logical_dpi=%.3fx%.3f "
                           "physical_dpi=%.3fx%.3f refresh=%.3f",
                           index++,
                           screen->name().toUtf8().constData(),
                           geometry.x(), geometry.y(), geometry.width(), geometry.height(),
                           available.x(), available.y(), available.width(), available.height(),
                           screen->devicePixelRatio(),
                           screen->logicalDotsPerInchX(), screen->logicalDotsPerInchY(),
                           screen->physicalDotsPerInchX(), screen->physicalDotsPerInchY(),
                           screen->refreshRate());
    }
}

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

/// @brief 按显示器名称查找屏幕。
/// @param screenName 显示器名称。
/// @return 匹配到的屏幕；未找到时返回 nullptr。
QScreen *screenByName(const QString &screenName)
{
    if (screenName.isEmpty()) {
        return nullptr;
    }
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (screen && screen->name() == screenName) {
            return screen;
        }
    }
    return nullptr;
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
                               QVector<markshot::WindowInfo> windowInfos,
                               bool detectWindows,
                               bool allOutputs,
                               bool useRegularWindow,
                               bool fullscreenAnnotation,
                               const markshot::DefaultTools &defaultTools)
{
    ShotWindow *window =
        new ShotWindow(std::move(image), std::move(outputName), sourceGeometry, std::move(windowInfos), detectWindows);
    window->setDefaultTools(defaultTools.normal, defaultTools.fullscreen);
    if (markshot::shouldApplyDefaultColor(defaultTools)) {
        window->setDefaultColor(defaultTools.color);
    }
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
    const QVector<markshot::WindowInfo> windowInfos = detectWindows
        ? markshot::collectConfiguredWindowInfos(captureGeometry, outputName, allOutputs)
        : QVector<markshot::WindowInfo>();
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
                              windowInfos,
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

/// @brief 显示一组截图窗口。
/// @param windows 需要显示的截图窗口列表。
/// @return 无返回值。
void showCaptureWindows(const QVector<QPointer<ShotWindow>> &windows)
{
    for (const QPointer<ShotWindow> &window : windows) {
        if (window) {
            window->show();
            window->raise();
        }
    }
}

/// @brief 按显示器快照目标创建并显示全屏标注窗口。
/// @param target 显示器快照目标。
/// @param useRegularWindow 是否使用普通窗口。
/// @param defaultTools 默认工具配置。
/// @param error 输出错误信息。
/// @return 创建出的截图窗口。
ShotWindow *showDisplayCaptureTarget(const markshot::display_capture::Target &target,
                                     bool useRegularWindow,
                                     const markshot::DefaultTools &defaultTools,
                                     QString *error)
{
    if (target.image.isNull()) {
        if (error) {
            *error = QStringLiteral("display capture image is empty");
        }
        return nullptr;
    }

    QScreen *screen = nullptr;
    if (!target.allOutputs) {
        screen = screenByName(target.screenName);
        if (!screen) {
            screen = QGuiApplication::screenAt(target.geometry.center());
        }
    }

    const bool detectWindows = markshot::windowDetectionEnabled();
    const QVector<markshot::WindowInfo> windowInfos = detectWindows
        ? markshot::collectConfiguredWindowInfos(target.geometry, target.outputName, target.allOutputs)
        : QVector<markshot::WindowInfo>();
    return showCapturedWindow(screen,
                              target.image,
                              target.outputName,
                              target.geometry,
                              windowInfos,
                              detectWindows,
                              target.allOutputs,
                              useRegularWindow,
                              true,
                              defaultTools);
}

/// @brief 为截图冻结窗口绑定互斥关闭和显示器快速截取逻辑。
/// @param app 应用对象。
/// @param windows 冻结窗口列表。
/// @param includeCursor 冻结图是否包含鼠标。
/// @param useRegularWindow 是否使用普通窗口。
/// @param defaultTools 默认工具配置。
/// @return 无返回值。
void connectCaptureWindowSession(QApplication *app,
                                 const QVector<QPointer<ShotWindow>> &windows,
                                 bool includeCursor,
                                 bool useRegularWindow,
                                 const markshot::DefaultTools &defaultTools)
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
        QObject::connect(window,
                         &ShotWindow::displayCaptureSnapshotRequested,
                         app,
                         [windows, closingSession, includeCursor](ShotWindow *activeWindow) {
            if (*closingSession) {
                return;
            }
            *closingSession = true;

            for (const QPointer<ShotWindow> &peerWindow : std::as_const(windows)) {
                if (peerWindow) {
                    peerWindow->hide();
                }
            }
            QApplication::processEvents(QEventLoop::ExcludeUserInputEvents);

            QString error;
            const QVector<markshot::display_capture::Target> targets =
                markshot::display_capture::captureDisplayTargets(includeCursor, &error);
            showCaptureWindows(windows);

            if (activeWindow && !targets.isEmpty()) {
                activeWindow->showDisplayCaptureTargets(targets);
            } else {
                QMessageBox::warning(nullptr,
                                     QStringLiteral("Mark Shot"),
                                     error.isEmpty()
                                         ? QStringLiteral("Display capture failed")
                                         : error);
            }

            *closingSession = false;
        });
        QObject::connect(window,
                         &ShotWindow::displayCaptureEditRequested,
                         app,
                         [app, windows, closingSession, includeCursor, useRegularWindow, defaultTools](
                             ShotWindow *, markshot::display_capture::Target target) {
            if (*closingSession) {
                return;
            }
            *closingSession = true;

            QString error;
            ShotWindow *newWindow = showDisplayCaptureTarget(target,
                                                             useRegularWindow,
                                                             defaultTools,
                                                             &error);

            if (newWindow) {
                QVector<QPointer<ShotWindow>> newWindows{QPointer<ShotWindow>(newWindow)};
                connectCaptureWindowSession(app, newWindows, includeCursor, useRegularWindow, defaultTools);
                closeCaptureWindows(windows);
            } else {
                showCaptureWindows(windows);
                QMessageBox::warning(nullptr,
                                     QStringLiteral("Mark Shot"),
                                     error.isEmpty()
                                         ? QStringLiteral("Display capture failed")
                                         : error);
            }

            *closingSession = false;
        });
    }
}

/// @brief 逐个显示器捕获冻结图,但暂不创建覆盖窗口。
/// @param screens 当前屏幕列表。
/// @param includeCursor 冻结图是否包含鼠标。
/// @param error 输出错误信息。
/// @return 捕获成功的逐屏冻结帧列表。
QVector<CapturedScreenFrame> captureScreensIndividually(const QList<QScreen *> &screens,
                                                        bool includeCursor,
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
        const QVector<markshot::WindowInfo> windowInfos = detectWindows
            ? markshot::collectConfiguredWindowInfos(captureGeometry, outputName, false)
            : QVector<markshot::WindowInfo>();

        CaptureRequest request;
        request.preferredOutputName = outputName;
        request.sourceGeometry = captureGeometry;
        request.allOutputs = false;
        request.includeCursor = includeCursor;

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
        frame.windowInfos = windowInfos;
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

/// @brief 通过逐屏捕获创建多显示器冻结窗口。
/// @param screens 当前屏幕列表。
/// @param includeCursor 冻结图是否包含鼠标。
/// @param useRegularWindow 是否使用普通窗口。
/// @param fullscreenAnnotation 是否直接进入全屏标注。
/// @param defaultTools 默认工具配置。
/// @param error 输出错误信息。
/// @return 创建出的截图窗口列表。
QVector<QPointer<ShotWindow>> showCaptureWindowsFromIndividualFrames(const QList<QScreen *> &screens,
                                                                     bool includeCursor,
                                                                     bool useRegularWindow,
                                                                     bool fullscreenAnnotation,
                                                                     const markshot::DefaultTools &defaultTools,
                                                                     QString *error)
{
    QVector<QPointer<ShotWindow>> windows;
    QVector<CapturedScreenFrame> frames = captureScreensIndividually(screens, includeCursor, error);
    if (frames.isEmpty()) {
        return windows;
    }

    for (CapturedScreenFrame &frame : frames) {
        if (!frame.screen) {
            continue;
        }

        // 2. 全部捕获完成后再显示窗口,保证冻结图不包含本应用覆盖层
        ShotWindow *window = showCapturedWindow(frame.screen.data(),
                                                std::move(frame.image),
                                                std::move(frame.outputName),
                                                frame.sourceGeometry,
                                                std::move(frame.windowInfos),
                                                frame.detectWindows,
                                                false,
                                                useRegularWindow,
                                                fullscreenAnnotation,
                                                defaultTools);
        windows.append(window);
    }

    return windows;
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
    markshot::debugLog("capture-session",
                       "【截图会话】【缩放诊断】single-frame-result virtual=%d,%d %dx%d "
                       "frame_geom=%d,%d %dx%d image=%dx%d scale=%.6fx%.6f",
                       virtualGeometry.x(), virtualGeometry.y(),
                       virtualGeometry.width(), virtualGeometry.height(),
                       frameGeometry.x(), frameGeometry.y(),
                       frameGeometry.width(), frameGeometry.height(),
                       capture.image.width(), capture.image.height(),
                       static_cast<qreal>(capture.image.width()) / std::max(1, frameGeometry.width()),
                       static_cast<qreal>(capture.image.height()) / std::max(1, frameGeometry.height()));
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

        markshot::debugLog("capture-session",
                           "【截图会话】【缩放诊断】single-frame-crop screen=%s geom=%d,%d %dx%d "
                           "dpr=%.3f image=%dx%d scale=%.6fx%.6f",
                           screen->name().toUtf8().constData(),
                           screenGeometry.x(), screenGeometry.y(),
                           screenGeometry.width(), screenGeometry.height(),
                           screen->devicePixelRatio(),
                           screenImage.width(), screenImage.height(),
                           static_cast<qreal>(screenImage.width()) / std::max(1, screenGeometry.width()),
                           static_cast<qreal>(screenImage.height()) / std::max(1, screenGeometry.height()));

        const QVector<markshot::WindowInfo> windowInfos = detectWindows
            ? markshot::collectConfiguredWindowInfos(screenGeometry, screen->name(), false)
            : QVector<markshot::WindowInfo>();
        ShotWindow *window = showCapturedWindow(screen,
                                                std::move(screenImage),
                                                screen->name(),
                                                screenGeometry,
                                                windowInfos,
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
    const bool freezeAllScreens = shouldFreezeAllScreens(allOutputs,
                                                         fullscreenAnnotation,
                                                         freezeScope,
                                                         screens.size());
    const bool waylandPlatform = isWaylandPlatform();
    const bool mixedDevicePixelRatios = hasMixedDevicePixelRatios(screens);
    const bool captureIndividually = shouldCaptureScreensIndividually(screens);
    markshot::debugLog("capture-session",
                       "【截图会话】【缩放诊断】session all_outputs=%d fullscreen_annotation=%d "
                       "freeze_scope=%s screen_count=%d focused=%s platform=%s wayland=%d "
                       "mixed_dpr=%d freeze_all_screens=%d individual=%d include_cursor=%d "
                       "regular_window=%d",
                       allOutputs ? 1 : 0,
                       fullscreenAnnotation ? 1 : 0,
                       freezeScopeDebugName(freezeScope),
                       static_cast<int>(screens.size()),
                       screen ? screen->name().toUtf8().constData() : "(none)",
                       QGuiApplication::platformName().toUtf8().constData(),
                       waylandPlatform ? 1 : 0,
                       mixedDevicePixelRatios ? 1 : 0,
                       freezeAllScreens ? 1 : 0,
                       captureIndividually ? 1 : 0,
                       includeCursor ? 1 : 0,
                       useRegularWindow ? 1 : 0);
    logCaptureSessionScreens(screens);
    if (freezeAllScreens) {
        if (captureIndividually) {
            windows = showCaptureWindowsFromIndividualFrames(screens,
                                                             includeCursor,
                                                             useRegularWindow,
                                                             fullscreenAnnotation,
                                                             defaultTools,
                                                             error);
        } else {
            windows = showCaptureWindowsFromSingleFrame(screens,
                                                        includeCursor,
                                                        useRegularWindow,
                                                        fullscreenAnnotation,
                                                        defaultTools,
                                                        error);
        }
        connectCaptureWindowSession(app, windows, includeCursor, useRegularWindow, defaultTools);
        return windows;
    }

    ShotWindow *window =
        showCaptureWindow(allOutputs ? nullptr : screen, allOutputs, includeCursor, useRegularWindow, fullscreenAnnotation, defaultTools, error);
    if (window) {
        windows.append(window);
    }
    connectCaptureWindowSession(app, windows, includeCursor, useRegularWindow, defaultTools);
    return windows;
}

}  // namespace markshot
