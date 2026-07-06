#include "annotation_launch.h"
#include "capture_cursor_policy.h"
#include "capture_freeze_scope.h"
#include "capture_own_windows_policy.h"
#include "capture_session_launcher.h"
#include "cli/image_pin_launch.h"
#include "cli/recording_cli.h"
#include "debug_log.h"
#include "ipc/single_instance_ipc.h"
#include "recording/recording_session_manager.h"
#include "shot_window.h"
#include "startup_config.h"
#include "ui/icons.h"
#include "ui/i18n.h"
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

/// @brief Main entry point of the application.
/// @param argc The count of command line arguments.
/// @param argv The array of command line arguments.
/// @return Exit code of the application.
int main(int argc, char *argv[])
{
    markshot::applyConfiguredEnvironment();

    QGuiApplication::setDesktopFileName(QStringLiteral("mark-shot"));
    markshot::disableQtPortalServicesForHostApp();

    QApplication app(argc, argv);
    QApplication::setApplicationName(QStringLiteral("mark-shot"));
    QApplication::setApplicationDisplayName(QStringLiteral("Mark Shot"));
    QApplication::setApplicationVersion(QStringLiteral(MARK_SHOT_VERSION));
    QApplication::setWindowIcon(markshot::ui::applicationIcon());
    QFont applicationFont = app.font();
    applicationFont.setFamily(markshot::theme::uiFontFamily());
    app.setFont(applicationFont);

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
    QCommandLineOption recordingStatusOption(QStringLiteral("recording-status"),
                                             QStringLiteral("Print the current recording status as JSON."));
    QCommandLineOption stopRecordingOption(QStringLiteral("stop-recording"),
                                           QStringLiteral("Stop the active recording through the running instance."));
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
    parser.addOption(recordingStatusOption);
    parser.addOption(stopRecordingOption);
    parser.addOption(defaultToolOption);
    parser.addOption(fullscreenDefaultToolOption);
    parser.addOption(fileDefaultToolOption);
    parser.addOption(defaultColorOption);
    parser.addOption(debugOption);
    parser.addOption(noDebugOption);
    parser.addOption(debugLogOption);
    parser.process(app);

    if (parser.isSet(stopRecordingOption)) {
        return markshot::cli::stopRecordingFromCommandLine();
    }
    if (parser.isSet(recordingStatusOption)) {
        return markshot::cli::printRecordingStatus();
    }

    const QStringList positionalArguments = parser.positionalArguments();
    if (positionalArguments.size() > 1) {
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
        const QImage image = reader.read();
        if (image.isNull()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Failed to load image: %1\n%2").arg(imageFile.absoluteFilePath(), reader.errorString()));
            return 1;
        }

        ShotWindow *window = new ShotWindow(image, imageFile.fileName());
        window->setDefaultTools(defaultTools.normal, defaultTools.file);
        if (markshot::shouldApplyDefaultColor(defaultTools)) {
            window->setDefaultColor(defaultTools.color);
        }
        QScreen *screen = markshot::focusedScreen();
        if (screen) {
            window->setScreen(screen);
        }
        window->setWindowFlags(Qt::Window);
        const QRect windowGeometry = markshot::centeredImageWindowGeometry(image.size(), screen);
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
    const bool includeCursor = markshot::configuredCaptureIncludeCursor();
    const bool hideOwnWindows = markshot::configuredHideOwnWindowsDuringCapture();
    const bool useRegularWindow = parser.isSet(xdgWindowOption);
    const bool fullscreenAnnotation = parser.isSet(fullscreenAnnotationOption);
    const markshot::WindowsTrayController::Config trayConfig = markshot::WindowsTrayController::readConfig();
    const bool explicitCaptureRequest =
        parser.isSet(captureOption) || parser.isSet(allOutputsOption) || parser.isSet(fullscreenAnnotationOption);
    const bool trayMode = !explicitCaptureRequest && (parser.isSet(trayOption) || trayConfig.autoStart);

    const bool explicitTrayOnly = parser.isSet(trayOption)
        && !parser.isSet(captureOption)
        && !parser.isSet(allOutputsOption)
        && !parser.isSet(fullscreenAnnotationOption);
    markshot::ipc::SingleInstanceCommand duplicateCommand;
    duplicateCommand.capture = !explicitTrayOnly;
    duplicateCommand.fullscreen = fullscreenAnnotation;
    duplicateCommand.allOutputs = allOutputs;
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
                          freezeScope,
                          includeCursor,
                          hideOwnWindows,
                          useRegularWindow,
                          defaultTools](bool startFullscreen,
                                        bool requestAllOutputs,
                                        std::optional<markshot::recording::RecordingOptions> regionRecordingOptions = std::nullopt) -> bool {
        if (captureActive) {
            return true;
        }

        QString captureError;
        QVector<QPointer<ShotWindow>> windows =
            markshot::showCaptureSession(&app,
                                         requestAllOutputs,
                                         freezeScope,
                                         includeCursor,
                                         useRegularWindow,
                                         startFullscreen,
                                         hideOwnWindows,
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

    auto &recordingManager = markshot::recording::RecordingSessionManager::instance();
    markshot::ipc::installSingleInstanceCommandHandler(
        singleInstanceServer.get(),
        &app,
        [&app, launchCapture, &recordingManager](const markshot::ipc::SingleInstanceCommand &command) {
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

            if (command.recordingStatus) {
                response.recording = recordingManager.status();
                response.message = response.recording.active
                    ? QStringLiteral("recording active")
                    : QStringLiteral("recording inactive");
                return response;
            }

            if (command.capture) {
                QTimer::singleShot(0, &app, [launchCapture, command] {
                    launchCapture(command.fullscreen, command.allOutputs);
                });
                response.message = QStringLiteral("capture requested");
            } else {
                response.message = QStringLiteral("running");
            }
            response.recording = recordingManager.status();
            return response;
        });

    if (trayMode) {
        auto *trayController = new markshot::WindowsTrayController(&app, trayConfig, &app);
        trayController->setCaptureCallbacks([launchCapture, allOutputs] { launchCapture(false, allOutputs); },
                                            [launchCapture, allOutputs] { launchCapture(true, allOutputs); });
        trayController->setRecordingRegionCallback([launchCapture, allOutputs](markshot::recording::RecordingOptions options) {
            launchCapture(false, allOutputs, std::move(options));
        });
        if (!trayController->start()) {
            QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), trayController->errorString());
            return 1;
        }
        return QApplication::exec();
    }

    if (!launchCapture(fullscreenAnnotation, allOutputs)) {
        return 1;
    }
    return QApplication::exec();
}
