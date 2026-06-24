#include "window_detection.h"

#include "config_value.h"
#include "debug_log.h"
#include "shell_command.h"
#include "ui/theme.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QMap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QRegularExpression>
#include <QStandardPaths>
#include <QStringList>

#include <algorithm>
#include <optional>

namespace markshot {
namespace {

#if defined(Q_OS_WIN)
QString envConfigDir(const QString &name, const QString &relativePath = QStringLiteral("mark-shot"))
{
    const QString root = QProcessEnvironment::systemEnvironment().value(name).trimmed();
    return root.isEmpty() ? QString() : QDir(root).filePath(relativePath);
}
#endif

/// @brief Generates a list of candidate directories where the application configuration might reside.
/// @return A list of directory paths as a QStringList.
QStringList appConfigDirCandidates()
{
    QStringList candidates;
#if defined(Q_OS_WIN)
    const QString localAppData = envConfigDir(QStringLiteral("LOCALAPPDATA"));
    if (!localAppData.isEmpty()) {
        candidates.append(localAppData);
    }

    const QString appData = envConfigDir(QStringLiteral("APPDATA"));
    if (!appData.isEmpty()) {
        candidates.append(appData);
    }

    const QString userProfile = envConfigDir(QStringLiteral("USERPROFILE"),
                                             QStringLiteral("AppData/Local/mark-shot"));
    if (!userProfile.isEmpty()) {
        candidates.append(userProfile);
    }
#endif

    const QString appConfig = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    if (!appConfig.isEmpty()) {
        candidates.append(appConfig);
    }

    const QString genericConfig = QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation);
    if (!genericConfig.isEmpty()) {
        candidates.append(QDir(genericConfig).filePath(QStringLiteral("mark-shot")));
    }

    candidates.append(QDir::home().filePath(QStringLiteral(".config/mark-shot")));
    candidates.removeAll(QString());
    candidates.removeDuplicates();
    return candidates;
}

/// @brief Determines the default application configuration directory.
/// @return The directory path as a QString.
QString defaultAppConfigDir()
{
    const QStringList candidates = appConfigDirCandidates();
    return candidates.isEmpty()
        ? QDir::home().filePath(QStringLiteral(".config/mark-shot"))
        : candidates.first();
}

/// @brief Searches candidate directories and returns the path to the first existing config file.
/// @return The path to the existing config file as a QString, or an empty string if not found.
QString existingAppConfigPath()
{
    const QStringList candidates = appConfigDirCandidates();
    for (const QString &dir : candidates) {
        const QString path = QDir(dir).filePath(QStringLiteral("config.json"));
        if (QFileInfo::exists(path)) {
            return path;
        }
    }
    return {};
}

/// @brief Detects the Wayland session type based on environment variables.
/// @return "gnome", "kde", "hyprland", "niri" for known Wayland sessions, empty string for X11 or unknown.
QString detectWaylandSessionType()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString sessionType = env.value(QStringLiteral("XDG_SESSION_TYPE")).toLower();
    if (sessionType != QStringLiteral("wayland")) {
        return {};
    }

    const QString desktop = (env.value(QStringLiteral("XDG_CURRENT_DESKTOP"))
        + QLatin1Char(':') + env.value(QStringLiteral("XDG_SESSION_DESKTOP"))
        + QLatin1Char(':') + env.value(QStringLiteral("DESKTOP_SESSION")))
        .toLower();

    if (desktop.contains(QStringLiteral("gnome"))) {
        return QStringLiteral("gnome");
    }
    if (desktop.contains(QStringLiteral("kde")) || desktop.contains(QStringLiteral("plasma"))) {
        return QStringLiteral("kde");
    }
    if (desktop.contains(QStringLiteral("hyprland"))) {
        return QStringLiteral("hyprland");
    }
    if (desktop.contains(QStringLiteral("niri"))) {
        return QStringLiteral("niri");
    }

    return QStringLiteral("niri");
}

/// @brief Returns the appropriate window detection command for the current desktop environment.
QString defaultWindowDetectionCommand()
{
#if defined(Q_OS_WIN)
    return QString();
#else
    const QString sessionType = detectWaylandSessionType();
    if (sessionType.isEmpty()) {
        return QString();
    }
    return QStringLiteral("mark-shot-window-detection-") + sessionType;
#endif
}

