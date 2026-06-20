#include "shot_window_module.h"

namespace cfg = markshot::config;

namespace markshot::shot {

/// @brief Checks if the given string consists only of valid hexadecimal color digits.
/// @param text The string to check.
/// @return True if the string contains only hex color digits, false otherwise.
bool isHexColorDigits(const QString &text)
{
    if (text.isEmpty()) {
        return false;
    }
    for (const QChar ch : text) {
        if (!ch.isDigit()
            && (ch < QLatin1Char('a') || ch > QLatin1Char('f'))
            && (ch < QLatin1Char('A') || ch > QLatin1Char('F'))) {
            return false;
        }
    }
    return true;
}

/// @brief Parses a QColor from a configuration string.
/// @param value The configuration string to parse.
/// @return An optional QColor if parsing was successful, otherwise std::nullopt.
std::optional<QColor> colorFromConfigString(QString value)
{
    QString text = value.trimmed();
    if (text.isEmpty()) {
        return std::nullopt;
    }
    if (text.startsWith(QStringLiteral("0x"), Qt::CaseInsensitive)) {
        text = text.mid(2);
    }
    if (!text.startsWith(QLatin1Char('#')) && (text.size() == 6 || text.size() == 8)
        && isHexColorDigits(text)) {
        text.prepend(QLatin1Char('#'));
    }
    if (text.startsWith(QLatin1Char('#')) && text.size() == 9 && isHexColorDigits(text.mid(1))) {
        bool ok = false;
        const int red = text.mid(1, 2).toInt(&ok, 16);
        if (!ok) return std::nullopt;
        const int green = text.mid(3, 2).toInt(&ok, 16);
        if (!ok) return std::nullopt;
        const int blue = text.mid(5, 2).toInt(&ok, 16);
        if (!ok) return std::nullopt;
        const int alpha = text.mid(7, 2).toInt(&ok, 16);
        if (!ok) return std::nullopt;
        return QColor(red, green, blue, alpha);
    }

    const QColor color(text);
    return color.isValid() ? std::optional<QColor>(color) : std::nullopt;
}

/// @brief Parses a QColor from a JSON value (supports string, hex, or RGB(A) objects).
/// @param value The JSON value representing the color.
/// @return An optional QColor if parsing was successful, otherwise std::nullopt.
std::optional<QColor> colorFromConfigValue(const QJsonValue &value)
{
    if (value.isString()) {
        return colorFromConfigString(value.toString());
    }
    if (!value.isObject()) {
        return std::nullopt;
    }

    const QJsonObject object = value.toObject();
    for (const QString &key : {QStringLiteral("value"), QStringLiteral("color"), QStringLiteral("hex")}) {
        if (object.value(key).isString()) {
            return colorFromConfigString(object.value(key).toString());
        }
    }
    if (!object.value(QStringLiteral("r")).isDouble()
        || !object.value(QStringLiteral("g")).isDouble()
        || !object.value(QStringLiteral("b")).isDouble()) {
        return std::nullopt;
    }
    const int alpha = object.value(QStringLiteral("a")).isDouble()
        ? std::clamp(object.value(QStringLiteral("a")).toInt(), 0, 255)
        : 255;
    return QColor(std::clamp(object.value(QStringLiteral("r")).toInt(), 0, 255),
                  std::clamp(object.value(QStringLiteral("g")).toInt(), 0, 255),
                  std::clamp(object.value(QStringLiteral("b")).toInt(), 0, 255),
                  alpha);
}

/// @brief Extracts a floating-point number from a JSON value.
/// @param value The JSON value to parse.
/// @return An optional floating-point number if valid, otherwise std::nullopt.
std::optional<qreal> realFromConfigValue(const QJsonValue &value)
{
    if (value.isDouble()) {
        return value.toDouble();
    }
    if (value.isString()) {
        bool ok = false;
        const qreal number = value.toString().trimmed().toDouble(&ok);
        if (ok) {
            return number;
        }
    }
    return std::nullopt;
}

/// @brief Parses a pixel distance from a JSON value and clamps it within a range.
/// @param value The JSON value representing the distance.
/// @param minimum The minimum allowable distance.
/// @param maximum The maximum allowable distance.
/// @return An optional integer distance if valid, otherwise std::nullopt.
std::optional<int> pixelDistanceFromConfigValue(const QJsonValue &value,
                                                int minimum,
                                                int maximum)
{
    if (const std::optional<qreal> distance = realFromConfigValue(value)) {
        return std::clamp(static_cast<int>(std::lround(*distance)), minimum, maximum);
    }
    return std::nullopt;
}

/// @brief Extracts a list of strings from a JSON value.
/// @param value The JSON value containing a string or an array of strings.
/// @return A list of extracted strings.
QStringList jsonStringList(const QJsonValue &value)
{
    QStringList items;
    if (value.isString()) {
        items.append(value.toString());
    } else if (value.isArray()) {
        for (const QJsonValue &item : value.toArray()) {
            if (item.isString()) {
                items.append(item.toString());
            }
        }
    }
    return items;
}

/// @brief Checks if the given OCR error text suggests a missing backend.
/// @param errorText The error message to check.
/// @return True if the error indicates a missing backend, false otherwise.
bool ocrErrorLooksLikeMissingBackend(QString errorText)
{
    const QString text = errorText.toLower();
    return text.contains(QStringLiteral("no module named"))
        || text.contains(QStringLiteral("modulenotfounderror"))
        || text.contains(QStringLiteral("importerror"))
        || text.contains(QStringLiteral("not installed"))
        || text.contains(QStringLiteral("not found"))
        || text.contains(QStringLiteral("no such file or directory"));
}

bool ocrOutputReportsMissingBackend(const QByteArray &stdoutData,
                                    const QByteArray &stderrData,
                                    const QString &configuredBackend)
{
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(stdoutData, &parseError);
    QStringList errors;
    QString reportedBackend;
    QJsonArray tokenArray;
    if (parseError.error == QJsonParseError::NoError && document.isObject()) {
        const QJsonObject root = document.object();
        reportedBackend = root.value(QStringLiteral("backend")).toString().trimmed();
        errors = jsonStringList(root.value(QStringLiteral("errors")));
        tokenArray = root.value(QStringLiteral("tokens")).toArray();
    }

    if (!stderrData.trimmed().isEmpty()) {
        errors.append(QString::fromUtf8(stderrData).trimmed());
    }

    if (tokenArray.isEmpty() && reportedBackend.isEmpty() && !errors.isEmpty()) {
        return true;
    }

    const QString backend = configuredBackend.trimmed().toLower();
    if (backend == QStringLiteral("auto")) {
        return false;
    }

    for (const QString &error : std::as_const(errors)) {
        if (ocrErrorLooksLikeMissingBackend(error)) {
            return true;
        }
    }

    return tokenArray.isEmpty() && !errors.isEmpty()
        && reportedBackend.compare(backend, Qt::CaseInsensitive) == 0;
}

/**
 * 读取扫码配置。
 * @return 扫码命令与超时时间配置。
 */
CodeScanConfig codeScanConfig()
{
    CodeScanConfig config;

    QFile file(appConfigPath());
    if (file.exists() && file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
        if (parseError.error == QJsonParseError::NoError && document.isObject()) {
            const QJsonObject root = document.object();
            const QJsonObject codeScan =
                cfg::firstObjectValue(root,
                                      {QStringLiteral("codeScan"),
                                       QStringLiteral("codeScanner"),
                                       QStringLiteral("barcodeScanner"),
                                       QStringLiteral("barcode")});
            config.command = codeScan.value(QStringLiteral("command"))
                                 .toString(codeScan.value(QStringLiteral("path"))
                                               .toString(codeScan.value(QStringLiteral("helper")).toString()))
                                 .trimmed();
            if (codeScan.value(QStringLiteral("timeoutMs")).isDouble()) {
                config.timeoutMs = std::max(1000,
                                            codeScan.value(QStringLiteral("timeoutMs")).toInt(config.timeoutMs));
            }
        }
    }

    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString envCommand = env.value(QStringLiteral("MARK_SHOT_CODE_SCAN_COMMAND")).trimmed();
    if (!envCommand.isEmpty()) {
        config.command = envCommand;
    }
    bool timeoutOk = false;
    const int envTimeout = env.value(QStringLiteral("MARK_SHOT_CODE_SCAN_TIMEOUT_MS")).trimmed().toInt(&timeoutOk);
    if (timeoutOk) {
        config.timeoutMs = std::max(1000, envTimeout);
    }
    return config;
}

/**
 * 读取图床上传配置。
 * @return 上传命令与超时时间配置。
 */
UploadConfig uploadConfig()
{
    UploadConfig config;

    QFile file(appConfigPath());
    if (file.exists() && file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
        if (parseError.error == QJsonParseError::NoError && document.isObject()) {
            const QJsonObject root = document.object();
            const QJsonObject upload =
                cfg::firstObjectValue(root,
                                      {QStringLiteral("upload"),
                                       QStringLiteral("imageUpload"),
                                       QStringLiteral("uploader"),
                                       QStringLiteral("imageHost")});
            config.command = upload.value(QStringLiteral("command"))
                                 .toString(upload.value(QStringLiteral("path"))
                                               .toString(upload.value(QStringLiteral("helper")).toString()))
                                 .trimmed();
            if (upload.value(QStringLiteral("timeoutMs")).isDouble()) {
                config.timeoutMs = std::max(1000,
                                            upload.value(QStringLiteral("timeoutMs")).toInt(config.timeoutMs));
            }
            // 读取 env 对象,合并到环境变量中(覆盖系统环境变量)。
            const QJsonObject envObject =
                cfg::firstObjectValue(upload,
                                      {QStringLiteral("env"),
                                       QStringLiteral("environment"),
                                       QStringLiteral("envVars"),
                                       QStringLiteral("variables")});
            for (auto it = envObject.constBegin(); it != envObject.constEnd(); ++it) {
                const QString key = it.key().trimmed();
                const QJsonValue value = it.value();
                if (key.isEmpty() || !value.isString()) {
                    continue;
                }
                config.env.insert(key, value.toString());
            }
        }
    }

    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString envCommand = env.value(QStringLiteral("MARK_SHOT_UPLOAD_COMMAND")).trimmed();
    if (!envCommand.isEmpty()) {
        config.command = envCommand;
    }
    bool timeoutOk = false;
    const int envTimeout = env.value(QStringLiteral("MARK_SHOT_UPLOAD_TIMEOUT_MS")).trimmed().toInt(&timeoutOk);
    if (timeoutOk) {
        config.timeoutMs = std::max(1000, envTimeout);
    }
    return config;
}

/// @brief Parses a boolean value from the OCR result panel JSON configuration.
/// @param value The JSON value to parse.
/// @return An optional boolean value if parsing was successful, otherwise std::nullopt.
std::optional<bool> boolFromResultPanelValue(const QJsonValue &value)
{
    if (const std::optional<bool> result = boolFromConfigValue(value)) {
        return result;
    }
    if (!value.isObject()) {
        return std::nullopt;
    }
    return cfg::boolValue(value.toObject(),
                     {QStringLiteral("enabled"),
                      QStringLiteral("show"),
                      QStringLiteral("visible"),
                      QStringLiteral("use")});
}

bool ocrResultPanelEnabled()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    if (const std::optional<bool> envValue =
            envFlagValue(env,
                         {QStringLiteral("MARK_SHOT_OCR_RESULT_PANEL"),
                          QStringLiteral("MARK_SHOT_OCR_RESULT_WINDOW")})) {
        return *envValue;
    }

