#include "annotation_launch.h"
#include "capture_cursor_policy.h"
#include "capture_freeze_scope.h"
#include "capture_session_launcher.h"
#include "cli/image_pin_launch.h"
#include "debug_log.h"
#include "shot_window.h"
#include "startup_config.h"
#include "ui/icons.h"
#include "ui/i18n.h"
#include "ui/theme.h"
#include "window_detection.h"
#include "windows_tray_controller.h"

#include <QApplication>
#include <QByteArray>
#include <QCommandLineParser>
#include <QFileInfo>
#include <QFont>
#include <QGuiApplication>
#include <QImageReader>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QIODevice>
#include <QLocalServer>
#include <QLocalSocket>
#include <QMessageBox>
#include <QPointer>
#include <QScreen>
#include <QStringList>
#include <QTimer>
#include <QVector>

#include <algorithm>
#include <functional>
#include <memory>
#include <optional>
#include <utility>

namespace {

struct SingleInstanceCommand {
    bool capture = false;
    bool fullscreen = false;
    bool allOutputs = false;
};

QString singleInstanceServerName()
{
    return QStringLiteral("mark-shot-single-instance");
}

QByteArray encodeSingleInstanceCommand(const SingleInstanceCommand &command)
{
    QJsonObject object;
    object.insert(QStringLiteral("capture"), command.capture);
    object.insert(QStringLiteral("fullscreen"), command.fullscreen);
    object.insert(QStringLiteral("allOutputs"), command.allOutputs);
    return QJsonDocument(object).toJson(QJsonDocument::Compact) + '\n';
}

std::optional<SingleInstanceCommand> decodeSingleInstanceCommand(const QByteArray &payload)
{
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(payload.trimmed(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        markshot::debugLog("single-instance",
                           "invalid ipc command: %s",
                           parseError.errorString().toUtf8().constData());
        return std::nullopt;
    }

    const QJsonObject object = document.object();
    SingleInstanceCommand command;
    command.capture = object.value(QStringLiteral("capture")).toBool(false);
    command.fullscreen = object.value(QStringLiteral("fullscreen")).toBool(false);
    command.allOutputs = object.value(QStringLiteral("allOutputs")).toBool(false);
    return command;
}

bool forwardSingleInstanceCommand(const SingleInstanceCommand &command, QString *error)
{
    if (error) {
        error->clear();
    }

    QLocalSocket socket;
    socket.connectToServer(singleInstanceServerName(), QIODevice::WriteOnly);
    if (!socket.waitForConnected(250)) {
        if (error) {
            *error = socket.errorString();
        }
        markshot::debugLog("single-instance",
                           "connect failed: %s",
                           socket.errorString().toUtf8().constData());
        return false;
    }

    const QByteArray payload = encodeSingleInstanceCommand(command);
    if (socket.write(payload) != payload.size() || !socket.waitForBytesWritten(500)) {
        if (error) {
            *error = socket.errorString();
        }
        markshot::debugLog("single-instance",
                           "write failed: %s",
                           socket.errorString().toUtf8().constData());
        return false;
    }

    socket.disconnectFromServer();
    socket.waitForDisconnected(100);
    markshot::debugLog("single-instance",
                       "forwarded capture=%d fullscreen=%d allOutputs=%d",
                       command.capture ? 1 : 0,
                       command.fullscreen ? 1 : 0,
                       command.allOutputs ? 1 : 0);
    return true;
}

std::unique_ptr<QLocalServer> listenForSingleInstanceCommands(QString *error)
{
    if (error) {
        error->clear();
    }

    auto server = std::make_unique<QLocalServer>();
    server->setSocketOptions(QLocalServer::UserAccessOption);
    if (server->listen(singleInstanceServerName())) {
        markshot::debugLog("single-instance",
                           "listening on %s",
                           singleInstanceServerName().toUtf8().constData());
        return server;
    }

    if (error) {
        *error = server->errorString();
    }
    markshot::debugLog("single-instance",
                       "listen failed: %s",
                       server->errorString().toUtf8().constData());
    return nullptr;
}

void installSingleInstanceCommandHandler(
    QLocalServer *server,
    QObject *context,
    std::function<void(const SingleInstanceCommand &)> handler)
{
    if (!server || !context || !handler) {
        return;
    }

    QObject::connect(server, &QLocalServer::newConnection, context, [server, handler = std::move(handler)] {
        while (QLocalSocket *socket = server->nextPendingConnection()) {
            QObject::connect(socket, &QLocalSocket::disconnected, socket, [socket, handler] {
                const QByteArray payload = socket->readAll();
                const std::optional<SingleInstanceCommand> command = decodeSingleInstanceCommand(payload);
                if (command.has_value()) {
                    markshot::debugLog("single-instance",
                                       "received capture=%d fullscreen=%d allOutputs=%d",
                                       command->capture ? 1 : 0,
                                       command->fullscreen ? 1 : 0,
                                       command->allOutputs ? 1 : 0);
                    handler(*command);
                }
                socket->deleteLater();
            });
        }
    });
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
                                  QStringLiteral("Keep running in the Windows system tray and register global hotkeys."));
    QCommandLineOption captureOption(QStringLiteral("capture"),
                                     QStringLiteral("Capture once even when Windows tray autostart is enabled."));
    QCommandLineOption pinImageOption(QStringLiteral("pin-image"),
                                      QStringLiteral("Open an image file directly as a pinned sticker."),
                                      QStringLiteral("path"));
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
    parser.addOption(defaultToolOption);
    parser.addOption(fullscreenDefaultToolOption);
    parser.addOption(fileDefaultToolOption);
    parser.addOption(defaultColorOption);
    parser.addOption(debugOption);
    parser.addOption(noDebugOption);
    parser.addOption(debugLogOption);
    parser.process(app);

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
    SingleInstanceCommand duplicateCommand;
    duplicateCommand.capture = !explicitTrayOnly;
    duplicateCommand.fullscreen = fullscreenAnnotation;
    duplicateCommand.allOutputs = allOutputs;
    if (forwardSingleInstanceCommand(duplicateCommand, nullptr)) {
        return 0;
    }

    if (trayMode) {
        if (!markshot::WindowsTrayController::isSupported()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("System tray is not available on this platform."));
            return 1;
        }

        QString singleInstanceError;
        std::unique_ptr<QLocalServer> singleInstanceServer =
            listenForSingleInstanceCommands(&singleInstanceError);
        if (!singleInstanceServer) {
            if (forwardSingleInstanceCommand(duplicateCommand, nullptr)) {
                return 0;
            }
            QLocalServer::removeServer(singleInstanceServerName());
            singleInstanceServer = listenForSingleInstanceCommands(&singleInstanceError);
        }
        if (!singleInstanceServer) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Failed to start single-instance guard: %1").arg(singleInstanceError));
            return 1;
        }