/// @brief Generates the default configuration JSON object structure.
/// @return The default QJsonObject configuration root.
QJsonObject defaultAppConfigRoot()
{
    QJsonObject root;
    root.insert(QStringLiteral("env"), QJsonObject());

    QJsonObject debug;
    debug.insert(QStringLiteral("enabled"), false);
    debug.insert(QStringLiteral("logPath"), QString());
    root.insert(QStringLiteral("debug"), debug);

    QJsonObject annotation;
    annotation.insert(QStringLiteral("defaultTool"), QStringLiteral("pen"));
    annotation.insert(QStringLiteral("fullscreenDefaultTool"), QStringLiteral("pen"));
    annotation.insert(QStringLiteral("defaultColor"), QStringLiteral("#FF4D4D"));
    root.insert(QStringLiteral("annotation"), annotation);

    QJsonObject toolbar;
    toolbar.insert(QStringLiteral("iconSize"), QStringLiteral("middle"));
    toolbar.insert(QStringLiteral("fontSize"), QStringLiteral("middle"));
    root.insert(QStringLiteral("toolbar"), toolbar);

    QJsonObject save;
    save.insert(QStringLiteral("pathTemplate"), QStringLiteral("{pictures}/mark-shot/mark-shot-{datetime}.png"));
    root.insert(QStringLiteral("save"), save);

    QJsonObject kwinScreenshot;
    kwinScreenshot.insert(QStringLiteral("enabled"), true);

    QJsonObject kde;
    kde.insert(QStringLiteral("kwinScreenshot"), kwinScreenshot);

    QJsonObject wayland;
    wayland.insert(QStringLiteral("kde"), kde);

    QJsonObject capture;
    capture.insert(QStringLiteral("wayland"), wayland);
    root.insert(QStringLiteral("capture"), capture);

    QJsonObject shortcutTools;
    shortcutTools.insert(QStringLiteral("pen"), QStringLiteral("P"));
    shortcutTools.insert(QStringLiteral("rectangle"), QStringLiteral("R"));

    QJsonObject shortcutActions;
    shortcutActions.insert(QStringLiteral("copy"), QStringLiteral("Ctrl+C"));
    shortcutActions.insert(QStringLiteral("save"), QStringLiteral("Ctrl+S"));
    shortcutActions.insert(QStringLiteral("pin"), QStringLiteral("Ctrl+P"));

    QJsonObject startupShortcuts;
    startupShortcuts.insert(QStringLiteral("colorPicker"), QStringLiteral("C"));
    startupShortcuts.insert(QStringLiteral("ruler"), QStringLiteral("R"));
    startupShortcuts.insert(QStringLiteral("codeScanner"), QStringLiteral("Q"));
    startupShortcuts.insert(QStringLiteral("displayCapture"), QStringLiteral("D"));

    QJsonObject shortcuts;
    shortcuts.insert(QStringLiteral("tools"), shortcutTools);
    shortcuts.insert(QStringLiteral("actions"), shortcutActions);
    shortcuts.insert(QStringLiteral("startup"), startupShortcuts);
    root.insert(QStringLiteral("shortcuts"), shortcuts);

    QJsonObject tray;
#if defined(Q_OS_WIN)
    tray.insert(QStringLiteral("enabled"), true);
#else
    tray.insert(QStringLiteral("enabled"), false);
#endif

    QJsonObject hotkeys;
    hotkeys.insert(QStringLiteral("capture"), QStringLiteral("Ctrl+Alt+S"));

    QJsonObject windows;
    windows.insert(QStringLiteral("tray"), tray);
    windows.insert(QStringLiteral("hotkeys"), hotkeys);
    root.insert(QStringLiteral("windows"), windows);

    QJsonObject codeScan;
    codeScan.insert(QStringLiteral("command"), QString());
    codeScan.insert(QStringLiteral("timeoutMs"), 15000);
    root.insert(QStringLiteral("codeScan"), codeScan);

    QJsonObject pinnedWindow;
    pinnedWindow.insert(QStringLiteral("autoOcr"), false);
    pinnedWindow.insert(QStringLiteral("alwaysOnTop"), true);
    pinnedWindow.insert(QStringLiteral("textSelectionCopyEnabled"), true);
    pinnedWindow.insert(QStringLiteral("border"), true);
    pinnedWindow.insert(QStringLiteral("borderColor"), markshot::theme::kAccent.name(QColor::HexRgb).toUpper());
    pinnedWindow.insert(QStringLiteral("borderWidth"), 2);
    root.insert(QStringLiteral("pinnedWindow"), pinnedWindow);

    QJsonObject scrollCapture;
    scrollCapture.insert(QStringLiteral("frame"), 5);
    scrollCapture.insert(QStringLiteral("previewGap"), 5);
    scrollCapture.insert(QStringLiteral("hidePreviewDuringCapture"), false);
    root.insert(QStringLiteral("scrollCapture"), scrollCapture);

    QJsonObject clipboardImage;
    clipboardImage.insert(QStringLiteral("mode"), QStringLiteral("image/png"));
    clipboardImage.insert(QStringLiteral("thresholdM"), 4);

    QJsonObject clipboard;
    clipboard.insert(QStringLiteral("image"), clipboardImage);
    root.insert(QStringLiteral("clipboard"), clipboard);

    QJsonObject windowDetection;
    windowDetection.insert(QStringLiteral("enabled"), true);
    windowDetection.insert(QStringLiteral("command"), defaultWindowDetectionCommand());
    windowDetection.insert(QStringLiteral("env"), QJsonObject());
    windowDetection.insert(QStringLiteral("timeoutMs"), 1000);
    root.insert(QStringLiteral("windowDetection"), windowDetection);

    QJsonObject ocr;
    ocr.insert(QStringLiteral("enabled"), true);
    ocr.insert(QStringLiteral("backend"), QStringLiteral("rapidocr"));
    ocr.insert(QStringLiteral("command"), QString());
    ocr.insert(QStringLiteral("timeoutMs"), 30000);
    root.insert(QStringLiteral("ocr"), ocr);

    QJsonObject translation;
    translation.insert(QStringLiteral("autoAfterOcr"), false);
    translation.insert(QStringLiteral("targetLanguage"), QStringLiteral("Simplified Chinese"));
    translation.insert(QStringLiteral("apiBase"), QStringLiteral("https://api.openai.com/v1"));
    translation.insert(QStringLiteral("apiKeyEnv"), QStringLiteral("OPENAI_API_KEY"));
    translation.insert(QStringLiteral("apiKey"), QString());
    translation.insert(QStringLiteral("model"), QStringLiteral("gpt-4o-mini"));
    translation.insert(QStringLiteral("temperature"), 0.2);
    translation.insert(QStringLiteral("timeoutMs"), 60000);
    translation.insert(QStringLiteral("timeoutSeconds"), 60);
    translation.insert(QStringLiteral("systemPrompt"), QString());
    translation.insert(QStringLiteral("command"), QString());
    root.insert(QStringLiteral("translation"), translation);

    return root;
}

}  // namespace