    QFile file(appConfigPath());
    if (file.exists() && file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
        if (parseError.error == QJsonParseError::NoError && document.isObject()) {
            const QJsonObject root = document.object();
            const QStringList ocrKeys = {
                QStringLiteral("resultPanel"),
                QStringLiteral("resultWindow"),
                QStringLiteral("resultPanelEnabled"),
                QStringLiteral("resultWindowEnabled"),
                QStringLiteral("ocrResultPanel"),
                QStringLiteral("ocrResultWindow"),
                QStringLiteral("showResultPanel"),
                QStringLiteral("showResultWindow"),
                QStringLiteral("useResultPanel"),
                QStringLiteral("useResultWindow"),
                QStringLiteral("newResultPanel"),
                QStringLiteral("newResultWindow"),
            };

            const QJsonObject ocr = cfg::firstObjectValue(root, QStringLiteral("ocr"));
            if (const std::optional<bool> ocrValue =
                    boolFromResultPanelValue(cfg::valueForKeys(ocr, ocrKeys))) {
                return *ocrValue;
            }

            if (const std::optional<bool> rootValue =
                    boolFromResultPanelValue(cfg::valueForKeys(root,
                                                          {QStringLiteral("ocrResultPanel"),
                                                           QStringLiteral("ocrResultWindow"),
                                                           QStringLiteral("ocrResultPanelEnabled"),
                                                           QStringLiteral("ocrResultWindowEnabled")}))) {
                return *rootValue;
            }
        }
    }

