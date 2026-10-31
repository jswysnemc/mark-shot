#include "annotation_launch.h"
#include "capture_cursor_policy.h"
#include "capture_freeze_scope.h"
#include "capture_history/history_window.h"
#include "capture_own_windows_policy.h"
#include "capture_session_launcher.h"
#include "cli/headless_capture.h"
#include "cli/image_pin_launch.h"
#include "capture_delay/capture_delay_option.h"
#include "capture_delay/capture_delay_scheduler.h"
#include "cli/recording_cli.h"
#include "clipboard_image.h"
#include "clipboard_paste/clipboard_pin_launch.h"
#include "debug_log.h"
#include "screen_capture_portal_guard.h"
#include "ipc/single_instance_ipc.h"
#include "memory/idle_memory_trimmer.h"
#include "memory/image_allocation_policy.h"
#include "region_detection/region_detection_scheduler.h"
#include "shot_ffi.h"
#include "recording/recording_session_manager.h"
#include "recording/ui/recording_overlay_service.h"
#include "shot_window.h"
#include "startup_config.h"
#include "ui/application_icon.h"
#include "ui/icons.h"
#include "ui/i18n.h"
#include "ui/interaction_cursor.h"
#include "ui/theme.h"
#include "window_detection.h"
#include "windows_tray_controller.h"

#include <QApplication>
#include <QCommandLineParser>
#include <QFileInfo>
#include <QFont>
#include <QGuiApplication>
#include <QImageReader>
#include <QLocalServer>
#include <QMessageBox>
#include <QPointer>
#include <QScreen>
#include <QStringList>
#include <QThread>
#include <QTimer>
#include <QVector>

#include <algorithm>
#include <memory>
#include <optional>
#include <utility>

namespace {

/**
 * 创建单实例 IPC 服务。
 * @param error 输出错误信息。
 * @return IPC 服务实例。
 */
std::unique_ptr<QLocalServer> createSingleInstanceServer(QString *error)
{
    return markshot::ipc::listenForSingleInstanceCommands(error);
}

} // namespace