QString markShotConfigDir()
{
    const QString existingPath = existingAppConfigPath();
    if (!existingPath.isEmpty()) {
        return QFileInfo(existingPath).absolutePath();
    }
    return defaultAppConfigDir();
}

QString appConfigPath()
{
    const QString existingPath = existingAppConfigPath();
    if (!existingPath.isEmpty()) {
        return existingPath;
    }
    return QDir(defaultAppConfigDir()).filePath(QStringLiteral("config.json"));
}

bool ensureAppConfigFile()
{
    const QString path = appConfigPath();
    if (QFileInfo::exists(path)) {
        return true;
    }

    QDir dir(QFileInfo(path).absolutePath());
    if (!dir.exists() && !dir.mkpath(QStringLiteral("."))) {
        markshot::debugLog("config", "cannot create config dir path=%s", dir.absolutePath().toUtf8().constData());
        return false;
    }

    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text | QIODevice::NewOnly)) {
        markshot::debugLog("config", "cannot create config path=%s", path.toUtf8().constData());
        return false;
    }

    const QJsonDocument document(defaultAppConfigRoot());
    file.write(document.toJson(QJsonDocument::Indented));
    file.write("\n");
    markshot::debugLog("config", "created default config path=%s", path.toUtf8().constData());
    return true;
}