    return true;
}

using AutoSelectTools = std::array<bool, static_cast<int>(ShotWindow::Tool::Laser) + 1>;

bool isAutoSelectDefaultKey(const QString &key)
{
    const QString normalized = cfg::normalizedKey(key);
    return normalized == QStringLiteral("default")
        || normalized == QStringLiteral("all")
        || normalized == QStringLiteral("enabled")
        || normalized == QStringLiteral("value");
}

void applyAutoSelectAfterDrawValue(const QJsonValue &value, AutoSelectTools *tools)
{
    if (!tools || value.isUndefined() || value.isNull()) {
        return;
    }

    if (value.isObject()) {
        const QJsonObject object = value.toObject();
        if (const std::optional<bool> defaultValue =
                cfg::boolValue(cfg::valueForKeys(object,
                                                 {QStringLiteral("default"),
                                                  QStringLiteral("all"),
                                                  QStringLiteral("enabled"),
                                                  QStringLiteral("value")}))) {
            tools->fill(*defaultValue);
        }
        for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
            if (isAutoSelectDefaultKey(it.key())) {
                continue;
            }
            const std::optional<ShotWindow::Tool> tool = ShotWindow::toolFromName(it.key());
            if (!tool.has_value()) {
                continue;
            }
            if (const std::optional<bool> enabled = cfg::boolValue(it.value())) {
                tools->at(static_cast<int>(*tool)) = *enabled;
            }
        }
        return;
    }

    if (const std::optional<bool> enabled = cfg::boolValue(value)) {
        tools->fill(*enabled);
    }
}