/// @brief 初始化应用并启动所选截图、历史或录制入口
/// @param argc 命令行参数数量
/// @param argv 命令行参数数组
/// @return 应用退出码
int main(int argc, char *argv[])
{
    markshot::memory::configureImageAllocationPolicy();
    if (shot_abi_version() != SHOT_ABI_VERSION) {
        qCritical("【应用启动】【Rust接口】Unsupported core ABI version");
        return 1;
    }
    markshot::applyConfiguredEnvironment();

    QGuiApplication::setDesktopFileName(QStringLiteral("mark-shot"));
    markshot::disableQtPortalServicesForHostApp();

    QApplication app(argc, argv);
    markshot::ui::installInteractionCursorPolicy(&app);
    QApplication::setApplicationName(QStringLiteral("mark-shot"));
    QApplication::setApplicationDisplayName(QStringLiteral("Mark Shot"));
    QApplication::setApplicationVersion(QStringLiteral(MARK_SHOT_VERSION));
    QApplication::setWindowIcon(markshot::ui::applicationIcon());
    QFont applicationFont = app.font();
    applicationFont.setFamily(markshot::theme::uiFontFamily());
    app.setFont(applicationFont);
    markshot::theme::synchronizeToolTipPalette(app.palette());

    markshot::i18n::initialize();

    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral("Screenshot selection and annotation tool."));
    parser.addHelpOption();
    parser.addVersionOption();
    parser.addPositionalArgument(QStringLiteral("file"), QStringLiteral("Open an existing image file for annotation instead of capturing the screen."), QStringLiteral("[file]"));
    QCommandLineOption allOutputsOption({QStringLiteral("all-outputs"), QStringLiteral("all-output")},
                                        QStringLiteral("Capture all outputs instead of the current Qt screen."));
    QCommandLineOption xdgWindowOption(QStringLiteral("xdg-window"), QStringLiteral("Use a regular fullscreen xdg window instead of layer-shell."));
    QCommandLineOption fullscreenAnnotationOption({QStringLiteral("fullscreen"), QStringLiteral("full-screen")},
                                                  QStringLiteral("Skip region selection and annotate the full captured frame."));
    QCommandLineOption trayOption(QStringLiteral("tray"),
                                  QStringLiteral("Keep running in the system tray and register global hotkeys when supported."));
    QCommandLineOption captureOption(QStringLiteral("capture"),
                                     QStringLiteral("Capture once even when tray autostart is enabled."));
    QCommandLineOption pinImageOption(QStringLiteral("pin-image"),
                                      QStringLiteral("Open an image file directly as a pinned sticker."),
                                      QStringLiteral("path"));
    QCommandLineOption pinClipboardOption(QStringLiteral("pin-clipboard"),
        QStringLiteral("Pin clipboard text or image without taking a screenshot."));
    QCommandLineOption recordingStatusOption(QStringLiteral("recording-status"),
                                             QStringLiteral("Print the current recording status as JSON."));
    QCommandLineOption stopRecordingOption(QStringLiteral("stop-recording"),
                                           QStringLiteral("Stop the active recording through the running instance."));
    QCommandLineOption pauseRecordingOption(QStringLiteral("pause-recording"),
                                            QStringLiteral("Toggle pause for the active recording through the running instance."));
    QCommandLineOption defaultToolOption(QStringLiteral("default-tool"),
                                         QStringLiteral("Set the default annotation tool after a selected region. Also seeds fullscreen mode unless overridden. Supported: %1.")
                                             .arg(ShotWindow::supportedToolNames().join(QStringLiteral(", "))),
                                         QStringLiteral("tool"));
    QCommandLineOption fullscreenDefaultToolOption(QStringLiteral("fullscreen-default-tool"),
                                                   QStringLiteral("Set the default annotation tool for fullscreen annotation mode. Supported: %1.")
                                                       .arg(ShotWindow::supportedToolNames().join(QStringLiteral(", "))),
                                                   QStringLiteral("tool"));
    QCommandLineOption fileDefaultToolOption(QStringLiteral("file-default-tool"),
                                             QStringLiteral("Set the default annotation tool when opening an existing image file. Supported: %1.")
                                                 .arg(ShotWindow::supportedToolNames().join(QStringLiteral(", "))),
                                             QStringLiteral("tool"));
    QCommandLineOption defaultColorOption(QStringLiteral("default-color"),
                                          QStringLiteral("Set the default annotation color. Supported formats: #RRGGBB or #RRGGBBAA."),
                                          QStringLiteral("color"));
    QCommandLineOption delayOption(QStringLiteral("delay"),
                                   QStringLiteral("Wait the given number of seconds (0-60, decimals allowed) before capturing."),
                                   QStringLiteral("seconds"));
    QCommandLineOption historyOption(QStringLiteral("history"),
                                     QStringLiteral("Open screenshot history without taking a screenshot."));
    QCommandLineOption debugOption(QStringLiteral("debug"),
                                   QStringLiteral("Enable debug logging."));
    QCommandLineOption noDebugOption(QStringLiteral("no-debug"),
                                     QStringLiteral("Disable debug logging even if config or environment enables it."));
    QCommandLineOption debugLogOption(QStringLiteral("debug-log"),
                                      QStringLiteral("Write debug logs to the specified file path."),
                                      QStringLiteral("path"));
    parser.addOption(allOutputsOption);
    parser.addOption(xdgWindowOption);
    parser.addOption(fullscreenAnnotationOption);
    parser.addOption(trayOption);
    parser.addOption(captureOption);
    parser.addOption(pinImageOption);
    parser.addOption(pinClipboardOption);
    parser.addOption(recordingStatusOption);
    parser.addOption(stopRecordingOption);
    parser.addOption(pauseRecordingOption);
    parser.addOption(defaultToolOption);
    parser.addOption(fullscreenDefaultToolOption);
    parser.addOption(fileDefaultToolOption);
    parser.addOption(defaultColorOption);
    parser.addOption(debugOption);
    parser.addOption(noDebugOption);
    parser.addOption(debugLogOption);
    parser.addOption(historyOption);
    parser.addOption(delayOption);
    markshot::cli::addHeadlessCaptureOptions(&parser);
    parser.process(app);

    // 1. 【剪贴板】【命令入口】剪贴板贴图独立运行，不接受截图或录制操作混用
    if (parser.isSet(pinClipboardOption)) {
        for (const QString &option : {QStringLiteral("pin-image"), QStringLiteral("history"),
                QStringLiteral("capture"), QStringLiteral("tray"), QStringLiteral("all-outputs"),
                QStringLiteral("fullscreen"), QStringLiteral("xdg-window"), QStringLiteral("delay"),
                QStringLiteral("recording-status"), QStringLiteral("stop-recording"),
                QStringLiteral("pause-recording"), QStringLiteral("capture-to"), QStringLiteral("list-displays")}) {
            if (parser.isSet(option)) {
                parser.showHelp(1);
            }
        }
        if (!parser.positionalArguments().isEmpty()) {
            parser.showHelp(1);
        }
    }

    // 1. 【应用】【截图历史】历史入口独立运行，拒绝与截图或录制控制参数混用
    if (parser.isSet(historyOption)) {
        if (!parser.positionalArguments().isEmpty() || parser.isSet(pinImageOption)
            || parser.isSet(captureOption) || parser.isSet(trayOption)
            || parser.isSet(allOutputsOption) || parser.isSet(fullscreenAnnotationOption)
            || parser.isSet(stopRecordingOption) || parser.isSet(pauseRecordingOption)
            || parser.isSet(recordingStatusOption) || parser.isSet(QStringLiteral("capture-to"))
            || parser.isSet(QStringLiteral("list-displays")) || parser.isSet(delayOption)) {
            parser.showHelp(1);
        }
    }

    // 2. 【应用】【延时截图】先校验延时参数，错误时直接退出，避免进入截图流程后才报错
    int captureDelayMs = 0;
    if (parser.isSet(delayOption)) {
        QString delayError;
        const std::optional<int> parsedDelay =
            markshot::capture_delay::parseCaptureDelaySeconds(parser.value(delayOption), &delayError);
        if (!parsedDelay.has_value()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Invalid --delay value: %1").arg(delayError));
            return 1;
        }
        captureDelayMs = *parsedDelay;
    }

    if (parser.isSet(stopRecordingOption)) {
        return markshot::cli::stopRecordingFromCommandLine();
    }
    if (parser.isSet(pauseRecordingOption)) {
        return markshot::cli::togglePauseRecordingFromCommandLine();
    }
    if (parser.isSet(recordingStatusOption)) {
        return markshot::cli::printRecordingStatus();
    }

    const QStringList positionalArguments = parser.positionalArguments();
    const bool headlessRequested = parser.isSet(QStringLiteral("capture-to"))
        || parser.isSet(QStringLiteral("list-displays"));
    if (!headlessRequested && positionalArguments.size() > 1) {
        QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), MS_TR("Only one image file can be opened at a time."));
        return 1;
    }

    markshot::ensureAppConfigFile();

    if (parser.isSet(debugOption) && parser.isSet(noDebugOption)) {
        QMessageBox::critical(nullptr,
                              QStringLiteral("Mark Shot"),
                              MS_TR("--debug and --no-debug cannot be used together."));
        return 1;
    }

    markshot::DebugRuntimeConfig debugConfig = markshot::configuredDebugRuntimeConfig();
    if (parser.isSet(debugOption)) {
        debugConfig.enabled = true;
    }
    if (parser.isSet(noDebugOption)) {
        debugConfig.enabled = false;
    }
    if (parser.isSet(debugLogOption)) {
        const QString optionPath = parser.value(debugLogOption).trimmed();
        if (!optionPath.isEmpty()) {
            debugConfig.logPath = markshot::expandedConfigPath(optionPath);
        }
        if (!parser.isSet(noDebugOption)) {
            debugConfig.enabled = true;
        }
    }
    markshot::configureDebugLogging(debugConfig.enabled, debugConfig.logPath);
    markshot::debugLog("config",
                       "debug enabled path=%s",
                       markshot::debugLogPath().toUtf8().constData());

    // 【内存】【空闲回收】截图窗口关闭或退回托盘后回收空闲内存，录制期间推迟
    markshot::memory::IdleMemoryTrimmer::Options trimOptions;
    trimOptions.busy = [] {
        return markshot::recording::RecordingSessionManager::instance().status().active
            || markshot::region_detection::regionDetectionBusy();
    };
    markshot::memory::IdleMemoryTrimmer idleMemoryTrimmer(std::move(trimOptions));
    if (parser.isSet(historyOption)) {
        markshot::history::showHistoryWindow();
        return QApplication::exec();
    }
    // 【应用】【延时截图】无界面截图没有事件循环，直接阻塞等待后再截图
    if (captureDelayMs > 0 && parser.isSet(QStringLiteral("capture-to"))) {
        QThread::msleep(static_cast<unsigned long>(captureDelayMs));
    }
    const int headlessExitCode = markshot::cli::runHeadlessCaptureIfRequested(parser);
    if (headlessExitCode >= 0) {
        return headlessExitCode;
    }

    QString configDefaultToolWarning;
    markshot::DefaultTools defaultTools = markshot::configuredDefaultTools(&configDefaultToolWarning);
    auto parseRuntimeTool = [](const QString &optionValue) {
        return ShotWindow::toolFromName(optionValue);
    };
    if (parser.isSet(defaultToolOption)) {
        const QString optionValue = parser.value(defaultToolOption);
        const std::optional<ShotWindow::Tool> parsedTool = parseRuntimeTool(optionValue);
        if (!parsedTool.has_value()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Unsupported default tool: %1\nSupported tools: %2")
                                      .arg(optionValue, ShotWindow::supportedToolNames().join(QStringLiteral(", "))));
            return 1;
        }
        defaultTools.normal = *parsedTool;
        defaultTools.fullscreen = *parsedTool;
        defaultTools.file = *parsedTool;
        configDefaultToolWarning.clear();
    }
    if (parser.isSet(fullscreenDefaultToolOption)) {
        const QString optionValue = parser.value(fullscreenDefaultToolOption);
        const std::optional<ShotWindow::Tool> parsedTool = parseRuntimeTool(optionValue);
        if (!parsedTool.has_value()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Unsupported fullscreen default tool: %1\nSupported tools: %2")
                                      .arg(optionValue, ShotWindow::supportedToolNames().join(QStringLiteral(", "))));
            return 1;
        }
        defaultTools.fullscreen = *parsedTool;
        defaultTools.file = *parsedTool;
    }
    if (parser.isSet(fileDefaultToolOption)) {
        const QString optionValue = parser.value(fileDefaultToolOption);
        const std::optional<ShotWindow::Tool> parsedTool = parseRuntimeTool(optionValue);
        if (!parsedTool.has_value()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Unsupported file default tool: %1\nSupported tools: %2")
                                      .arg(optionValue, ShotWindow::supportedToolNames().join(QStringLiteral(", "))));
            return 1;
        }
        defaultTools.file = *parsedTool;
    }
    if (parser.isSet(defaultColorOption)) {
        const QString optionValue = parser.value(defaultColorOption);
        const std::optional<QColor> parsedColor = markshot::colorFromString(optionValue);
        if (!parsedColor.has_value()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Unsupported default color: %1\nSupported color formats: #RRGGBB or #RRGGBBAA")
                                      .arg(optionValue));
            return 1;
        }
        defaultTools.color = *parsedColor;
        defaultTools.colorSource = markshot::DefaultColorSource::CommandLine;
    }
    if (!configDefaultToolWarning.isEmpty()) {
        QMessageBox::warning(nullptr, QStringLiteral("Mark Shot"), configDefaultToolWarning);
    }

    const QString imagePath = positionalArguments.isEmpty() ? QString() : positionalArguments.first();
    if (parser.isSet(pinClipboardOption)) {
        QString error;
        if (!markshot::clipboard::pinContent(&error)) {
            fprintf(stderr, "%s\n", error.toLocal8Bit().constData());
            return 1;
        }
        return QApplication::exec();
    }
    if (parser.isSet(pinImageOption)) {
        if (!positionalArguments.isEmpty()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Only one image file can be opened at a time."));
            return 1;
        }

        QString error;
        QWidget *window = markshot::cli::launchPinnedImageFromPath(parser.value(pinImageOption), &error);
        if (!window) {
            QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), error);
            return 1;
        }
        return QApplication::exec();
    }

    const bool fileMode = !imagePath.isEmpty();
    if (fileMode) {
        QFileInfo imageFile(imagePath);
        if (!imageFile.exists() || !imageFile.isFile()) {
            QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), MS_TR("Image file does not exist: %1").arg(imagePath));
            return 1;
        }

        QImageReader reader(imageFile.absoluteFilePath());
        reader.setAutoTransform(true);
        QImage image = reader.read();
        if (image.isNull()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Failed to load image: %1\n%2").arg(imageFile.absoluteFilePath(), reader.errorString()));
            return 1;
        }

        // 1. 【图像编辑】【缓冲转移】尺寸单独保存，读入图像交给窗口，避免事件循环期间保留原始缓冲
        const QSize imageSize = image.size();
        ShotWindow *window = new ShotWindow(std::move(image), imageFile.fileName());
        window->setDefaultTools(defaultTools.normal, defaultTools.file);
        if (markshot::shouldApplyDefaultColor(defaultTools)) {
            window->setDefaultColor(defaultTools.color);
        }
        QScreen *screen = markshot::focusedScreen();
        if (screen) {
            window->setScreen(screen);
        }
        window->setWindowFlags(Qt::Window);
        const QRect windowGeometry = markshot::centeredImageWindowGeometry(imageSize, screen);
        if (windowGeometry.isValid() && !windowGeometry.isEmpty()) {
            window->setGeometry(windowGeometry);
        }
        window->setImageNavigationEnabled(true);
        window->show();
        window->raise();
        window->activateWindow();
        QTimer::singleShot(0, window, [window] {
            window->startFullscreenAnnotation();
        });
        return QApplication::exec();
    }

    const bool allOutputs = parser.isSet(allOutputsOption);
    const markshot::CaptureFreezeScope freezeScope = markshot::configuredCaptureFreezeScope();
    const bool useRegularWindow = parser.isSet(xdgWindowOption);
    const bool fullscreenAnnotation = parser.isSet(fullscreenAnnotationOption);
    const markshot::WindowsTrayController::Config trayConfig = markshot::WindowsTrayController::readConfig();
    const bool explicitCaptureRequest = parser.isSet(captureOption) || parser.isSet(allOutputsOption)
        || parser.isSet(fullscreenAnnotationOption) || parser.isSet(delayOption);
    const bool trayMode = !explicitCaptureRequest && (parser.isSet(trayOption) || trayConfig.autoStart);

    const bool explicitTrayOnly = parser.isSet(trayOption)
        && !parser.isSet(captureOption)
        && !parser.isSet(allOutputsOption)
        && !parser.isSet(fullscreenAnnotationOption)
        && !parser.isSet(delayOption);
    markshot::ipc::SingleInstanceCommand duplicateCommand;
    duplicateCommand.capture = !explicitTrayOnly;
    duplicateCommand.fullscreen = fullscreenAnnotation;
    duplicateCommand.allOutputs = allOutputs;
    duplicateCommand.captureDelayMs = captureDelayMs;
    if (markshot::ipc::sendSingleInstanceCommand(duplicateCommand, nullptr, nullptr)) {
        return 0;
    }

    QString singleInstanceError;
    std::unique_ptr<QLocalServer> singleInstanceServer = createSingleInstanceServer(&singleInstanceError);
    if (!singleInstanceServer) {
        if (markshot::ipc::sendSingleInstanceCommand(duplicateCommand, nullptr, nullptr)) {
            return 0;
        }
        QLocalServer::removeServer(markshot::ipc::singleInstanceServerName());
        singleInstanceServer = createSingleInstanceServer(&singleInstanceError);
    }
    if (!singleInstanceServer) {
        QMessageBox::critical(nullptr,
                              QStringLiteral("Mark Shot"),
                              MS_TR("Failed to start single-instance guard: %1").arg(singleInstanceError));
        return 1;
    }

    bool captureActive = false;
    auto launchCapture = [&app,
                          &captureActive,
                          useRegularWindow](bool startFullscreen,
                                        bool requestAllOutputs,
                                        std::optional<markshot::recording::RecordingOptions> regionRecordingOptions = std::nullopt) -> bool {
        if (captureActive) {
            return true;
        }
        // 门户选择弹窗使用嵌套事件循环，热键会在弹窗还在时再次进来。
        // 这时再开截图会话会叠出第二个 SelectSources 窗口。
        if (markshot::interactiveScreenCastInProgress()) {
            markshot::debugLog("capture-session",
                               "【截图会话】跳过：ScreenCast 门户选择尚未结束");
            return true;
        }

        QString captureError;
        markshot::DefaultTools defaultTools = markshot::configuredDefaultTools(nullptr);
        QVector<QPointer<ShotWindow>> windows =
            markshot::showCaptureSession(&app,
                                         requestAllOutputs,
                                         markshot::configuredCaptureFreezeScope(),
                                         markshot::configuredCaptureIncludeCursor(),
                                         markshot::configuredHideOwnWindowsDuringCapture(),
                                         useRegularWindow,
                                         startFullscreen,
                                         defaultTools,
                                         &captureError,
                                         std::move(regionRecordingOptions));
        if (windows.isEmpty()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  captureError.isEmpty() ? MS_TR("Failed to start capture session.") : captureError);
            return false;
        }

        captureActive = true;
        auto remainingWindows = std::make_shared<int>(0);
        for (const QPointer<ShotWindow> &window : std::as_const(windows)) {
            if (window) {
                ++(*remainingWindows);
            }
        }
        for (const QPointer<ShotWindow> &window : std::as_const(windows)) {
            if (!window) {
                continue;
            }
            QObject::connect(window, &QObject::destroyed, &app, [&captureActive, remainingWindows] {
                --(*remainingWindows);
                if (*remainingWindows <= 0) {
                    captureActive = false;
                }
            });
        }
        return true;
    };

    // 【应用】【延时截图】整个进程共用一个调度器，托盘、IPC 与命令行的延时请求互相替换
    markshot::capture_delay::CaptureDelayScheduler delayScheduler(&app);
    auto requestCapture = [launchCapture, &delayScheduler](bool startFullscreen,
                                                           bool requestAllOutputs,
                                                           int delayMs) {
        if (delayMs <= 0) {
            delayScheduler.cancel();
            return launchCapture(startFullscreen, requestAllOutputs);
        }
        delayScheduler.schedule(delayMs, [launchCapture, startFullscreen, requestAllOutputs] {
            launchCapture(startFullscreen, requestAllOutputs);
        });
        return true;
    };

    auto &recordingManager = markshot::recording::RecordingSessionManager::instance();
    // 录制期间的悬浮控制条与区域边框跟随会话状态自动出现和消失
    markshot::recording::ui::RecordingOverlayService::instance().attach();
    markshot::ipc::installSingleInstanceCommandHandler(
        singleInstanceServer.get(),
        &app,
        [&app, requestCapture, &recordingManager](const markshot::ipc::SingleInstanceCommand &command) {
            markshot::ipc::SingleInstanceResponse response;
            response.handled = true;

            if (command.stopRecording) {
                QString error;
                response.stopped = recordingManager.stop(&error);
                response.message = response.stopped
                    ? QStringLiteral("stop requested")
                    : (error.isEmpty() ? QStringLiteral("no active recording") : error);
                response.recording = recordingManager.status();
                return response;
            }

            if (command.pauseRecording || command.resumeRecording || command.togglePauseRecording) {
                QString error;
                const bool changed = command.togglePauseRecording
                    ? recordingManager.togglePause(&error)
                    : recordingManager.setPaused(command.pauseRecording, &error);
                response.recording = recordingManager.status();
                response.paused = response.recording.paused;
                response.message = changed
                    ? (response.paused ? QStringLiteral("recording paused")
                                       : QStringLiteral("recording resumed"))
                    : (error.isEmpty() ? QStringLiteral("no active recording") : error);
                return response;
            }

            if (command.recordingStatus) {
                response.recording = recordingManager.status();
                response.message = response.recording.active
                    ? QStringLiteral("recording active")
                    : QStringLiteral("recording inactive");
                return response;
            }

            if (command.capture) {
                QTimer::singleShot(0, &app, [requestCapture, command] {
                    requestCapture(command.fullscreen, command.allOutputs, command.captureDelayMs);
                });
                response.message = command.captureDelayMs > 0
                    ? QStringLiteral("capture scheduled in %1 ms").arg(command.captureDelayMs)
                    : QStringLiteral("capture requested");
            } else {
                response.message = QStringLiteral("running");
            }
            response.recording = recordingManager.status();
            return response;
        });

    if (trayMode) {
        // 【剪贴板】【常驻持有】托盘进程常驻，图片由本进程持有以同时提供 PNG 与 BMP
        markshot::setClipboardOwnershipLongLived(true);
        auto *trayController = new markshot::WindowsTrayController(&app, trayConfig, &app);
        trayController->setCaptureCallbacks([launchCapture, allOutputs] { launchCapture(false, allOutputs); },
                                            [launchCapture, allOutputs] { launchCapture(true, allOutputs); });
        markshot::capture_delay::DelayedCaptureMenuCallbacks delayedCallbacks;
        delayedCallbacks.schedule = [requestCapture, allOutputs](int delayMs) {
            requestCapture(false, allOutputs, delayMs);
        };
        delayedCallbacks.cancel = [&delayScheduler] { delayScheduler.cancel(); };
        delayedCallbacks.pending = [&delayScheduler] { return delayScheduler.pending(); };
        trayController->setDelayedCaptureCallbacks(std::move(delayedCallbacks));
        trayController->setRecordingRegionCallback([launchCapture, allOutputs](markshot::recording::RecordingOptions options) {
            launchCapture(false, allOutputs, std::move(options));
        });
        if (!trayController->start()) {
            QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), trayController->errorString());
            return 1;
        }
        return QApplication::exec();
    }

    if (captureDelayMs > 0) {
        // 延时期间还没有截图窗口，禁止因无窗口而提前退出
        app.setQuitOnLastWindowClosed(false);
        delayScheduler.schedule(captureDelayMs, [&app, launchCapture, fullscreenAnnotation, allOutputs] {
            app.setQuitOnLastWindowClosed(true);
            if (!launchCapture(fullscreenAnnotation, allOutputs)) {
                app.exit(1);
            }
        });
        return QApplication::exec();
    }
    if (!launchCapture(fullscreenAnnotation, allOutputs)) {
        return 1;
    }
    return QApplication::exec();
}