namespace {

/// @brief Default timeout value in milliseconds for the window detection process.
constexpr int kDefaultWindowDetectionTimeoutMs = 1000;
/// @brief Minimum allowed timeout value in milliseconds for window detection.
constexpr int kMinWindowDetectionTimeoutMs = 100;
/// @brief Maximum allowed timeout value in milliseconds for window detection.
constexpr int kMaxWindowDetectionTimeoutMs = 10000;

/// @brief Configuration settings for the external window detection process.
struct WindowDetectionConfig {
    /// @brief The shell command or script path used to perform window detection.
    QString command;
    /// @brief The working directory to execute the detection script in.
    QString workingDirectory;
    /// @brief Custom environment variables to override for the detection process.
    QMap<QString, QString> environment;
    /// @brief Maximum time in milliseconds to wait for the detection process to finish.
    int timeoutMs = kDefaultWindowDetectionTimeoutMs;
};

/// @brief Expands tilde (~) prefixes in file paths to the user's home directory path.
/// @param path The input path string.
/// @return The expanded absolute path.
QString expandUserPath(const QString &path)
{
    if (path == QStringLiteral("~")) {
        return QDir::homePath();
    }
    if (path.startsWith(QStringLiteral("~/"))) {
        return QDir::home().filePath(path.mid(2));
    }
    return path;
}

/// @brief Attempts to extract an integer value from a JSON object using a list of alternative keys.
/// @param object The JSON object.
/// @param keys List of key names to check in order.
/// @return The integer value if found under any key, std::nullopt otherwise.
std::optional<int> namedIntValue(const QJsonObject &object, const QStringList &keys)
{
    for (const QString &key : keys) {
        const std::optional<int> value = config::intValue(object.value(key));
        if (value.has_value()) {
            return value;
        }
    }
    return std::nullopt;
}

/// @brief Parses the environment variables and overrides defined in the window detection config.
/// @param windowDetection The window detection JSON configuration object.
/// @return A map of environment variable keys to their string values.
QMap<QString, QString> environmentOverrides(const QJsonObject &windowDetection)
{
    QJsonObject environment = config::objectValue(windowDetection, QStringLiteral("env"));
    const QJsonObject namedEnvironment = config::objectValue(windowDetection, QStringLiteral("environment"));
    for (auto it = namedEnvironment.constBegin(); it != namedEnvironment.constEnd(); ++it) {
        environment.insert(it.key(), it.value());
    }

    QMap<QString, QString> overrides;
    for (auto it = environment.constBegin(); it != environment.constEnd(); ++it) {
        const QString key = it.key().trimmed();
        if (key.isEmpty()) {
            continue;
        }
        if (const std::optional<QString> value = config::environmentStringValue(it.value())) {
            overrides.insert(key, *value);
        }
    }
    return overrides;
}

/// @brief Converts a JSON array of 4 integers [x, y, w, h] into a bounding rectangle.
/// @param array The JSON array containing at least 4 integer coordinates.
/// @return A QRect if parsed successfully, std::nullopt otherwise.
std::optional<QRect> rectFromArray(const QJsonArray &array)
{
    if (array.size() < 4) {
        return std::nullopt;
    }

    const std::optional<int> x = config::intValue(array.at(0));
    const std::optional<int> y = config::intValue(array.at(1));
    const std::optional<int> width = config::intValue(array.at(2));
    const std::optional<int> height = config::intValue(array.at(3));
    if (!x.has_value() || !y.has_value() || !width.has_value() || !height.has_value()) {
        return std::nullopt;
    }
    return QRect(*x, *y, *width, *height);
}

/// @brief Parses a geometry string (formatted like "x,y WxH") into a bounding rectangle.
/// @param geometry The geometry text string.
/// @return A QRect if parsed successfully, std::nullopt otherwise.
std::optional<QRect> rectFromGeometryText(const QString &geometry)
{
    static const QRegularExpression pattern(
        QStringLiteral("^\\s*(-?\\d+)\\s*,\\s*(-?\\d+)\\s+(-?\\d+)\\s*x\\s*(-?\\d+)\\s*$"));
    const QRegularExpressionMatch match = pattern.match(geometry);
    if (!match.hasMatch()) {
        return std::nullopt;
    }

    bool ok = true;
    const int x = match.captured(1).toInt(&ok);
    if (!ok) {
        return std::nullopt;
    }
    const int y = match.captured(2).toInt(&ok);
    if (!ok) {
        return std::nullopt;
    }
    const int width = match.captured(3).toInt(&ok);
    if (!ok) {
        return std::nullopt;
    }
    const int height = match.captured(4).toInt(&ok);
    if (!ok) {
        return std::nullopt;
    }
    return QRect(x, y, width, height);
}

/// @brief Attempts to extract a bounding rectangle from a JSON window object.
/// @param object The JSON object representing a window's metadata.
/// @return A QRect if a geometry was successfully extracted, std::nullopt otherwise.
std::optional<QRect> rectFromWindowObject(const QJsonObject &object)
{
    if (object.value(QStringLiteral("geometry")).isString()) {
        return rectFromGeometryText(object.value(QStringLiteral("geometry")).toString());
    }

    if (object.value(QStringLiteral("rect")).isArray()) {
        return rectFromArray(object.value(QStringLiteral("rect")).toArray());
    }

    if (object.value(QStringLiteral("at")).isArray() && object.value(QStringLiteral("size")).isArray()) {
        const QJsonArray at = object.value(QStringLiteral("at")).toArray();
        const QJsonArray size = object.value(QStringLiteral("size")).toArray();
        if (at.size() >= 2 && size.size() >= 2) {
            QJsonArray rect;
            rect.append(at.at(0));
            rect.append(at.at(1));
            rect.append(size.at(0));
            rect.append(size.at(1));
            return rectFromArray(rect);
        }
    }

    const std::optional<int> x = namedIntValue(object, {QStringLiteral("x"), QStringLiteral("left")});
    const std::optional<int> y = namedIntValue(object, {QStringLiteral("y"), QStringLiteral("top")});
    const std::optional<int> width = namedIntValue(object, {QStringLiteral("width"), QStringLiteral("w")});
    const std::optional<int> height = namedIntValue(object, {QStringLiteral("height"), QStringLiteral("h")});
    if (!x.has_value() || !y.has_value() || !width.has_value() || !height.has_value()) {
        return std::nullopt;
    }
    return QRect(*x, *y, *width, *height);
}

/// @brief Attempts to extract a WindowInfo from a JSON window object.
/// @param object The JSON object representing a window's metadata.
/// @return A WindowInfo if a geometry was successfully extracted, std::nullopt otherwise.
std::optional<WindowInfo> windowInfoFromWindowObject(const QJsonObject &object)
{
    std::optional<QRect> rect = rectFromWindowObject(object);
    if (!rect.has_value()) {
        return std::nullopt;
    }

    WindowInfo info;
    info.rect = *rect;

    const QJsonValue zOrderValue = object.value(QStringLiteral("zOrder"));
    if (zOrderValue.isDouble()) {
        info.zOrder = static_cast<int>(zOrderValue.toDouble());
    }

    return info;
}

/// @brief Normalizes a rectangle and appends it to the list of geometries if valid and not a duplicate.
/// @param rects Pointer to the destination vector of rectangles.
/// @param rect The rectangle geometry to add.
void appendValidRect(QVector<QRect> *rects, QRect rect)
{
    if (!rects) {
        return;
    }
    rect = rect.normalized();
    if (rect.width() <= 1 || rect.height() <= 1) {
        return;
    }
    if (!rects->contains(rect)) {
        rects->append(rect);
    }
}

/// @brief Normalizes a WindowInfo and appends it to the list if valid and not a duplicate.
/// @param infos Pointer to the destination vector of WindowInfo.
/// @param info The WindowInfo to add.
void appendValidWindowInfo(QVector<WindowInfo> *infos, WindowInfo info)
{
    if (!infos) {
        return;
    }
    info.rect = info.rect.normalized();
    if (info.rect.width() <= 1 || info.rect.height() <= 1) {
        return;
    }
    for (const WindowInfo &existing : *infos) {
        if (existing.rect == info.rect) {
            return;
        }
    }
    infos->append(info);
}

/// @brief Parses the standard output of the window detection script.
/// @param output The raw byte array output from the script.
/// @return A vector of parsed window info with optional z-order.
QVector<WindowInfo> parseWindowDetectionOutput(const QByteArray &output)
{
    QVector<WindowInfo> results;

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
    if (parseError.error != QJsonParseError::NoError) {
        markshot::debugLog("window-detection",
                           "script returned invalid JSON offset=%d error=%s",
                           parseError.offset,
                           parseError.errorString().toUtf8().constData());
        return results;
    }

    QJsonArray windows;
    if (document.isArray()) {
        windows = document.array();
    } else if (document.isObject()) {
        const QJsonObject root = document.object();
        if (root.value(QStringLiteral("windows")).isArray()) {
            windows = root.value(QStringLiteral("windows")).toArray();
        } else if (root.value(QStringLiteral("windowGeometries")).isArray()) {
            windows = root.value(QStringLiteral("windowGeometries")).toArray();
        } else if (const std::optional<WindowInfo> info = windowInfoFromWindowObject(root)) {
            appendValidWindowInfo(&results, *info);
            return results;
        }
    }

    for (const QJsonValue &value : windows) {
        std::optional<WindowInfo> info;
        if (value.isObject()) {
            info = windowInfoFromWindowObject(value.toObject());
        } else if (value.isArray()) {
            std::optional<QRect> rect = rectFromArray(value.toArray());
            if (rect.has_value()) {
                info = WindowInfo{*rect, std::nullopt};
            }
        } else if (value.isString()) {
            std::optional<QRect> rect = rectFromGeometryText(value.toString());
            if (rect.has_value()) {
                info = WindowInfo{*rect, std::nullopt};
            }
        }
        if (info.has_value()) {
            appendValidWindowInfo(&results, *info);
        }
    }

    return results;
}

/// @brief Reads and parses the application configuration file root object.
/// @return The root QJsonObject if read successfully, std::nullopt otherwise.
std::optional<QJsonObject> readAppConfigRoot()
{
    QFile file(appConfigPath());
    if (!file.exists()) {
        return std::nullopt;
    }
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        markshot::debugLog("window-detection",
                           "cannot read config path=%s",
                           appConfigPath().toUtf8().constData());
        return std::nullopt;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        markshot::debugLog("window-detection",
                           "config parse failed offset=%d error=%s",
                           parseError.offset,
                           parseError.errorString().toUtf8().constData());
        return std::nullopt;
    }

