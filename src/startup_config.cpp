#include "startup_config.h"

#include "annotation_state_store.h"
#include "config_value.h"
#include "debug_log.h"
#include "ui/i18n.h"
#include "window_detection.h"

#include <QByteArray>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QProcessEnvironment>
#include <QStringList>

#include <optional>

namespace {

/// @brief 生成应用配置文件候选路径。
/// @return 候选配置文件路径列表。
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

/// @brief 返回首次存在的预应用配置文件路径。
/// @return 配置文件路径。
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

/// @brief 按候选键读取第一个非空字符串。
/// @param object JSON 对象。
/// @param keys 候选键。
/// @return 读取到的字符串。
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

/// @brief 返回列表中的第一个非空字符串。
/// @param values 候选字符串列表。
/// @return 第一个非空字符串。
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

/// @brief 按候选键读取第一个已定义 JSON 值。
/// @param object JSON 对象。
/// @param keys 候选键。
/// @return 读取到的 JSON 值。
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

/// @brief 判断字符串是否全部为十六进制字符。
/// @param text 要检查的字符串。
/// @return 全部为十六进制字符时返回 true。
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

/// @brief 读取颜色通道值。
/// @param object JSON 对象。
/// @param keys 候选键。
/// @return 限制到 0 到 255 的通道值。
std::optional<int> channelValue(const QJsonObject &object, const QStringList &keys)
{
    return markshot::config::clampedIntValue(jsonValue(object, keys), 0, 255);
}

/// @brief 从 JSON 对象解析颜色。
/// @param object 颜色 JSON 对象。
/// @return 解析出的颜色。
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
        return markshot::colorFromString(nestedColor.toString());
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

/// @brief 从 JSON 值解析颜色。
/// @param value JSON 值。
/// @return 解析出的颜色。
std::optional<QColor> colorFromValue(const QJsonValue &value)
{
    if (value.isString()) {
        return markshot::colorFromString(value.toString());
    }
    if (value.isObject()) {
        return colorFromObject(value.toObject());
    }
    return std::nullopt;
}

}  // namespace

namespace markshot {

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
            tools.colorSource = DefaultColorSource::Config;
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

bool shouldApplyDefaultColor(const DefaultTools &tools)
{
    const bool annotationStateExists = QFileInfo::exists(markshot::annotationStateFilePath());
    return shouldApplyDefaultColor(tools.colorSource, annotationStateExists);
}

void disableQtPortalServicesForHostApp()
{
#if defined(Q_OS_UNIX) && !defined(Q_OS_DARWIN)
    if (qEnvironmentVariableIsSet("QT_NO_XDG_DESKTOP_PORTAL")) {
        return;
    }
    if (QFileInfo::exists(QStringLiteral("/.flatpak-info")) || qEnvironmentVariableIsSet("SNAP")) {
        return;
    }

    // Qt 6.10+ 会在应用 id 注册前查询 portal，导致部分 portal 绑定错误的调用方
    qputenv("QT_NO_XDG_DESKTOP_PORTAL", QByteArrayLiteral("1"));
#endif
}

}  // namespace markshot
