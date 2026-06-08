#include "annotation_launch.h"
#include "config_value.h"
#include "screen_capture.h"
#include "shot_window.h"
#include "debug_log.h"
#include "ui/icons.h"
#include "ui/i18n.h"
#include "ui/theme.h"
#include "window_detection.h"
#include "windows_tray_controller.h"

#include <QApplication>
#include <QByteArray>
#include <QColor>
#include <QCommandLineParser>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFont>
#include <QGuiApplication>
#include <QImageReader>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QMessageBox>
#include <QPointer>
#include <QProcessEnvironment>
#include <QScreen>
#include <QStringList>
#include <QTimer>
#include <QVector>

#include <algorithm>
#include <memory>
#include <optional>

namespace {

/// @brief Calculates the bounding rectangle of all connected screens combined.
/// @return A QRect representing the geometry of the virtual desktop.
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

/// @brief Generates a list of candidate file paths for the application configuration.
/// @return A list of candidate file paths.
QStringList preApplicationConfigPathCandidates()
{
    QStringList candidates;
#if defined(Q_OS_WIN)
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString localAppData = env.value(QStringLiteral("LOCALAPPDATA"));
    if (!localAppData.isEmpty()) {
        candidates.append(QDir(localAppData).filePath(QStringLiteral("mark-shot/config.json")));
    }

    const QString appData = env.value(QStringLiteral("APPDATA"));
    if (!appData.isEmpty()) {
        candidates.append(QDir(appData).filePath(QStringLiteral("mark-shot/config.json")));
    }

    const QString userProfile = env.value(QStringLiteral("USERPROFILE"));
    if (!userProfile.isEmpty()) {
        candidates.append(QDir(userProfile).filePath(QStringLiteral("AppData/Local/mark-shot/config.json")));
    }
#else
    const QByteArray xdgConfigHome = qgetenv("XDG_CONFIG_HOME");
    if (!xdgConfigHome.isEmpty()) {
        candidates.append(QDir(QString::fromLocal8Bit(xdgConfigHome))
                              .filePath(QStringLiteral("mark-shot/config.json")));
    }

    const QByteArray home = qgetenv("HOME");
    if (!home.isEmpty()) {
        candidates.append(QDir(QString::fromLocal8Bit(home)).filePath(QStringLiteral(".config/mark-shot/config.json")));
    }
#endif
    candidates.removeAll(QString());
    candidates.removeDuplicates();
    return candidates;
}

/// @brief Finds the path to the application configuration file by scanning candidates.
/// @return The path to the first existing configuration file, or a default candidate path.
QString preApplicationConfigPath()
{
    const QStringList candidates = preApplicationConfigPathCandidates();
    for (const QString &path : candidates) {
        if (QFileInfo::exists(path)) {
            return path;
        }
    }
    return candidates.isEmpty() ? QString() : candidates.first();
}

/// @brief Applies environment variables configured in the application config file.
void applyConfiguredEnvironment()
{
    const QString configPath = preApplicationConfigPath();
    if (configPath.isEmpty()) {
        return;
    }

    QFile file(configPath);
    if (!file.exists() || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return;
    }

    QJsonObject environment = markshot::config::objectValue(document.object(), QStringLiteral("env"));
    const QJsonObject namedEnvironment = markshot::config::objectValue(document.object(), QStringLiteral("environment"));
    for (auto it = namedEnvironment.constBegin(); it != namedEnvironment.constEnd(); ++it) {
        environment.insert(it.key(), it.value());
    }

    for (auto it = environment.constBegin(); it != environment.constEnd(); ++it) {
        const QString key = it.key().trimmed();
        if (key.isEmpty()) {
            continue;
        }
        if (const std::optional<QString> value = markshot::config::environmentStringValue(it.value())) {
            const QByteArray keyBytes = key.toUtf8();
            const QByteArray valueBytes = value->toUtf8();
            qputenv(keyBytes.constData(), valueBytes);
            markshot::debugLog("config", "env override %s", keyBytes.constData());
        }
    }
}

/// @brief Resolves the first non-empty string value from a list of candidate keys in a JSON object.
/// @param object The JSON object to inspect.
/// @param keys The list of candidate keys to look up.
/// @return The resolved non-empty string, or an empty string if none.
QString stringValue(const QJsonObject &object, const QStringList &keys)
{
    for (const QString &key : keys) {
        const QString value = object.value(key).toString().trimmed();
        if (!value.isEmpty()) {
            return value;
        }
    }
    return {};
}

/// @brief Retrieves the first non-empty trimmed string from a list.
/// @param values The list of string values.
/// @return The first non-empty trimmed string, or an empty string if none.
QString firstStringValue(const QStringList &values)
{
    for (const QString &value : values) {
        const QString trimmed = value.trimmed();
        if (!trimmed.isEmpty()) {
            return trimmed;
        }
    }
    return {};
}

/// @brief Resolves the first defined JSON value from a list of candidate keys.
/// @param object The JSON object to inspect.
/// @param keys The candidate keys to look up.
/// @return The resolved JSON value, or an undefined QJsonValue if none were found.
QJsonValue jsonValue(const QJsonObject &object, const QStringList &keys)
{
    for (const QString &key : keys) {
        const QJsonValue value = object.value(key);
        if (!value.isUndefined()) {
            return value;
        }
    }
    return {};
}

/// @brief Checks whether the given text consists entirely of hexadecimal digits.
/// @param text The text to inspect.
/// @return True if the text only contains hex digits, false otherwise.
bool isHexDigits(const QString &text)
{
    for (const QChar ch : text) {
        if (!ch.isDigit()
            && (ch < QLatin1Char('a') || ch > QLatin1Char('f'))
            && (ch < QLatin1Char('A') || ch > QLatin1Char('F'))) {
            return false;
        }
    }
    return !text.isEmpty();
}

/// @brief Extracts a color channel value from a JSON object.
/// @param object The JSON object to inspect.
/// @param keys The list of candidate keys representing the channel.
/// @return An optional integer channel value clamped between 0 and 255.
std::optional<int> channelValue(const QJsonObject &object, const QStringList &keys)
{
    return markshot::config::clampedIntValue(jsonValue(object, keys), 0, 255);
}

/// @brief Parses a QColor from a string representation.
/// @param value The string representing the color.
/// @return An optional QColor if parsing is successful.
std::optional<QColor> colorFromString(QString value);

/// @brief Parses a QColor from a QJsonObject.
/// @param object The JSON object representing a color.
/// @return An optional QColor if parsing is successful.
std::optional<QColor> colorFromObject(const QJsonObject &object)
{
    const QJsonValue nestedColor =
        jsonValue(object,
                  {QStringLiteral("value"),
                   QStringLiteral("color"),
                   QStringLiteral("hex"),
                   QStringLiteral("format"),
                   QStringLiteral("formats")});
    if (nestedColor.isString()) {
        return colorFromString(nestedColor.toString());
    }

    const std::optional<int> red = channelValue(object, {QStringLiteral("r"), QStringLiteral("red")});
    const std::optional<int> green = channelValue(object, {QStringLiteral("g"), QStringLiteral("green")});
    const std::optional<int> blue = channelValue(object, {QStringLiteral("b"), QStringLiteral("blue")});
    if (!red.has_value() || !green.has_value() || !blue.has_value()) {
        return std::nullopt;
    }

    const std::optional<int> alpha = channelValue(object, {QStringLiteral("a"), QStringLiteral("alpha")});
    return QColor(*red, *green, *blue, alpha.value_or(255));
}

/// @brief Parses a QColor from a string representation.
/// @param value The string representing the color.
/// @return An optional QColor if parsing is successful.
std::optional<QColor> colorFromString(QString value)
{
    QString text = value.trimmed();
    if (text.isEmpty()) {
        return std::nullopt;
    }
    if (text.startsWith(QStringLiteral("0x"), Qt::CaseInsensitive)) {
        text = text.mid(2);
    }
    if (!text.startsWith(QLatin1Char('#')) && (text.size() == 6 || text.size() == 8) && isHexDigits(text)) {
        text.prepend(QLatin1Char('#'));
    }
    if (text.startsWith(QLatin1Char('#')) && text.size() == 9 && isHexDigits(text.mid(1))) {
        bool ok = false;
        const int red = text.mid(1, 2).toInt(&ok, 16);
        if (!ok) {
            return std::nullopt;
        }
        const int green = text.mid(3, 2).toInt(&ok, 16);
        if (!ok) {
            return std::nullopt;
        }
        const int blue = text.mid(5, 2).toInt(&ok, 16);
        if (!ok) {
            return std::nullopt;
        }
        const int alpha = text.mid(7, 2).toInt(&ok, 16);
        if (!ok) {
            return std::nullopt;
        }
        return QColor(red, green, blue, alpha);
    }

    const QColor color(text);
    if (color.isValid()) {
        return color;
    }
    return std::nullopt;
}

/// @brief Parses a QColor from a QJsonValue.
/// @param value The JSON value containing color information (string or object).
/// @return An optional QColor if parsing is successful.
std::optional<QColor> colorFromValue(const QJsonValue &value)
{
    if (value.isString()) {
        return colorFromString(value.toString());
    }
    if (value.isObject()) {
        return colorFromObject(value.toObject());
    }
    return std::nullopt;
}

/// @brief Structure to store default annotation tool and color settings.
struct DefaultTools {
    /// @brief Default tool for normal capture mode.
    ShotWindow::Tool normal = ShotWindow::Tool::Pen;
    /// @brief Default tool for fullscreen capture mode.
    ShotWindow::Tool fullscreen = ShotWindow::Tool::Pen;
    /// @brief Default tool for annotating an existing image file.
    ShotWindow::Tool file = ShotWindow::Tool::Pen;
    /// @brief Default annotation color.
    QColor color = markshot::theme::kDefaultAnnotationColor;
};

struct DebugRuntimeConfig {
    bool enabled = markshot::debugEnabled();
    QString logPath = markshot::debugLogPath();
};

QString expandedConfigPath(QString path)
{
    path = path.trimmed();
    if (path == QStringLiteral("~")) {
        return QDir::homePath();
    }
    if (path.startsWith(QStringLiteral("~/"))) {
        return QDir::home().filePath(path.mid(2));
    }
    return path;
}

DebugRuntimeConfig configuredDebugRuntimeConfig()
{
    DebugRuntimeConfig config;
    QFile file(markshot::appConfigPath());
    if (!file.exists() || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return config;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return config;
    }

    const QJsonObject root = document.object();
    const QJsonValue debugValue = root.value(QStringLiteral("debug"));
    if (const std::optional<bool> enabled = markshot::config::boolValue(debugValue)) {
        config.enabled = *enabled;
    }

    if (debugValue.isObject()) {
        const QJsonObject debug = debugValue.toObject();
        if (const std::optional<bool> enabled = markshot::config::boolValue(
                markshot::config::valueForKeys(debug,
                                               {QStringLiteral("enabled"),
                                                QStringLiteral("enable"),
                                                QStringLiteral("on")}))) {
            config.enabled = *enabled;
        }

        const QString path = stringValue(debug,
                                         {QStringLiteral("logPath"),
                                          QStringLiteral("path"),
                                          QStringLiteral("file"),
                                          QStringLiteral("logFile")});
        if (!path.isEmpty()) {
            config.logPath = expandedConfigPath(path);
        }
    }

    const QString rootLogPath = stringValue(root,
                                           {QStringLiteral("debugLogPath"),
                                            QStringLiteral("debugLog"),
                                            QStringLiteral("logPath")});
    if (!rootLogPath.isEmpty()) {
        config.logPath = expandedConfigPath(rootLogPath);
    }
    if (markshot::debugEnabled()) {
        config.enabled = true;
    }
    return config;
}

/// @brief Loads and returns the configured default annotation tools and colors.
/// @param warning Output parameter to receive warning messages during configuration loading.
/// @return The configured DefaultTools structure.
DefaultTools configuredDefaultTools(QString *warning)
{
    if (warning) {
        warning->clear();
    }

    DefaultTools tools;
    QFile file(markshot::appConfigPath());
    if (!file.exists() || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return tools;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return tools;
    }

    const QJsonObject root = document.object();
    const QJsonObject annotation = markshot::config::objectValue(root, QStringLiteral("annotation"));
    const QJsonObject defaultTools = markshot::config::objectValue(annotation, QStringLiteral("defaultTools"));
    const QJsonObject rootDefaultTools = markshot::config::objectValue(root, QStringLiteral("defaultTools"));
    QStringList warnings;
    bool hasToolWarnings = false;

    auto parseTool = [&warnings, &hasToolWarnings](const QString &name, const QString &source) -> std::optional<ShotWindow::Tool> {
        if (name.isEmpty()) {
            return std::nullopt;
        }
        const std::optional<ShotWindow::Tool> tool = ShotWindow::toolFromName(name);
        if (tool.has_value()) {
            return tool;
        }
        warnings.append(MS_TR("Unsupported default tool in config %1: %2").arg(source, name));
        hasToolWarnings = true;
        return std::nullopt;
    };

    const QString commonDefault = firstStringValue({
        stringValue(annotation, {QStringLiteral("defaultTool"), QStringLiteral("defaultAnnotationTool")}),
        stringValue(root, {QStringLiteral("defaultTool"), QStringLiteral("defaultAnnotationTool")}),
    });
    if (const std::optional<ShotWindow::Tool> tool =
            parseTool(commonDefault, QStringLiteral("annotation.defaultTool"))) {
        tools.normal = *tool;
        tools.fullscreen = *tool;
        tools.file = *tool;
    }

    const QString normalDefault = firstStringValue({
        stringValue(defaultTools,
                    {QStringLiteral("normal"), QStringLiteral("region"), QStringLiteral("selection")}),
        stringValue(rootDefaultTools,
                    {QStringLiteral("normal"), QStringLiteral("region"), QStringLiteral("selection")}),
        stringValue(annotation,
                    {QStringLiteral("normalDefaultTool"),
                     QStringLiteral("regionDefaultTool"),
                     QStringLiteral("selectionDefaultTool")}),
        stringValue(root,
                    {QStringLiteral("normalDefaultTool"),
                     QStringLiteral("regionDefaultTool"),
                     QStringLiteral("selectionDefaultTool")}),
    });
    if (const std::optional<ShotWindow::Tool> tool =
            parseTool(normalDefault, QStringLiteral("annotation.defaultTools.normal"))) {
        tools.normal = *tool;
    }

    const QString fullscreenDefault = firstStringValue({
        stringValue(defaultTools,
                    {QStringLiteral("fullscreen"), QStringLiteral("fullScreen"), QStringLiteral("full-screen")}),
        stringValue(rootDefaultTools,
                    {QStringLiteral("fullscreen"), QStringLiteral("fullScreen"), QStringLiteral("full-screen")}),
        stringValue(annotation,
                    {QStringLiteral("fullscreenDefaultTool"),
                     QStringLiteral("fullScreenDefaultTool"),
                     QStringLiteral("defaultFullscreenTool")}),
        stringValue(root,
                    {QStringLiteral("fullscreenDefaultTool"),
                     QStringLiteral("fullScreenDefaultTool"),
                     QStringLiteral("defaultFullscreenTool")}),
    });
    if (const std::optional<ShotWindow::Tool> tool =
            parseTool(fullscreenDefault, QStringLiteral("annotation.fullscreenDefaultTool"))) {
        tools.fullscreen = *tool;
        tools.file = *tool;
    }

    const QString fileDefault = firstStringValue({
        stringValue(annotation,
                    {QStringLiteral("fileDefaultTool"),
                     QStringLiteral("imageDefaultTool"),
                     QStringLiteral("openFileDefaultTool")}),
        stringValue(root,
                    {QStringLiteral("fileDefaultTool"),
                     QStringLiteral("imageDefaultTool"),
                     QStringLiteral("openFileDefaultTool")}),
        stringValue(defaultTools,
                    {QStringLiteral("file"), QStringLiteral("image"), QStringLiteral("openFile")}),
        stringValue(rootDefaultTools,
                    {QStringLiteral("file"), QStringLiteral("image"), QStringLiteral("openFile")}),
    });
    if (const std::optional<ShotWindow::Tool> tool =
            parseTool(fileDefault, QStringLiteral("annotation.fileDefaultTool"))) {
        tools.file = *tool;
    }

    const QJsonValue defaultColor = jsonValue(annotation,
                                             {QStringLiteral("defaultColor"),
                                              QStringLiteral("defaultAnnotationColor")});
    const QJsonValue rootDefaultColor = jsonValue(root,
                                                 {QStringLiteral("defaultColor"),
                                                  QStringLiteral("defaultAnnotationColor")});
    const QJsonValue configuredColor = !defaultColor.isUndefined() ? defaultColor : rootDefaultColor;
    if (!configuredColor.isUndefined()) {
        if (const std::optional<QColor> color = colorFromValue(configuredColor)) {
            tools.color = *color;
        } else {
            markshot::debugLog("config",
                               "unsupported default color in annotation.defaultColor; using built-in default");
        }
    }

    if (warning && !warnings.isEmpty()) {
        QStringList supportLines;
        if (hasToolWarnings) {
            supportLines.append(MS_TR("Supported tools: %1").arg(ShotWindow::supportedToolNames().join(QStringLiteral(", "))));
        }
        *warning = warnings.join(QStringLiteral("\n"));
        if (!supportLines.isEmpty()) {
            *warning += QStringLiteral("\n") + supportLines.join(QStringLiteral("\n"));
        }
    }
    return tools;
}

/// @brief Disables standard Qt portal services to allow custom host application registration.
void disableQtPortalServicesForHostApp()
{
#if defined(Q_OS_UNIX) && !defined(Q_OS_DARWIN)
    if (qEnvironmentVariableIsSet("QT_NO_XDG_DESKTOP_PORTAL")) {
        return;
    }
    if (QFileInfo::exists(QStringLiteral("/.flatpak-info")) || qEnvironmentVariableIsSet("SNAP")) {
        return;
    }

    // Qt 6.10+ can query the portal before registering the app id, which makes
    // host registration fail on xdg-desktop-portal versions that bind an id to
    // the first portal call. mark-shot performs its own registration before
    // screenshot portal calls.
    qputenv("QT_NO_XDG_DESKTOP_PORTAL", QByteArrayLiteral("1"));
#endif
}

/// @brief Captures a screen and displays the screenshot inside a capture window.
/// @param screen The screen to capture.
/// @param allOutputs Whether to capture all screen outputs as a single virtual screen.
/// @param useRegularWindow Whether to use standard windows instead of layer shell overlays.
/// @param fullscreenAnnotation Whether to automatically start annotation in fullscreen mode.
/// @param defaultTools Configuration specifying the default annotation tools and color.
/// @param error Output parameter to receive any error message that occurs.
/// @return Pointer to the created ShotWindow instance, or nullptr on failure.
ShotWindow *showCaptureWindow(QScreen *screen,
                              bool allOutputs,
                              bool useRegularWindow,
                              bool fullscreenAnnotation,
                              const DefaultTools &defaultTools,
                              QString *error)
{
    const QRect captureGeometry = allOutputs ? virtualScreensGeometry() : (screen ? screen->geometry() : QRect());
    const QString outputName = (!allOutputs && screen) ? screen->name() : QString();
    const bool detectWindows = markshot::windowDetectionEnabled();
    const QVector<QRect> windowGeometries = detectWindows
        ? markshot::collectConfiguredWindowGeometries(captureGeometry, outputName, allOutputs)
        : QVector<QRect>();
    CaptureResult capture = captureScreenFrame({outputName, captureGeometry, allOutputs});
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
            window->showFullScreen();
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

/// @brief Starts a capture session and shows capture windows for the relevant screens.
/// @param app Pointer to the QApplication instance.
/// @param allOutputs Whether to capture all screen outputs as a single virtual screen.
/// @param useRegularWindow Whether to use standard windows instead of layer shell overlays.
/// @param fullscreenAnnotation Whether to automatically start annotation in fullscreen mode.
/// @param defaultTools Configuration specifying the default annotation tools and color.
/// @param error Output parameter to receive any error message that occurs.
/// @return A vector of pointers to the created ShotWindow instances.
QVector<QPointer<ShotWindow>> showCaptureSession(QApplication *app,
                                                 bool allOutputs,
                                                 bool useRegularWindow,
                                                 bool fullscreenAnnotation,
                                                 const DefaultTools &defaultTools,
                                                 QString *error)
{
    QVector<QPointer<ShotWindow>> windows;
    QScreen *screen = markshot::focusedScreen();
    const QList<QScreen *> screens = QGuiApplication::screens();
    if (!allOutputs && !fullscreenAnnotation && screens.size() > 1) {
        for (QScreen *candidate : screens) {
            if (!candidate || candidate->geometry().isEmpty()) {
                continue;
            }
            ShotWindow *window =
                showCaptureWindow(candidate, false, useRegularWindow, fullscreenAnnotation, defaultTools, error);
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
        showCaptureWindow(allOutputs ? nullptr : screen, allOutputs, useRegularWindow, fullscreenAnnotation, defaultTools, error);
    if (window) {
        windows.append(window);
    }
    return windows;
}

} // namespace

/// @brief Main entry point of the application.
/// @param argc The count of command line arguments.
/// @param argv The array of command line arguments.
/// @return Exit code of the application.
int main(int argc, char *argv[])
{
    applyConfiguredEnvironment();

    QGuiApplication::setDesktopFileName(QStringLiteral("mark-shot"));
    disableQtPortalServicesForHostApp();

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
    QCommandLineOption allOutputsOption(QStringLiteral("all-outputs"), QStringLiteral("Capture all outputs instead of the current Qt screen."));
    QCommandLineOption xdgWindowOption(QStringLiteral("xdg-window"), QStringLiteral("Use a regular fullscreen xdg window instead of layer-shell."));
    QCommandLineOption fullscreenAnnotationOption({QStringLiteral("fullscreen"), QStringLiteral("full-screen")},
                                                  QStringLiteral("Skip region selection and annotate the full captured frame."));
    QCommandLineOption trayOption(QStringLiteral("tray"),
                                  QStringLiteral("Keep running in the Windows system tray and register global hotkeys."));
    QCommandLineOption captureOption(QStringLiteral("capture"),
                                     QStringLiteral("Capture once even when Windows tray autostart is enabled."));
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

    DebugRuntimeConfig debugConfig = configuredDebugRuntimeConfig();
    if (parser.isSet(debugOption)) {
        debugConfig.enabled = true;
    }
    if (parser.isSet(noDebugOption)) {
        debugConfig.enabled = false;
    }
    if (parser.isSet(debugLogOption)) {
        const QString optionPath = parser.value(debugLogOption).trimmed();
        if (!optionPath.isEmpty()) {
            debugConfig.logPath = expandedConfigPath(optionPath);
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
    DefaultTools defaultTools = configuredDefaultTools(&configDefaultToolWarning);
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
        const std::optional<QColor> parsedColor = colorFromString(optionValue);
        if (!parsedColor.has_value()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Unsupported default color: %1\nSupported color formats: #RRGGBB or #RRGGBBAA")
                                      .arg(optionValue));
            return 1;
        }
        defaultTools.color = *parsedColor;
    }
    if (!configDefaultToolWarning.isEmpty()) {
        QMessageBox::warning(nullptr, QStringLiteral("Mark Shot"), configDefaultToolWarning);
    }

    const QString imagePath = positionalArguments.isEmpty() ? QString() : positionalArguments.first();
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
        window->setDefaultColor(defaultTools.color);
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
    const bool useRegularWindow = parser.isSet(xdgWindowOption);
    const bool fullscreenAnnotation = parser.isSet(fullscreenAnnotationOption);
    const markshot::WindowsTrayController::Config trayConfig = markshot::WindowsTrayController::readConfig();
    const bool trayMode = !parser.isSet(captureOption) && (parser.isSet(trayOption) || trayConfig.autoStart);
    if (trayMode) {
        if (!markshot::WindowsTrayController::isSupported()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Windows tray mode is not supported on this platform."));
            return 1;
        }

        auto *trayController = new markshot::WindowsTrayController(&app, trayConfig, &app);
        bool captureActive = false;
        auto launchCapture = [&app, &captureActive, allOutputs, useRegularWindow, defaultTools](bool startFullscreen) {
            if (captureActive) {
                return;
            }

            QString captureError;
            QVector<QPointer<ShotWindow>> windows =
                showCaptureSession(&app, allOutputs, useRegularWindow, startFullscreen, defaultTools, &captureError);
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

        trayController->setCaptureCallbacks([launchCapture] { launchCapture(false); },
                                            [launchCapture] { launchCapture(true); });
        if (!trayController->start()) {
            QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), trayController->errorString());
            return 1;
        }
        return QApplication::exec();
    }

    QString captureError;
    QVector<QPointer<ShotWindow>> windows =
        showCaptureSession(&app, allOutputs, useRegularWindow, fullscreenAnnotation, defaultTools, &captureError);
    if (windows.isEmpty()) {
        QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), captureError);
        return 1;
    }
    return QApplication::exec();
}