    return document.object();
}

/// @brief Checks if window detection is explicitly enabled or disabled in the config.
/// @param root The root configuration JSON object.
/// @return True/false if the setting is specified, std::nullopt otherwise.
std::optional<bool> configuredWindowDetectionEnabled(const QJsonObject &root)
{
    const QJsonValue value = root.value(QStringLiteral("windowDetection"));
    if (const std::optional<bool> enabled = config::boolValue(value)) {
        return enabled;
    }
    if (!value.isObject()) {
        return std::nullopt;
    }

    return config::boolValue(value.toObject().value(QStringLiteral("enabled")));
}

/// @brief 判断已配置的窗口检测命令是否匹配当前桌面环境。
/// @param command 用户配置中的窗口检测命令。
/// @return 匹配当前桌面环境时返回 true，否则返回 false。
bool commandMatchesEnvironment(const QString &command)
{
#if defined(Q_OS_WIN)
    return command.isEmpty();
#else
    const QString sessionType = detectWaylandSessionType();
    if (sessionType.isEmpty()) {
        return command.isEmpty();
    }
    if (command.isEmpty()) {
        return false;
    }
    return command.contains(sessionType);
#endif
}

/// @brief Reads and parses the window detection configuration from the application settings.
/// @return A WindowDetectionConfig struct if configured and enabled, std::nullopt otherwise.
std::optional<WindowDetectionConfig> readWindowDetectionConfig()
{
    const std::optional<QJsonObject> root = readAppConfigRoot();
    if (!root.has_value()) {
        return std::nullopt;
    }
    if (const std::optional<bool> enabled = configuredWindowDetectionEnabled(*root);
        enabled.has_value() && !*enabled) {
        return std::nullopt;
    }

    const QJsonObject windowDetection = config::objectValue(*root, QStringLiteral("windowDetection"));
    WindowDetectionConfig config;
    config.command = windowDetection.value(QStringLiteral("command")).toString().trimmed();
    config.workingDirectory = windowDetection.value(QStringLiteral("workingDirectory")).toString().trimmed();
    config.environment = environmentOverrides(windowDetection);
    if (config.workingDirectory.isEmpty()) {
        config.workingDirectory = windowDetection.value(QStringLiteral("cwd")).toString().trimmed();
    }
    if (const std::optional<int> timeoutMs = config::intValue(windowDetection.value(QStringLiteral("timeoutMs")))) {
        config.timeoutMs = std::clamp(*timeoutMs, kMinWindowDetectionTimeoutMs, kMaxWindowDetectionTimeoutMs);
    }

    if (!commandMatchesEnvironment(config.command)) {
        config.command = defaultWindowDetectionCommand();
    }
    if (config.command.isEmpty()) {
        return std::nullopt;
    }
    return config;
}