void applyAutoSelectAfterDrawEnvironment(AutoSelectTools *tools)
{
    if (!tools) {
        return;
    }

    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    if (const std::optional<bool> envValue =
            envFlagValue(env,
                         {QStringLiteral("MARK_SHOT_AUTO_SELECT_AFTER_DRAW"),
                          QStringLiteral("MARK_SHOT_SELECT_AFTER_DRAW")})) {
        tools->fill(*envValue);
    }

    for (const QString &toolName : ShotWindow::supportedToolNames()) {
        const std::optional<ShotWindow::Tool> tool = ShotWindow::toolFromName(toolName);
        if (!tool.has_value()) {
            continue;
        }
        QString envToolName = toolName.toUpper();
        envToolName.replace(QLatin1Char('-'), QLatin1Char('_'));
        if (const std::optional<bool> toolValue =
                envFlagValue(env,
                             {QStringLiteral("MARK_SHOT_AUTO_SELECT_AFTER_DRAW_%1").arg(envToolName),
                              QStringLiteral("MARK_SHOT_SELECT_AFTER_DRAW_%1").arg(envToolName)})) {
            tools->at(static_cast<int>(*tool)) = *toolValue;
        }
    }
}

AutoSelectTools annotationAutoSelectAfterDrawTools()
{
    AutoSelectTools tools = {};

    QFile file(appConfigPath());
    if (file.exists() && file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
        if (parseError.error == QJsonParseError::NoError && document.isObject()) {
            const QJsonObject root = document.object();
            const QStringList keys = {
                QStringLiteral("autoSelectAfterDraw"),
                QStringLiteral("selectAfterDraw"),
                QStringLiteral("autoSelectAfterCommit"),
                QStringLiteral("selectAfterCommit"),
                QStringLiteral("autoSelectAfterAnnotation"),
                QStringLiteral("selectAfterAnnotation"),
            };
            applyAutoSelectAfterDrawValue(cfg::valueForKeys(root, keys), &tools);

            const QJsonObject annotations =
                cfg::firstObjectValue(root,
                                      {QStringLiteral("annotations"),
                                       QStringLiteral("annotation"),
                                       QStringLiteral("drawing"),
                                       QStringLiteral("tools")});
            applyAutoSelectAfterDrawValue(cfg::valueForKeys(annotations, keys), &tools);
        }
    }

    applyAutoSelectAfterDrawEnvironment(&tools);
    return tools;
}

