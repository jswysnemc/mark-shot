#include "settings/settings_config.h"

#include "app_config_store.h"
#include "autostart/autostart_manager.h"
#include "capture_cursor_policy.h"
#include "config_value.h"
#include "kde_capture_config.h"
#include "recording/recording_storage_config.h"
#include "save_path_config.h"
#include "startup_config.h"
#include "toolbar_appearance_config.h"
#include "ui/i18n.h"
#include "ui/interface_language_config.h"
#include "ui/theme.h"
#include "ui/interface_theme_config.h"
#include "window_detection.h"
#include "windows_tray_controller.h"

#include <QJsonObject>
#include <QJsonValue>

#include <algorithm>
#include <optional>

namespace markshot::settings {
namespace {

constexpr int kMinTimeoutMs = 1000;
constexpr int kMaxCommandTimeoutMs = 300000;
constexpr int kMaxWindowDetectionTimeoutMs = 30000;

/// @brief 从对象中读取第一个可解析的布尔值。
/// @param object JSON 对象。
/// @param keys 候选字段。
/// @return 解析成功时返回布尔值。
std::optional<bool> boolForKeys(const QJsonObject &object, const QStringList &keys)
{
    return config::boolValue(config::valueForKeys(object, keys));
}

/// @brief 从对象中读取第一个字符串值。
/// @param object JSON 对象。
/// @param keys 候选字段。
/// @return 读取到的字符串。
QString stringForKeys(const QJsonObject &object, const QStringList &keys)
{
    const QJsonValue value = config::valueForKeys(object, keys);
    return value.isString() ? value.toString().trimmed() : QString();
}

/// @brief 从对象中读取带范围限制的整数值。
/// @param object JSON 对象。
/// @param keys 候选字段。
/// @param fallback 默认值。
/// @param minimum 最小值。
/// @param maximum 最大值。
/// @return 读取到的整数值。
int intForKeys(const QJsonObject &object, const QStringList &keys, int fallback, int minimum, int maximum)
{
    if (const std::optional<int> value = config::intValue(config::valueForKeys(object, keys))) {
        return std::clamp(*value, minimum, maximum);
    }
    return fallback;
}

/// @brief 从 JSON 值读取小数。
/// @param value JSON 值。
/// @return 解析成功的小数。
std::optional<double> doubleValue(const QJsonValue &value)
{
    if (value.isDouble()) {
        return value.toDouble();
    }
    if (value.isString()) {
        bool ok = false;
        const double number = value.toString().trimmed().toDouble(&ok);
        if (ok) {
            return number;
        }
    }
    return std::nullopt;
}

/// @brief 从对象中读取带范围限制的小数值。
/// @param object JSON 对象。
/// @param keys 候选字段。
/// @param fallback 默认值。
/// @param minimum 最小值。
/// @param maximum 最大值。
/// @return 读取到的小数值。
double doubleForKeys(const QJsonObject &object,
                     const QStringList &keys,
                     double fallback,
                     double minimum,
                     double maximum)
{
    if (const std::optional<double> value = doubleValue(config::valueForKeys(object, keys))) {
        return std::clamp(*value, minimum, maximum);
    }
    return fallback;
}

/// @brief 从 JSON 对象读取环境变量映射。
/// @param object JSON 对象。
/// @return 环境变量映射。
QMap<QString, QString> envMapFromObject(const QJsonObject &object)
{
    QMap<QString, QString> values;
    for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
        const QString key = it.key().trimmed();
        if (key.isEmpty()) {
            continue;
        }
        if (const std::optional<QString> value = config::environmentStringValue(it.value())) {
            values.insert(key, *value);
        }
    }
    return values;
}

/// @brief 从置顶图片边框对象读取边框设置。
/// @param border JSON 边框配置。
/// @param settings 需要更新的置顶图片设置。
void applyPinnedBorderObject(const QJsonObject &border, PinnedSettings *settings)
{
    if (!settings) {
        return;
    }
    if (const std::optional<bool> enabled =
            boolForKeys(border,
                        {QStringLiteral("enabled"),
                         QStringLiteral("visible"),
                         QStringLiteral("show")})) {
        settings->borderEnabled = *enabled;
    }
    if (const std::optional<QColor> color =
            colorFromString(stringForKeys(border,
                                         {QStringLiteral("color"),
                                          QStringLiteral("borderColor"),
                                          QStringLiteral("value")}))) {
        settings->borderColor = *color;
    }
    settings->borderWidth =
        doubleForKeys(border,
                      {QStringLiteral("width"),
                       QStringLiteral("borderWidth"),
                       QStringLiteral("size")},
                      settings->borderWidth,
                      1.0,
                      12.0);
}

/// @brief 从配置读取置顶图片设置。
/// @param root 应用配置根对象。
/// @return 置顶图片设置。
PinnedSettings readPinnedSettings(const QJsonObject &root)
{
    PinnedSettings settings;
    const QJsonObject pinned =
        config::firstObjectValue(root,
                                 {QStringLiteral("pinnedWindow"),
                                  QStringLiteral("pinned"),
                                  QStringLiteral("pin")});
    const QJsonObject ocr = config::firstObjectValue(root, QStringLiteral("ocr"));
    const QJsonObject translation = config::firstObjectValue(root, QStringLiteral("translation"));

    if (const std::optional<bool> alwaysOnTop =
            boolForKeys(pinned,
                        {QStringLiteral("alwaysOnTop"),
                         QStringLiteral("stayOnTop"),
                         QStringLiteral("topmost"),
                         QStringLiteral("above")})) {
        settings.alwaysOnTop = *alwaysOnTop;
    }
    if (const std::optional<bool> textSelectionCopy =
            boolForKeys(pinned,
                        {QStringLiteral("textSelectionCopyEnabled"),
                         QStringLiteral("textSelectionEnabled"),
                         QStringLiteral("allowTextSelection"),
                         QStringLiteral("allowTextSelectionCopy"),
                         QStringLiteral("selectableText"),
                         QStringLiteral("selectableOcrText"),
                         QStringLiteral("selectableOCRText")})) {
        settings.textSelectionCopyEnabled = *textSelectionCopy;
    }
    if (const std::optional<bool> autoOcr =
            boolForKeys(pinned,
                        {QStringLiteral("autoOcr"),
                         QStringLiteral("autoOCR"),
                         QStringLiteral("autoRecognizeText"),
                         QStringLiteral("ocrOnPin")})) {
        settings.autoOcr = *autoOcr;
    }

    if (const std::optional<bool> ocrEnabled =
            boolForKeys(ocr, {QStringLiteral("enabled"), QStringLiteral("enable")})) {
        settings.ocrEnabled = *ocrEnabled;
    }
    const QString ocrBackend = ocr.value(QStringLiteral("backend")).toString().trimmed();
    if (!ocrBackend.isEmpty()) {
        settings.ocrBackend = ocrBackend;
    }
    settings.ocrCommand = ocr.value(QStringLiteral("command")).toString().trimmed();
    settings.ocrTimeoutMs =
        intForKeys(ocr, {QStringLiteral("timeoutMs")}, settings.ocrTimeoutMs, kMinTimeoutMs, kMaxCommandTimeoutMs);

    if (const std::optional<bool> autoTranslate =
            boolForKeys(translation,
                        {QStringLiteral("autoAfterOcr"),
                         QStringLiteral("autoAfterOCR"),
                         QStringLiteral("autoTranslateAfterOcr"),
                         QStringLiteral("auto")})) {
        settings.autoTranslateAfterOcr = *autoTranslate;
    }
    const QString targetLanguage = translation.value(QStringLiteral("targetLanguage")).toString().trimmed();
    if (!targetLanguage.isEmpty()) {
        settings.translationTargetLanguage = targetLanguage;
    }
    settings.translationCommand = translation.value(QStringLiteral("command")).toString().trimmed();
    settings.translationTimeoutMs =
        intForKeys(translation,
                   {QStringLiteral("timeoutMs")},
                   settings.translationTimeoutMs,
                   kMinTimeoutMs,
                   kMaxCommandTimeoutMs);

    const QJsonValue border = pinned.value(QStringLiteral("border"));
    if (border.isBool()) {
        settings.borderEnabled = border.toBool();
    } else if (border.isObject()) {
        settings.borderEnabled = true;
        applyPinnedBorderObject(border.toObject(), &settings);
    }
    if (const std::optional<bool> borderEnabled =
            boolForKeys(pinned, {QStringLiteral("borderEnabled"), QStringLiteral("showBorder")})) {
        settings.borderEnabled = *borderEnabled;
    }
    if (const std::optional<QColor> borderColor =
            colorFromString(pinned.value(QStringLiteral("borderColor")).toString())) {
        settings.borderColor = *borderColor;
    } else if (!settings.borderColor.isValid()) {
        settings.borderColor = markshot::theme::kAccent;
    }
    settings.borderWidth =
        doubleForKeys(pinned,
                      {QStringLiteral("borderWidth"), QStringLiteral("borderSize")},
                      settings.borderWidth,
                      1.0,
                      12.0);
    return settings;
}

/// @brief 从配置读取标注默认值设置。
/// @param root 应用配置根对象。
/// @return 标注设置。
AnnotationSettings readAnnotationSettings(const QJsonObject &root)
{
    AnnotationSettings settings;

    QString warning;
    const DefaultTools tools = configuredDefaultTools(&warning);
    settings.normalTool = tools.normal;
    settings.fullscreenTool = tools.fullscreen;
    settings.fileTool = tools.file;
    settings.defaultColor = tools.color.isValid() ? tools.color : markshot::theme::kDefaultAnnotationColor;

    const ToolbarAppearanceConfig toolbar = toolbarAppearanceFromConfigRoot(root);
    settings.toolbarIconSize = toolbar.toolbarIconSize;
    settings.toolbarFontSize = toolbar.fontSize;
    return settings;
}

/// @brief 从配置读取保存和剪贴板设置。
/// @param root 应用配置根对象。
/// @return 保存和剪贴板设置。
StorageSettings readStorageSettings(const QJsonObject &root)
{
    StorageSettings settings;
    settings.savePathTemplate = savePathTemplateFromConfigRoot(root);
    if (settings.savePathTemplate.isEmpty()) {
        settings.savePathTemplate = defaultSavePathTemplate();
    }

    const recording::RecordingStorageConfig recordingStorage = recording::recordingStorageConfigFromRoot(root);
    settings.recordingVideoDirectory = recordingStorage.videoDirectory;
    settings.recordingGifDirectory = recordingStorage.gifDirectory;

    const ClipboardImageConfig clipboard = clipboardImageConfigFromRoot(root);
    settings.clipboardImageMode = clipboard.mode;
    settings.clipboardThresholdM = clipboard.thresholdM;
    settings.exportImageEffect = exportImageEffectConfigFromRoot(root);
    return settings;
}

/// @brief 从配置读取快捷键设置。
/// @return 快捷键设置。
ShortcutSettings readShortcutSettings()
{
    const shortcut::ShortcutConfig shortcuts = shortcut::configuredShortcuts(appConfigPath());
    ShortcutSettings settings;
    settings.actions = shortcuts.actions;
    settings.tools = shortcuts.tools;
    settings.startupColorPicker = shortcuts.startupColorPicker;
    settings.startupRuler = shortcuts.startupRuler;
    settings.startupCodeScanner = shortcuts.startupCodeScanner;
    settings.startupDisplayCapture = shortcuts.startupDisplayCapture;
    settings.startupGifRecorder = shortcuts.startupGifRecorder;
    settings.startupVideoRecorder = shortcuts.startupVideoRecorder;
    return settings;
}

/// @brief 从配置读取滚动截图设置。
/// @param root 应用配置根对象。
/// @return 滚动截图设置。
ScrollSettings readScrollSettings(const QJsonObject &root)
{
    ScrollSettings settings;
    const QJsonObject scroll =
        config::firstObjectValue(root,
                                 {QStringLiteral("scrollCapture"),
                                  QStringLiteral("scrollingCapture"),
                                  QStringLiteral("scroll")});
    if (scroll.isEmpty()) {
        return settings;
    }

    const QJsonValue frame =
        config::valueForKeys(scroll,
                             {QStringLiteral("frame"),
                              QStringLiteral("captureFrame"),
                              QStringLiteral("border"),
                              QStringLiteral("outline")});
    if (frame.isBool()) {
        settings.frameEnabled = frame.toBool();
    } else if (frame.isDouble()) {
        settings.frameEnabled = true;
        settings.frameGap = std::clamp(frame.toInt(settings.frameGap), 0, 256);
    } else if (frame.isObject()) {
        const QJsonObject frameObject = frame.toObject();
        if (const std::optional<bool> enabled =
                boolForKeys(frameObject,
                            {QStringLiteral("enabled"),
                             QStringLiteral("visible"),
                             QStringLiteral("show"),
                             QStringLiteral("showFrame"),
                             QStringLiteral("showBorder")})) {
            settings.frameEnabled = *enabled;
        }
        settings.frameGap =
            intForKeys(frameObject,
                       {QStringLiteral("gap"),
                        QStringLiteral("distance"),
                        QStringLiteral("offset"),
                        QStringLiteral("padding"),
                        QStringLiteral("margin"),
                        QStringLiteral("value"),
                        QStringLiteral("px")},
                       settings.frameGap,
                       0,
                       256);
    }

    if (const std::optional<bool> frameEnabled =
            boolForKeys(scroll,
                        {QStringLiteral("frameEnabled"),
                         QStringLiteral("borderEnabled"),
                         QStringLiteral("showFrame"),
                         QStringLiteral("showBorder")})) {
        settings.frameEnabled = *frameEnabled;
    }
    settings.frameGap =
        intForKeys(scroll,
                   {QStringLiteral("frameGap"),
                    QStringLiteral("frameDistance"),
                    QStringLiteral("frameOffset"),
                    QStringLiteral("borderGap"),
                    QStringLiteral("borderDistance"),
                    QStringLiteral("borderOffset")},
                   settings.frameGap,
                   0,
                   256);
    settings.previewGap =
        intForKeys(scroll,
                   {QStringLiteral("previewGap"),
                    QStringLiteral("previewDistance"),
                    QStringLiteral("previewOffset"),
                    QStringLiteral("panelGap"),
                    QStringLiteral("panelDistance"),
                    QStringLiteral("panelOffset")},
                   settings.previewGap,
                   0,
                   512);
    if (const std::optional<bool> hidePreview =
            boolForKeys(scroll,
                        {QStringLiteral("hidePreviewDuringCapture"),
                         QStringLiteral("hidePreviewWhileCapturing"),
                         QStringLiteral("hidePanelDuringCapture"),
                         QStringLiteral("alwaysHidePreview"),
                         QStringLiteral("forceHidePreview")})) {
        settings.hidePreviewDuringCapture = *hidePreview;
    }
    return settings;
}

/// @brief 从配置读取集成设置。
/// @param root 应用配置根对象。
/// @return 集成设置。
IntegrationSettings readIntegrationSettings(const QJsonObject &root)
{
    IntegrationSettings settings;
    const QJsonObject codeScan =
        config::firstObjectValue(root,
                                 {QStringLiteral("codeScan"),
                                  QStringLiteral("codeScanner"),
                                  QStringLiteral("barcodeScanner"),
                                  QStringLiteral("barcode")});
    settings.codeScanCommand =
        stringForKeys(codeScan,
                      {QStringLiteral("command"), QStringLiteral("path"), QStringLiteral("helper")});
    settings.codeScanTimeoutMs =
        intForKeys(codeScan,
                   {QStringLiteral("timeoutMs")},
                   settings.codeScanTimeoutMs,
                   kMinTimeoutMs,
                   kMaxCommandTimeoutMs);

    const QJsonObject upload =
        config::firstObjectValue(root,
                                 {QStringLiteral("upload"),
                                  QStringLiteral("imageUpload"),
                                  QStringLiteral("uploader"),
                                  QStringLiteral("imageHost")});
    settings.uploadCommand =
        stringForKeys(upload,
                      {QStringLiteral("command"), QStringLiteral("path"), QStringLiteral("helper")});
    settings.uploadTimeoutMs =
        intForKeys(upload,
                   {QStringLiteral("timeoutMs")},
                   settings.uploadTimeoutMs,
                   kMinTimeoutMs,
                   kMaxCommandTimeoutMs);
    settings.uploadEnv =
        envMapFromObject(config::firstObjectValue(upload,
                                                  {QStringLiteral("env"),
                                                   QStringLiteral("environment"),
                                                   QStringLiteral("envVars"),
                                                   QStringLiteral("variables")}));

    const QJsonObject ocr = config::firstObjectValue(root, QStringLiteral("ocr"));
    if (const std::optional<bool> resultPanel =
            boolForKeys(ocr,
                        {QStringLiteral("resultPanel"),
                         QStringLiteral("resultWindow"),
                         QStringLiteral("resultPanelEnabled"),
                         QStringLiteral("resultWindowEnabled")})) {
        settings.ocrResultPanelEnabled = *resultPanel;
    }

    const QJsonObject translation = config::firstObjectValue(root, QStringLiteral("translation"));
    const QString apiBase = translation.value(QStringLiteral("apiBase")).toString().trimmed();
    if (!apiBase.isEmpty()) {
        settings.translationApiBase = apiBase;
    }
    const QString apiKeyEnv = translation.value(QStringLiteral("apiKeyEnv")).toString().trimmed();
    if (!apiKeyEnv.isEmpty()) {
        settings.translationApiKeyEnv = apiKeyEnv;
    }
    settings.translationApiKey = translation.value(QStringLiteral("apiKey")).toString().trimmed();
    const QString model = translation.value(QStringLiteral("model")).toString().trimmed();
    if (!model.isEmpty()) {
        settings.translationModel = model;
    }
    settings.translationTemperature =
        doubleForKeys(translation,
                      {QStringLiteral("temperature")},
                      settings.translationTemperature,
                      0.0,
                      2.0);
    settings.translationSystemPrompt = translation.value(QStringLiteral("systemPrompt")).toString();
    return settings;
}

/// @brief 从配置读取高级设置。
/// @param root 应用配置根对象。
/// @return 高级设置。
AdvancedSettings readAdvancedSettings(const QJsonObject &root)
{
    AdvancedSettings settings;
    const DebugRuntimeConfig debug = configuredDebugRuntimeConfig();
    settings.debugEnabled = debug.enabled;
    settings.debugLogPath = debug.logPath;

    const QJsonObject windowDetection = config::objectValue(root, QStringLiteral("windowDetection"));
    if (const std::optional<bool> enabled = config::boolValue(root.value(QStringLiteral("windowDetection")))) {
        settings.windowDetectionEnabled = *enabled;
    } else if (const std::optional<bool> enabledObject =
                   config::boolValue(windowDetection.value(QStringLiteral("enabled")))) {
        settings.windowDetectionEnabled = *enabledObject;
    }
    settings.windowDetectionCommand = windowDetection.value(QStringLiteral("command")).toString().trimmed();
    settings.windowDetectionWorkingDirectory =
        stringForKeys(windowDetection, {QStringLiteral("workingDirectory"), QStringLiteral("cwd")});
    settings.windowDetectionTimeoutMs =
        intForKeys(windowDetection,
                   {QStringLiteral("timeoutMs")},
                   settings.windowDetectionTimeoutMs,
                   100,
                   kMaxWindowDetectionTimeoutMs);
    settings.windowDetectionEnv =
        envMapFromObject(config::firstObjectValue(windowDetection,
                                                  {QStringLiteral("env"),
                                                   QStringLiteral("environment"),
                                                   QStringLiteral("envVars"),
                                                   QStringLiteral("variables")}));

    QMap<QString, QString> appEnv = envMapFromObject(config::objectValue(root, QStringLiteral("env")));
    const QMap<QString, QString> namedEnv = envMapFromObject(config::objectValue(root, QStringLiteral("environment")));
    for (auto it = namedEnv.constBegin(); it != namedEnv.constEnd(); ++it) {
        appEnv.insert(it.key(), it.value());
    }
    settings.appEnv = appEnv;
    return settings;
}

}  // namespace

