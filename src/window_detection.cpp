#include "window_detection.h"

#include "debug_log.h"
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

QString defaultAppConfigDir()
{
    const QStringList candidates = appConfigDirCandidates();
    return candidates.isEmpty()
        ? QDir::home().filePath(QStringLiteral(".config/mark-shot"))
        : candidates.first();
}

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

QJsonObject defaultAppConfigRoot()
{
    QJsonObject root;
    root.insert(QStringLiteral("env"), QJsonObject());

    QJsonObject annotation;
    annotation.insert(QStringLiteral("defaultTool"), QStringLiteral("pen"));
    annotation.insert(QStringLiteral("fullscreenDefaultTool"), QStringLiteral("pen"));
    annotation.insert(QStringLiteral("defaultColor"), QStringLiteral("#FF4D4D"));
    root.insert(QStringLiteral("annotation"), annotation);

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

    QJsonObject pinnedWindow;
    pinnedWindow.insert(QStringLiteral("autoOcr"), false);
    pinnedWindow.insert(QStringLiteral("border"), true);
    pinnedWindow.insert(QStringLiteral("borderColor"), markshot::theme::kAccent.name(QColor::HexRgb).toUpper());
    pinnedWindow.insert(QStringLiteral("borderWidth"), 2);
    root.insert(QStringLiteral("pinnedWindow"), pinnedWindow);

    QJsonObject scrollCapture;
    scrollCapture.insert(QStringLiteral("frame"), 5);
    scrollCapture.insert(QStringLiteral("previewGap"), 5);
    scrollCapture.insert(QStringLiteral("hidePreviewDuringCapture"), false);
    root.insert(QStringLiteral("scrollCapture"), scrollCapture);

    QJsonObject windowDetection;
    windowDetection.insert(QStringLiteral("enabled"), true);
#if defined(Q_OS_WIN)
    windowDetection.insert(QStringLiteral("command"), QString());
#else
    windowDetection.insert(QStringLiteral("command"), QStringLiteral("mark-shot-window-detection-niri"));
#endif
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

constexpr int kDefaultWindowDetectionTimeoutMs = 1000;
constexpr int kMinWindowDetectionTimeoutMs = 100;
constexpr int kMaxWindowDetectionTimeoutMs = 10000;

struct WindowDetectionConfig {
    QString command;
    QString workingDirectory;
    QMap<QString, QString> environment;
    int timeoutMs = kDefaultWindowDetectionTimeoutMs;
};

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

QString commandShellProgram()
{
#if defined(Q_OS_WIN)
    const QString comspec = QProcessEnvironment::systemEnvironment().value(QStringLiteral("COMSPEC"));
    return comspec.isEmpty() ? QStringLiteral("cmd.exe") : comspec;
#else
    QString shell = QProcessEnvironment::systemEnvironment().value(QStringLiteral("SHELL"),
                                                                   QStringLiteral("/bin/sh"));
    return shell.isEmpty() ? QStringLiteral("/bin/sh") : shell;
#endif
}

QStringList commandShellArguments(const QString &commandLine)
{
#if defined(Q_OS_WIN)
    return {QStringLiteral("/D"), QStringLiteral("/V:OFF"), QStringLiteral("/S"), QStringLiteral("/C"), commandLine};
#else
    return {QStringLiteral("-c"), commandLine};
#endif
}

QJsonObject objectValue(const QJsonObject &object, const QString &key)
{
    const QJsonValue value = object.value(key);
    return value.isObject() ? value.toObject() : QJsonObject();
}

std::optional<int> intValue(const QJsonValue &value)
{
    if (value.isDouble()) {
        return value.toInt();
    }
    if (value.isString()) {
        bool ok = false;
        const int number = value.toString().trimmed().toInt(&ok);
        if (ok) {
            return number;
        }
    }
    return std::nullopt;
}

std::optional<bool> boolValue(const QJsonValue &value)
{
    if (value.isBool()) {
        return value.toBool();
    }
    if (value.isDouble()) {
        return !qFuzzyIsNull(value.toDouble());
    }
    if (value.isString()) {
        QString text = value.toString().trimmed().toLower();
        if (text == QStringLiteral("1")
            || text == QStringLiteral("true")
            || text == QStringLiteral("yes")
            || text == QStringLiteral("on")
            || text == QStringLiteral("enabled")) {
            return true;
        }
        if (text == QStringLiteral("0")
            || text == QStringLiteral("false")
            || text == QStringLiteral("no")
            || text == QStringLiteral("off")
            || text == QStringLiteral("disabled")) {
            return false;
        }
    }
    return std::nullopt;
}

std::optional<int> namedIntValue(const QJsonObject &object, const QStringList &keys)
{
    for (const QString &key : keys) {
        const std::optional<int> value = intValue(object.value(key));
        if (value.has_value()) {
            return value;
        }
    }
    return std::nullopt;
}

std::optional<QString> environmentStringValue(const QJsonValue &value)
{
    if (value.isString()) {
        return value.toString();
    }
    if (value.isDouble()) {
        return QString::number(value.toDouble(), 'g', 15);
    }
    if (value.isBool()) {
        return value.toBool() ? QStringLiteral("1") : QStringLiteral("0");
    }
    return std::nullopt;
}

QMap<QString, QString> environmentOverrides(const QJsonObject &windowDetection)
{
    QJsonObject environment = objectValue(windowDetection, QStringLiteral("env"));
    const QJsonObject namedEnvironment = objectValue(windowDetection, QStringLiteral("environment"));
    for (auto it = namedEnvironment.constBegin(); it != namedEnvironment.constEnd(); ++it) {
        environment.insert(it.key(), it.value());
    }

    QMap<QString, QString> overrides;
    for (auto it = environment.constBegin(); it != environment.constEnd(); ++it) {
        const QString key = it.key().trimmed();
        if (key.isEmpty()) {
            continue;
        }
        if (const std::optional<QString> value = environmentStringValue(it.value())) {
            overrides.insert(key, *value);
        }
    }
    return overrides;
}

std::optional<QRect> rectFromArray(const QJsonArray &array)
{
    if (array.size() < 4) {
        return std::nullopt;
    }

    const std::optional<int> x = intValue(array.at(0));
    const std::optional<int> y = intValue(array.at(1));
    const std::optional<int> width = intValue(array.at(2));
    const std::optional<int> height = intValue(array.at(3));
    if (!x.has_value() || !y.has_value() || !width.has_value() || !height.has_value()) {
        return std::nullopt;
    }
    return QRect(*x, *y, *width, *height);
}

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

QVector<QRect> parseWindowDetectionOutput(const QByteArray &output)
{
    QVector<QRect> results;

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
        } else if (const std::optional<QRect> rect = rectFromWindowObject(root)) {
            appendValidRect(&results, *rect);
            return results;
        }
    }

    for (const QJsonValue &value : windows) {
        std::optional<QRect> rect;
        if (value.isObject()) {
            rect = rectFromWindowObject(value.toObject());
        } else if (value.isArray()) {
            rect = rectFromArray(value.toArray());
        } else if (value.isString()) {
            rect = rectFromGeometryText(value.toString());
        }
        if (rect.has_value()) {
            appendValidRect(&results, *rect);
        }
    }

    return results;
}

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