bool annotationAutoSelectAfterDrawEnabled()
{
    const AutoSelectTools tools = annotationAutoSelectAfterDrawTools();
    return std::any_of(tools.cbegin(), tools.cend(), [](bool enabled) { return enabled; });
}

/// @brief Applies frame configuration from a JSON value to a scroll session UI configuration.
/// @param value The JSON value containing the frame configuration.
/// @param config Pointer to the scroll session UI config to be updated.
void applyScrollFrameConfig(const QJsonValue &value, markshot::scroll::ScrollSessionUiConfig *config)
{
    if (!config || value.isUndefined()) {
        return;
    }

    if (value.isBool()) {
        config->frameEnabled = value.toBool();
        return;
    }
    if (const std::optional<int> gap = pixelDistanceFromConfigValue(value, 0, 256)) {
        config->frameEnabled = true;
        config->frameGap = *gap;
        return;
    }
    if (!value.isObject()) {
        return;
    }

    const QJsonObject object = value.toObject();
    if (const std::optional<bool> enabled =
            cfg::boolValue(object,
                      {QStringLiteral("enabled"),
                       QStringLiteral("visible"),
                       QStringLiteral("show"),
                       QStringLiteral("showFrame"),
                       QStringLiteral("showBorder")});
        enabled.has_value()) {
        config->frameEnabled = *enabled;
    } else {
        config->frameEnabled = true;
    }

    const QJsonValue gapValue =
        cfg::valueForKeys(object,
                     {QStringLiteral("gap"),
                      QStringLiteral("distance"),
                      QStringLiteral("offset"),
                      QStringLiteral("padding"),
                      QStringLiteral("margin"),
                      QStringLiteral("value"),
                      QStringLiteral("px")});
    if (const std::optional<int> gap = pixelDistanceFromConfigValue(gapValue, 0, 256)) {
        config->frameGap = *gap;
    }
}

/// @brief Applies preview configuration from a JSON value to a scroll session UI configuration.
/// @param value The JSON value containing the preview configuration.
/// @param config Pointer to the scroll session UI config to be updated.
void applyScrollPreviewConfig(const QJsonValue &value, markshot::scroll::ScrollSessionUiConfig *config)
{
    if (!config || value.isUndefined()) {
        return;
    }
    if (const std::optional<int> gap = pixelDistanceFromConfigValue(value, 0, 512)) {
        config->previewGap = *gap;
        return;
    }
    if (!value.isObject()) {
        return;
    }

    const QJsonObject object = value.toObject();
    if (const std::optional<bool> hideDuringCapture =
            cfg::boolValue(object,
                      {QStringLiteral("hideDuringCapture"),
                       QStringLiteral("hideWhileCapturing"),
                       QStringLiteral("hidePreviewDuringCapture"),
                       QStringLiteral("hidePreviewWhileCapturing"),
                       QStringLiteral("hidePanelDuringCapture"),
                       QStringLiteral("hidePanelWhileCapturing"),
                       QStringLiteral("hideUiDuringCapture"),
                       QStringLiteral("hideUIDuringCapture"),
                       QStringLiteral("hideUiWhileCapturing"),
                       QStringLiteral("hideUIWhileCapturing")});
        hideDuringCapture.has_value()) {
        config->hidePreviewDuringCapture = *hideDuringCapture;
    }

    const QJsonValue gapValue =
        cfg::valueForKeys(object,
                     {QStringLiteral("gap"),
                      QStringLiteral("distance"),
                      QStringLiteral("offset"),
                      QStringLiteral("margin"),
                      QStringLiteral("value"),
                      QStringLiteral("px")});
    if (const std::optional<int> gap = pixelDistanceFromConfigValue(gapValue, 0, 512)) {
        config->previewGap = *gap;
    }
}