SettingsConfig readSettingsConfig(QString *error)
{
    if (error) {
        error->clear();
    }

    bool ok = false;
    const QJsonObject root = readAppConfigRoot(&ok);
    if (!ok && error) {
        *error = MS_TR("Cannot read application config");
    }

    SettingsConfig settings;
    settings.general.uiLanguageMode = markshot::ui::uiLanguageModeFromConfigRoot(root);
    settings.general.uiThemeMode = markshot::ui::uiThemeModeFromConfigRoot(root);
    const WindowsTrayController::Config tray = WindowsTrayController::readConfig();
    settings.general.trayEnabled = tray.autoStart;
    settings.general.launchOnStartup = autostart::isEnabled();
    settings.general.hotkeysEnabled = tray.hotkeysEnabled;
    settings.general.captureHotkey = tray.captureHotkey;
    settings.general.fullscreenHotkey = tray.fullscreenHotkey;

    settings.capture.includeCursor = captureIncludeCursorFromConfigRoot(root);
    settings.capture.freezeScope = captureFreezeScopeFromConfigRoot(root);
    settings.capture.kdeKwinScreenshotEnabled = kdeKWinScreenshotEnabledFromConfigRoot(root);
    settings.shortcuts = readShortcutSettings();
    settings.annotation = readAnnotationSettings(root);
    settings.pinned = readPinnedSettings(root);
    settings.storage = readStorageSettings(root);
    settings.scroll = readScrollSettings(root);
    settings.integrations = readIntegrationSettings(root);
    settings.advanced = readAdvancedSettings(root);
    return settings;
}

}  // namespace markshot::settings