/// @brief Sets up environment variables passed to the window detection script.
/// @param captureGeometry The capture area geometry.
/// @param outputName The display output name.
/// @param allOutputs True if capturing all outputs.
/// @param overrides Map of additional environment variables to override.
/// @return The populated QProcessEnvironment.
QProcessEnvironment scriptEnvironment(const QRect &captureGeometry,
                                      const QString &outputName,
                                      bool allOutputs,
                                      const QMap<QString, QString> &overrides)
{
    QProcessEnvironment environment = QProcessEnvironment::systemEnvironment();
    environment.insert(QStringLiteral("MARK_SHOT_CONFIG"), appConfigPath());
    environment.insert(QStringLiteral("MARK_SHOT_CAPTURE_OUTPUT"), outputName);
    environment.insert(QStringLiteral("MARK_SHOT_CAPTURE_ALL_OUTPUTS"),
                       allOutputs ? QStringLiteral("1") : QStringLiteral("0"));
    if (captureGeometry.isValid() && !captureGeometry.isEmpty()) {
        const QRect geometry = captureGeometry.normalized();
        environment.insert(QStringLiteral("MARK_SHOT_CAPTURE_X"), QString::number(geometry.x()));
        environment.insert(QStringLiteral("MARK_SHOT_CAPTURE_Y"), QString::number(geometry.y()));
        environment.insert(QStringLiteral("MARK_SHOT_CAPTURE_WIDTH"), QString::number(geometry.width()));
        environment.insert(QStringLiteral("MARK_SHOT_CAPTURE_HEIGHT"), QString::number(geometry.height()));
    }
    for (auto it = overrides.constBegin(); it != overrides.constEnd(); ++it) {
        environment.insert(it.key(), it.value());
    }
    return environment;
}

} // namespace