std::optional<bool> configuredWindowDetectionEnabled(const QJsonObject &root)
{
    const QJsonValue value = root.value(QStringLiteral("windowDetection"));
    if (const std::optional<bool> enabled = boolValue(value)) {
        return enabled;
    }
    if (!value.isObject()) {
        return std::nullopt;
    }

    return boolValue(value.toObject().value(QStringLiteral("enabled")));
}

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

    const QJsonObject windowDetection = objectValue(*root, QStringLiteral("windowDetection"));
    WindowDetectionConfig config;
    config.command = windowDetection.value(QStringLiteral("command")).toString().trimmed();
    config.workingDirectory = windowDetection.value(QStringLiteral("workingDirectory")).toString().trimmed();
    config.environment = environmentOverrides(windowDetection);
    if (config.workingDirectory.isEmpty()) {
        config.workingDirectory = windowDetection.value(QStringLiteral("cwd")).toString().trimmed();
    }
    if (const std::optional<int> timeoutMs = intValue(windowDetection.value(QStringLiteral("timeoutMs")))) {
        config.timeoutMs = std::clamp(*timeoutMs, kMinWindowDetectionTimeoutMs, kMaxWindowDetectionTimeoutMs);
    }

    if (config.command.isEmpty()) {
        return std::nullopt;
    }
    return config;
}

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

QVector<QRect> collectConfiguredWindowGeometries(const QRect &captureGeometry,
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

    const QVector<QRect> windows = parseWindowDetectionOutput(process.readAllStandardOutput());
    markshot::debugLog("window-detection", "script returned windows=%d", static_cast<int>(windows.size()));
    return windows;
}

} // namespace markshot