markshot::scroll::ScrollSessionUiConfig scrollSessionUiConfig()
{
    markshot::scroll::ScrollSessionUiConfig config;

    QFile file(appConfigPath());
    if (!file.exists() || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return config;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return config;
    }

    const QJsonObject root = document.object();
    const QJsonObject scroll =
        cfg::firstObjectValue(root,
                    {QStringLiteral("scrollCapture"),
                     QStringLiteral("scrollingCapture"),
                     QStringLiteral("scroll")});
    if (scroll.isEmpty()) {
        return config;
    }

    applyScrollFrameConfig(cfg::valueForKeys(scroll,
                                        {QStringLiteral("frame"),
                                         QStringLiteral("captureFrame"),
                                         QStringLiteral("border"),
                                         QStringLiteral("outline")}),
                           &config);

    if (const std::optional<bool> frameEnabled =
            cfg::boolValue(scroll,
                      {QStringLiteral("frameEnabled"),
                       QStringLiteral("borderEnabled"),
                       QStringLiteral("showFrame"),
                       QStringLiteral("showBorder")});
        frameEnabled.has_value()) {
        config.frameEnabled = *frameEnabled;
    }
    if (const std::optional<int> frameGap =
            pixelDistanceFromConfigValue(cfg::valueForKeys(scroll,
                                                      {QStringLiteral("frameGap"),
                                                       QStringLiteral("frameDistance"),
                                                       QStringLiteral("frameOffset"),
                                                       QStringLiteral("borderGap"),
                                                       QStringLiteral("borderDistance"),
                                                       QStringLiteral("borderOffset")}),
                                         0,
                                         256)) {
        config.frameGap = *frameGap;
    }

    applyScrollPreviewConfig(cfg::valueForKeys(scroll,
                                          {QStringLiteral("preview"),
                                           QStringLiteral("previewUi"),
                                           QStringLiteral("previewUI"),
                                           QStringLiteral("panel")}),
                             &config);
    if (const std::optional<int> previewGap =
            pixelDistanceFromConfigValue(cfg::valueForKeys(scroll,
                                                      {QStringLiteral("previewGap"),
                                                       QStringLiteral("previewDistance"),
                                                       QStringLiteral("previewOffset"),
                                                       QStringLiteral("panelGap"),
                                                       QStringLiteral("panelDistance"),
                                                       QStringLiteral("panelOffset")}),
                                         0,
                                         512)) {
        config.previewGap = *previewGap;
    }
    if (const std::optional<bool> hidePreviewDuringCapture =
            cfg::boolValue(scroll,
                      {QStringLiteral("hidePreviewDuringCapture"),
                       QStringLiteral("hidePreviewWhileCapturing"),
                       QStringLiteral("hidePanelDuringCapture"),
                       QStringLiteral("hidePanelWhileCapturing"),
                       QStringLiteral("hideUiDuringCapture"),
                       QStringLiteral("hideUIDuringCapture"),
                       QStringLiteral("hideUiWhileCapturing"),
                       QStringLiteral("hideUIWhileCapturing"),
                       QStringLiteral("alwaysHidePreview"),
                       QStringLiteral("forceHidePreview")});
        hidePreviewDuringCapture.has_value()) {
        config.hidePreviewDuringCapture = *hidePreviewDuringCapture;
    }

    return config;
}

}  // namespace markshot::shot