bool windowDetectionEnabled()
{
    const std::optional<QJsonObject> root = readAppConfigRoot();
    if (!root.has_value()) {
        return true;
    }
    return configuredWindowDetectionEnabled(*root).value_or(true);
}

/// @brief Collects the window info detected by running the configured external script/command.
/// @param captureGeometry The current screen or capture area geometry.
/// @param outputName The name of the preferred display output.
/// @param allOutputs Flag indicating whether capturing should target all outputs.
/// @return A vector of detected window info with optional z-order.
QVector<WindowInfo> collectConfiguredWindowInfos(const QRect &captureGeometry,
                                                 const QString &outputName,
                                                 bool allOutputs)
{
    const std::optional<WindowDetectionConfig> config = readWindowDetectionConfig();
    if (!config.has_value()) {
        return {};
    }

    QProcess process;
    process.setProgram(commandShellProgram());
    process.setArguments(commandShellArguments(config->command));
    process.setProcessEnvironment(scriptEnvironment(captureGeometry,
                                                   outputName,
                                                   allOutputs,
                                                   config->environment));
    if (!config->workingDirectory.isEmpty()) {
        process.setWorkingDirectory(expandUserPath(config->workingDirectory));
    }
    process.start(QIODevice::ReadOnly);

    if (!process.waitForStarted(1000)) {
        markshot::debugLog("window-detection", "script did not start");
        return {};
    }
    if (!process.waitForFinished(config->timeoutMs)) {
        process.kill();
        process.waitForFinished(1000);
        markshot::debugLog("window-detection",
                           "script timed out timeout_ms=%d",
                           config->timeoutMs);
        return {};
    }
    if (process.exitStatus() != QProcess::NormalExit || process.exitCode() != 0) {
        const QByteArray stderrText = process.readAllStandardError().trimmed().left(512);
        markshot::debugLog("window-detection",
                           "script failed exit_code=%d stderr=%s",
                           process.exitCode(),
                           stderrText.constData());
        return {};
    }

    const QVector<WindowInfo> windows = parseWindowDetectionOutput(process.readAllStandardOutput());
    markshot::debugLog("window-detection", "script returned windows=%d", static_cast<int>(windows.size()));
    return windows;
}

} // namespace markshot
