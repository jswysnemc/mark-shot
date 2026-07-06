#include "settings/settings_config.h"

#include "annotation_state_store.h"
#include "app_config_store.h"
#include "autostart/autostart_manager.h"
#include "capture_cursor_policy.h"
#include "config_value.h"
#include "recording/recording_storage_config.h"
#include "settings/provider_preference_config.h"
#include "ui/i18n.h"
#include "ui/interface_language_config.h"
#include "ui/interface_theme_config.h"

#include <QJsonObject>
#include <QJsonValue>

#include <algorithm>
#include <array>

namespace markshot::settings {
namespace {

constexpr int kMinToolbarIconSize = 12;
constexpr int kMaxToolbarIconSize = 48;
constexpr int kMinToolbarFontSize = 8;
constexpr int kMaxToolbarFontSize = 24;
constexpr int kMinClipboardThresholdM = 1;
constexpr int kMaxClipboardThresholdM = 1024;
constexpr int kMinTimeoutMs = 1000;
constexpr int kMaxCommandTimeoutMs = 300000;
constexpr int kMinWindowDetectionTimeoutMs = 100;
constexpr int kMaxWindowDetectionTimeoutMs = 30000;

/// @brief 将 QKeySequence 转换为可写入配置的文本。
/// @param sequence 快捷键序列。
/// @return PortableText 格式快捷键文本。
QString sequenceText(const QKeySequence &sequence)
{
    return sequence.isEmpty() ? QString() : sequence.toString(QKeySequence::PortableText);
}

/// @brief 将环境变量映射转换为 JSON 对象。
/// @param values 环境变量映射。
/// @return JSON 对象。
QJsonObject envObjectFromMap(const QMap<QString, QString> &values)
{
    QJsonObject object;
    for (auto it = values.constBegin(); it != values.constEnd(); ++it) {
        const QString key = it.key().trimmed();
        if (!key.isEmpty()) {
            object.insert(key, it.value());
        }
    }
    return object;
}

/// @brief 设置根对象中的嵌套对象路径。
/// @param object 需要更新的 JSON 对象。
/// @param path 嵌套字段路径。
/// @param value 要写入的 JSON 值。
void setNestedValue(QJsonObject *object, const QStringList &path, const QJsonValue &value)
{
    if (!object || path.isEmpty()) {
        return;
    }

    const QString key = path.first();
    if (path.size() == 1) {
        object->insert(key, value);
        return;
    }

    QJsonObject child = object->value(key).isObject() ? object->value(key).toObject() : QJsonObject();
    setNestedValue(&child, path.mid(1), value);
    object->insert(key, child);
}

/// @brief 返回设置界面支持编辑的标注工具列表。
/// @return 标注工具数组。
std::array<ShotWindow::Tool, static_cast<int>(ShotWindow::Tool::Laser) + 1> configurableTools()
{
    return {ShotWindow::Tool::Move,
            ShotWindow::Tool::Select,
            ShotWindow::Tool::Pen,
            ShotWindow::Tool::Line,
            ShotWindow::Tool::Highlighter,
            ShotWindow::Tool::Rectangle,
            ShotWindow::Tool::Ellipse,
            ShotWindow::Tool::Arrow,
            ShotWindow::Tool::Text,
            ShotWindow::Tool::Number,
            ShotWindow::Tool::Mosaic,
            ShotWindow::Tool::Magnifier,
            ShotWindow::Tool::Laser};
}

/// @brief 返回设置界面支持编辑的动作列表。
/// @return 标注动作数组。
std::array<ShotWindow::Action, 15> configurableActions()
{
    return {ShotWindow::Action::ToggleCaptureScope,
            ShotWindow::Action::ToggleToolbarLayout,
            ShotWindow::Action::Clear,
            ShotWindow::Action::Undo,
            ShotWindow::Action::Redo,
            ShotWindow::Action::OpenWith,
            ShotWindow::Action::Extensions,
            ShotWindow::Action::Pin,
            ShotWindow::Action::ScrollCapture,
            ShotWindow::Action::OcrCopy,
            ShotWindow::Action::Copy,
            ShotWindow::Action::Save,
            ShotWindow::Action::Upload,
            ShotWindow::Action::Settings,
            ShotWindow::Action::Cancel};
}

/// @brief 写入通用与全局快捷键设置。
/// @param root 应用配置根对象。
/// @param settings 通用设置。
void writeGeneralSettings(QJsonObject *root, const GeneralSettings &settings)
{
    setNestedValue(root, {QStringLiteral("ui"), QStringLiteral("language")},
                   markshot::ui::uiLanguageModeName(settings.uiLanguageMode));
    setNestedValue(root, {QStringLiteral("ui"), QStringLiteral("theme")},
                   markshot::ui::uiThemeModeName(settings.uiThemeMode));
    setNestedValue(root, {QStringLiteral("windows"), QStringLiteral("tray"), QStringLiteral("enabled")},
                   settings.trayEnabled);
    setNestedValue(root, {QStringLiteral("windows"), QStringLiteral("tray"), QStringLiteral("autoStart")},
                   settings.trayEnabled);
    setNestedValue(root, {QStringLiteral("windows"), QStringLiteral("hotkeys"), QStringLiteral("enabled")},
                   settings.hotkeysEnabled);
    setNestedValue(root, {QStringLiteral("windows"), QStringLiteral("hotkeys"), QStringLiteral("capture")},
                   sequenceText(settings.captureHotkey));
    setNestedValue(root, {QStringLiteral("windows"), QStringLiteral("hotkeys"), QStringLiteral("fullscreen")},
                   sequenceText(settings.fullscreenHotkey));
    setNestedValue(root, {QStringLiteral("globalHotkeys"), QStringLiteral("enabled")},
                   settings.hotkeysEnabled);
    setNestedValue(root, {QStringLiteral("globalHotkeys"), QStringLiteral("capture")},
                   sequenceText(settings.captureHotkey));
    setNestedValue(root, {QStringLiteral("globalHotkeys"), QStringLiteral("fullscreen")},
                   sequenceText(settings.fullscreenHotkey));
    setNestedValue(root, {QStringLiteral("startup"), QStringLiteral("launchOnStartup")},
                   settings.launchOnStartup);
}

/// @brief 写入截图行为设置。
/// @param root 应用配置根对象。
/// @param settings 截图设置。
void writeCaptureSettings(QJsonObject *root, const CaptureSettings &settings)
{
    setNestedValue(root, {QStringLiteral("capture"), QStringLiteral("includeCursor")},
                   settings.includeCursor);
    setNestedValue(root, {QStringLiteral("capture"), QStringLiteral("freezeScope")},
                   captureFreezeScopeName(settings.freezeScope));
    setNestedValue(root,
                   {QStringLiteral("capture"),
                    QStringLiteral("wayland"),
                    QStringLiteral("kde"),
                    QStringLiteral("kwinScreenshot"),
                    QStringLiteral("enabled")},
                   settings.kdeKwinScreenshotEnabled);
    setNestedValue(root, {QStringLiteral("capture"), QStringLiteral("hideOwnWindows")},
                   settings.hideOwnWindows);
}

/// @brief 写入快捷键设置。
/// @param root 应用配置根对象。
/// @param settings 快捷键设置。
void writeShortcutSettings(QJsonObject *root, const ShortcutSettings &settings)
{
    QJsonObject tools;
    for (ShotWindow::Tool tool : configurableTools()) {
        tools.insert(toolConfigName(tool), sequenceText(settings.tools.at(shortcut::toolIndex(tool))));
    }

    QJsonObject actions;
    for (ShotWindow::Action action : configurableActions()) {
        actions.insert(actionConfigName(action), sequenceText(settings.actions.at(shortcut::actionIndex(action))));
    }

    QJsonObject startup;
    startup.insert(QStringLiteral("colorPicker"), sequenceText(settings.startupColorPicker));
    startup.insert(QStringLiteral("ruler"), sequenceText(settings.startupRuler));
    startup.insert(QStringLiteral("codeScanner"), sequenceText(settings.startupCodeScanner));
    startup.insert(QStringLiteral("displayCapture"), sequenceText(settings.startupDisplayCapture));
    startup.insert(QStringLiteral("gifRecorder"), sequenceText(settings.startupGifRecorder));
    startup.insert(QStringLiteral("videoRecorder"), sequenceText(settings.startupVideoRecorder));

    setNestedValue(root, {QStringLiteral("shortcuts"), QStringLiteral("tools")}, tools);
    setNestedValue(root, {QStringLiteral("shortcuts"), QStringLiteral("actions")}, actions);
    setNestedValue(root, {QStringLiteral("shortcuts"), QStringLiteral("startup")}, startup);
}

/// @brief 写入标注默认值设置。
/// @param root 应用配置根对象。
/// @param settings 标注设置。
void writeAnnotationSettings(QJsonObject *root, const AnnotationSettings &settings)
{
    setNestedValue(root, {QStringLiteral("annotation"), QStringLiteral("defaultTool")},
                   toolConfigName(settings.normalTool));
    setNestedValue(root, {QStringLiteral("annotation"), QStringLiteral("fullscreenDefaultTool")},
                   toolConfigName(settings.fullscreenTool));
    setNestedValue(root, {QStringLiteral("annotation"), QStringLiteral("fileDefaultTool")},
                   toolConfigName(settings.fileTool));
    setNestedValue(root, {QStringLiteral("annotation"), QStringLiteral("defaultColor")},
                   settings.defaultColor.name(QColor::HexRgb).toUpper());
    setNestedValue(root, {QStringLiteral("toolbar"), QStringLiteral("iconSize")},
                   std::clamp(settings.toolbarIconSize, kMinToolbarIconSize, kMaxToolbarIconSize));
    setNestedValue(root, {QStringLiteral("toolbar"), QStringLiteral("fontSize")},
                   std::clamp(settings.toolbarFontSize, kMinToolbarFontSize, kMaxToolbarFontSize));
}

/// @brief 写入置顶图片设置。
/// @param root 应用配置根对象。
/// @param settings 置顶图片设置。
void writePinnedSettings(QJsonObject *root, const PinnedSettings &settings)
{
    setNestedValue(root, {QStringLiteral("pinnedWindow"), QStringLiteral("alwaysOnTop")},
                   settings.alwaysOnTop);
    setNestedValue(root, {QStringLiteral("pinnedWindow"), QStringLiteral("textSelectionCopyEnabled")},
                   settings.textSelectionCopyEnabled);
    setNestedValue(root, {QStringLiteral("pinnedWindow"), QStringLiteral("autoOcr")},
                   settings.autoOcr);
    setNestedValue(root, {QStringLiteral("pinnedWindow"), QStringLiteral("borderEnabled")},
                   settings.borderEnabled);
    setNestedValue(root, {QStringLiteral("pinnedWindow"), QStringLiteral("borderColor")},
                   settings.borderColor.name(QColor::HexRgb).toUpper());
    setNestedValue(root, {QStringLiteral("pinnedWindow"), QStringLiteral("borderWidth")},
                   std::clamp(settings.borderWidth, 1.0, 12.0));
    setNestedValue(root, {QStringLiteral("ocr"), QStringLiteral("enabled")}, settings.ocrEnabled);
    setNestedValue(root, {QStringLiteral("ocr"), QStringLiteral("backend")}, settings.ocrBackend.trimmed());
    setNestedValue(root, {QStringLiteral("ocr"), QStringLiteral("command")}, settings.ocrCommand.trimmed());
    setNestedValue(root, {QStringLiteral("ocr"), QStringLiteral("timeoutMs")},
                   std::clamp(settings.ocrTimeoutMs, kMinTimeoutMs, kMaxCommandTimeoutMs));
    setNestedValue(root, {QStringLiteral("translation"), QStringLiteral("autoAfterOcr")},
                   settings.autoTranslateAfterOcr);
    setNestedValue(root, {QStringLiteral("translation"), QStringLiteral("targetLanguage")},
                   settings.translationTargetLanguage.trimmed());
    setNestedValue(root, {QStringLiteral("translation"), QStringLiteral("command")},
                   settings.translationCommand.trimmed());
    setNestedValue(root, {QStringLiteral("translation"), QStringLiteral("timeoutMs")},
                   std::clamp(settings.translationTimeoutMs, kMinTimeoutMs, kMaxCommandTimeoutMs));
}

/// @brief 写入保存与剪贴板设置。
/// @param root 应用配置根对象。
/// @param settings 保存与剪贴板设置。
void writeStorageSettings(QJsonObject *root, const StorageSettings &settings)
{
    setNestedValue(root, {QStringLiteral("save"), QStringLiteral("pathTemplate")},
                   settings.savePathTemplate.trimmed());
    const recording::RecordingStorageConfig defaults = recording::defaultRecordingStorageConfig();
    setNestedValue(root,
                   {QStringLiteral("recording"), QStringLiteral("storage"), QStringLiteral("videoDirectory")},
                   recording::normalizedRecordingDirectory(settings.recordingVideoDirectory,
                                                           defaults.videoDirectory));
    setNestedValue(root,
                   {QStringLiteral("recording"), QStringLiteral("storage"), QStringLiteral("gifDirectory")},
                   recording::normalizedRecordingDirectory(settings.recordingGifDirectory,
                                                           defaults.gifDirectory));
    setNestedValue(root, {QStringLiteral("clipboard"), QStringLiteral("image"), QStringLiteral("mode")},
                   clipboardImageModeName(settings.clipboardImageMode));
    setNestedValue(root, {QStringLiteral("clipboard"), QStringLiteral("image"), QStringLiteral("thresholdM")},
                   std::clamp(settings.clipboardThresholdM, kMinClipboardThresholdM, kMaxClipboardThresholdM));
    setNestedValue(root, {QStringLiteral("export"), QStringLiteral("imageFrame"), QStringLiteral("enabled")},
                   settings.exportImageEffect.enabled);
    setNestedValue(root, {QStringLiteral("export"), QStringLiteral("imageFrame"), QStringLiteral("padding")},
                   std::clamp(settings.exportImageEffect.padding, 0, 256));
    setNestedValue(root, {QStringLiteral("export"), QStringLiteral("imageFrame"), QStringLiteral("cornerRadius")},
                   std::clamp(settings.exportImageEffect.cornerRadius, 0.0, 128.0));
    setNestedValue(root, {QStringLiteral("export"), QStringLiteral("imageFrame"), QStringLiteral("shadowRadius")},
                   std::clamp(settings.exportImageEffect.shadowRadius, 0, 128));
    setNestedValue(root, {QStringLiteral("export"), QStringLiteral("imageFrame"), QStringLiteral("shadowOffsetY")},
                   std::clamp(settings.exportImageEffect.shadowOffsetY, 0, 128));
    setNestedValue(root, {QStringLiteral("export"), QStringLiteral("imageFrame"), QStringLiteral("shadowOpacity")},
                   std::clamp(settings.exportImageEffect.shadowOpacity, 0.0, 1.0));
}

/// @brief 写入滚动截图设置。
/// @param root 应用配置根对象。
/// @param settings 滚动截图设置。
void writeScrollSettings(QJsonObject *root, const ScrollSettings &settings)
{
    setNestedValue(root, {QStringLiteral("scrollCapture"), QStringLiteral("frameEnabled")},
                   settings.frameEnabled);
    setNestedValue(root, {QStringLiteral("scrollCapture"), QStringLiteral("frameGap")},
                   std::clamp(settings.frameGap, 0, 256));
    setNestedValue(root, {QStringLiteral("scrollCapture"), QStringLiteral("previewGap")},
                   std::clamp(settings.previewGap, 0, 512));
    setNestedValue(root, {QStringLiteral("scrollCapture"), QStringLiteral("hidePreviewDuringCapture")},
                   settings.hidePreviewDuringCapture);
}

/// @brief 写入外部集成设置。
/// @param root 应用配置根对象。
/// @param settings 外部集成设置。
void writeIntegrationSettings(QJsonObject *root, const IntegrationSettings &settings)
{
    setNestedValue(root, {QStringLiteral("codeScan"), QStringLiteral("command")},
                   settings.codeScanCommand.trimmed());
    setNestedValue(root, {QStringLiteral("codeScan"), QStringLiteral("timeoutMs")},
                   std::clamp(settings.codeScanTimeoutMs, kMinTimeoutMs, kMaxCommandTimeoutMs));
    setNestedValue(root, {QStringLiteral("upload"), QStringLiteral("command")},
                   settings.uploadCommand.trimmed());
    setNestedValue(root, {QStringLiteral("upload"), QStringLiteral("timeoutMs")},
                   std::clamp(settings.uploadTimeoutMs, kMinTimeoutMs, kMaxCommandTimeoutMs));
    setNestedValue(root, {QStringLiteral("upload"), QStringLiteral("env")},
                   envObjectFromMap(settings.uploadEnv));
    setNestedValue(root, {QStringLiteral("ocr"), QStringLiteral("resultPanel")},
                   settings.ocrResultPanelEnabled);
    setNestedValue(root, {QStringLiteral("translation"), QStringLiteral("apiBase")},
                   settings.translationApiBase.trimmed());
    setNestedValue(root, {QStringLiteral("translation"), QStringLiteral("apiKeyEnv")},
                   settings.translationApiKeyEnv.trimmed());
    setNestedValue(root, {QStringLiteral("translation"), QStringLiteral("apiKey")},
                   settings.translationApiKey.trimmed());
    setNestedValue(root, {QStringLiteral("translation"), QStringLiteral("model")},
                   settings.translationModel.trimmed());
    setNestedValue(root, {QStringLiteral("translation"), QStringLiteral("temperature")},
                   std::clamp(settings.translationTemperature, 0.0, 2.0));
    setNestedValue(root, {QStringLiteral("translation"), QStringLiteral("systemPrompt")},
                   settings.translationSystemPrompt);
}

/// @brief 写入高级运行时设置。
/// @param root 应用配置根对象。
/// @param settings 高级设置。
void writeAdvancedSettings(QJsonObject *root, const AdvancedSettings &settings)
{
    setNestedValue(root, {QStringLiteral("debug"), QStringLiteral("enabled")},
                   settings.debugEnabled);
    setNestedValue(root, {QStringLiteral("debug"), QStringLiteral("logPath")},
                   settings.debugLogPath.trimmed());
    setNestedValue(root, {QStringLiteral("windowDetection"), QStringLiteral("enabled")},
                   settings.windowDetectionEnabled);
    setNestedValue(root, {QStringLiteral("windowDetection"), QStringLiteral("command")},
                   settings.windowDetectionCommand.trimmed());
    setNestedValue(root, {QStringLiteral("windowDetection"), QStringLiteral("workingDirectory")},
                   settings.windowDetectionWorkingDirectory.trimmed());
    setNestedValue(root, {QStringLiteral("windowDetection"), QStringLiteral("timeoutMs")},
                   std::clamp(settings.windowDetectionTimeoutMs,
                              kMinWindowDetectionTimeoutMs,
                              kMaxWindowDetectionTimeoutMs));
    setNestedValue(root, {QStringLiteral("windowDetection"), QStringLiteral("env")},
                   envObjectFromMap(settings.windowDetectionEnv));
    setNestedValue(root, {QStringLiteral("env")}, envObjectFromMap(settings.appEnv));
}

}  // namespace

bool writeSettingsConfig(const SettingsConfig &config, QString *error)
{
    if (error) {
        error->clear();
    }

    bool ok = false;
    QJsonObject root = readAppConfigRoot(&ok);
    if (!ok) {
        if (error) {
            *error = MS_TR("Cannot read application config");
        }
        return false;
    }

    // 1. 写入 config.json 中的各设置分组
    writeGeneralSettings(&root, config.general);
    writeCaptureSettings(&root, config.capture);
    writeShortcutSettings(&root, config.shortcuts);
    writeAnnotationSettings(&root, config.annotation);
    writePinnedSettings(&root, config.pinned);
    writeStorageSettings(&root, config.storage);
    writeScrollSettings(&root, config.scroll);
    writeIntegrationSettings(&root, config.integrations);
    writeProviderPreferenceConfig(&root,
                                  ProviderPreferenceConfig{config.pinned.ocrProvider,
                                                           config.pinned.translationProvider,
                                                           config.integrations.codeScanProvider});
    writeAdvancedSettings(&root, config.advanced);
    if (!writeAppConfigRoot(root, error)) {
        return false;
    }

    // 2. 同步系统开机启动项，失败时保留错误给设置弹窗展示
    if (!autostart::setEnabled(config.general.launchOnStartup, error)) {
        return false;
    }

    // 3. 同步标注状态中的当前颜色，确保重启后默认颜色不回退
    AnnotationState annotationState = loadAnnotationState();
    annotationState.currentColor = config.annotation.defaultColor;
    if (!saveAnnotationState(annotationState)) {
        if (error) {
            *error = MS_TR("Cannot save annotation state");
        }
        return false;
    }
    return true;
}

}  // namespace markshot::settings