        auto *trayController = new markshot::WindowsTrayController(&app, trayConfig, &app);
        bool captureActive = false;
        auto launchCapture = [&app, &captureActive, freezeScope, includeCursor, useRegularWindow, defaultTools](bool startFullscreen, bool requestAllOutputs) {
            if (captureActive) {
                return;
            }

            QString captureError;
            QVector<QPointer<ShotWindow>> windows =
                markshot::showCaptureSession(&app,
                                             requestAllOutputs,
                                             freezeScope,
                                             includeCursor,
                                             useRegularWindow,
                                             startFullscreen,
                                             defaultTools,
                                             &captureError);
            if (windows.isEmpty()) {
                if (!captureError.isEmpty()) {
                    QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), captureError);
                }
                return;
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
        };

        trayController->setCaptureCallbacks([launchCapture, allOutputs] { launchCapture(false, allOutputs); },
                                            [launchCapture, allOutputs] { launchCapture(true, allOutputs); });
        installSingleInstanceCommandHandler(singleInstanceServer.get(), &app, [&app, launchCapture](const SingleInstanceCommand &command) {
            if (!command.capture) {
                return;
            }
            QTimer::singleShot(0, &app, [launchCapture, command] {
                launchCapture(command.fullscreen, command.allOutputs);
            });
        });
        if (!trayController->start()) {
            QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), trayController->errorString());
            return 1;
        }
        return QApplication::exec();
    }

    QString captureError;
    QVector<QPointer<ShotWindow>> windows =
        markshot::showCaptureSession(&app,
                                     allOutputs,
                                     freezeScope,
                                     includeCursor,
                                     useRegularWindow,
                                     fullscreenAnnotation,
                                     defaultTools,
                                     &captureError);
    if (windows.isEmpty()) {
        QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), captureError);
        return 1;
    }
    return QApplication::exec();
}
