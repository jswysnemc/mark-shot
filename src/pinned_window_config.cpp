#include "shot_window_internal.h"

#include "config_value.h"

#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QProcessEnvironment>

#include <algorithm>

namespace cfg = markshot::config;

namespace markshot::shot {
namespace {

/**
 * 从根配置读取 OCR 配置到置顶图片配置。
 * @param root 应用配置根对象。
 * @param config 需要更新的置顶图片配置。
 * @return 无返回值。
 */
void applyOcrConfig(const QJsonObject &root, PinnedWindowConfig *config)
{
    const QJsonObject ocr = cfg::firstObjectValue(root, QStringLiteral("ocr"));
    if (ocr.value(QStringLiteral("enabled")).isBool()) {
        config->ocrEnabled = ocr.value(QStringLiteral("enabled")).toBool();
    }
    if (const std::optional<bool> autoOcr =
            cfg::boolValue(ocr,
                           {QStringLiteral("autoOnPin"),
                            QStringLiteral("autoPinned"),
                            QStringLiteral("autoOcr"),
                            QStringLiteral("autoOCR"),
                            QStringLiteral("background")});
        autoOcr.has_value()) {
        config->autoOcr = *autoOcr;
    }
    config->ocrBackend = ocr.value(QStringLiteral("backend")).toString(config->ocrBackend).trimmed();
    config->ocrCommand = ocr.value(QStringLiteral("command")).toString().trimmed();
    if (ocr.value(QStringLiteral("timeoutMs")).isDouble()) {
        config->ocrTimeoutMs = std::max(1000, ocr.value(QStringLiteral("timeoutMs")).toInt(config->ocrTimeoutMs));
    }
}

/**
 * 从根配置读取翻译配置到置顶图片配置。
 * @param root 应用配置根对象。
 * @param config 需要更新的置顶图片配置。
 * @return 无返回值。
 */
void applyTranslationConfig(const QJsonObject &root, PinnedWindowConfig *config)
{
    const QJsonObject translation = cfg::firstObjectValue(root, QStringLiteral("translation"));
    config->translationCommand = translation.value(QStringLiteral("command")).toString().trimmed();
    config->translationTargetLanguage = translation.value(QStringLiteral("targetLanguage"))
                                            .toString(config->translationTargetLanguage)
                                            .trimmed();
    if (const std::optional<bool> autoTranslate =
            cfg::boolValue(translation,
                           {QStringLiteral("autoAfterOcr"),
                            QStringLiteral("autoAfterOCR"),
                            QStringLiteral("autoTranslateAfterOcr"),
                            QStringLiteral("autoTranslateAfterOCR"),
                            QStringLiteral("auto")});
        autoTranslate.has_value()) {
        config->autoTranslateAfterOcr = *autoTranslate;
    }
    if (translation.value(QStringLiteral("timeoutMs")).isDouble()) {
        config->translationTimeoutMs =
            std::max(3000, translation.value(QStringLiteral("timeoutMs")).toInt(config->translationTimeoutMs));
    }
}

/**
 * 从置顶图片配置读取文本拖选复制权限。
 * @param pinned 置顶图片配置对象。
 * @param config 需要更新的置顶图片配置。
 * @return 无返回值。
 */
void applyPinnedTextSelectionConfig(const QJsonObject &pinned, PinnedWindowConfig *config)
{
    if (const std::optional<bool> textSelectionCopy =
            cfg::boolValue(pinned,
                           {QStringLiteral("textSelectionCopyEnabled"),
                            QStringLiteral("textSelectionEnabled"),
                            QStringLiteral("allowTextSelection"),
                            QStringLiteral("allowTextSelectionCopy"),
                            QStringLiteral("selectableText"),
                            QStringLiteral("selectableOcrText"),
                            QStringLiteral("selectableOCRText")});
        textSelectionCopy.has_value()) {
        config->textSelectionCopyEnabled = *textSelectionCopy;
    }
}

/**
 * 从置顶图片配置读取边框配置。
 * @param pinned 置顶图片配置对象。
 * @param config 需要更新的置顶图片配置。
 * @return 无返回值。
 */
void applyPinnedBorderConfig(const QJsonObject &pinned, PinnedWindowConfig *config)
{
    const QJsonValue border = pinned.value(QStringLiteral("border"));
    if (border.isBool()) {
        config->borderEnabled = border.toBool();
    } else if (border.isObject()) {
        const QJsonObject borderObject = border.toObject();
        if (borderObject.value(QStringLiteral("enabled")).isBool()) {
            config->borderEnabled = borderObject.value(QStringLiteral("enabled")).toBool();
        } else {
            config->borderEnabled = true;
        }
        if (const std::optional<QColor> color = colorFromConfigValue(borderObject)) {
            config->borderColor = *color;
        }
        if (const std::optional<qreal> width =
                realFromConfigValue(borderObject.value(QStringLiteral("width")))) {
            config->borderWidth = std::clamp(*width, 1.0, 12.0);
        }
    }
    if (pinned.value(QStringLiteral("borderEnabled")).isBool()) {
        config->borderEnabled = pinned.value(QStringLiteral("borderEnabled")).toBool();
    }
    if (const std::optional<QColor> color =
            colorFromConfigValue(pinned.value(QStringLiteral("borderColor")))) {
        config->borderColor = *color;
    }
    if (const std::optional<qreal> width =
            realFromConfigValue(pinned.value(QStringLiteral("borderWidth")))) {
        config->borderWidth = std::clamp(*width, 1.0, 12.0);
    }
}

/**
 * 从根配置读取置顶图片专属配置。
 * @param root 应用配置根对象。
 * @param config 需要更新的置顶图片配置。
 * @return 无返回值。
 */
void applyPinnedConfig(const QJsonObject &root, PinnedWindowConfig *config)
{
    const QJsonObject pinned = cfg::firstObjectValue(root,
                                                     {QStringLiteral("pinnedWindow"),
                                                      QStringLiteral("pinned"),
                                                      QStringLiteral("pin")});
    if (pinned.isEmpty()) {
        return;
    }
    if (const std::optional<bool> alwaysOnTop =
            cfg::boolValue(pinned,
                           {QStringLiteral("alwaysOnTop"),
                            QStringLiteral("stayOnTop"),
                            QStringLiteral("topmost"),
                            QStringLiteral("above")});
        alwaysOnTop.has_value()) {
        config->alwaysOnTop = *alwaysOnTop;
    }
    if (const std::optional<bool> autoOcr =
            cfg::boolValue(pinned,
                           {QStringLiteral("autoOcr"),
                            QStringLiteral("autoOCR"),
                            QStringLiteral("autoRecognizeText"),
                            QStringLiteral("autoTextRecognition"),
                            QStringLiteral("ocrOnPin"),
                            QStringLiteral("backgroundOcr"),
                            QStringLiteral("backgroundOCR")});
        autoOcr.has_value()) {
        config->autoOcr = *autoOcr;
    }
    applyPinnedTextSelectionConfig(pinned, config);
    applyPinnedBorderConfig(pinned, config);
}

/**
 * 从环境变量覆盖置顶图片 OCR 配置。
 * @param env 系统环境变量。
 * @param config 需要更新的置顶图片配置。
 * @return 无返回值。
 */
void applyOcrEnvironmentConfig(const QProcessEnvironment &env, PinnedWindowConfig *config)
{
    const QString disabled = env.value(QStringLiteral("MARK_SHOT_OCR_DISABLED")).trimmed().toLower();
    if (disabled == QStringLiteral("1") || disabled == QStringLiteral("true") || disabled == QStringLiteral("yes")) {
        config->ocrEnabled = false;
    }
    const QString autoOcr = env.value(QStringLiteral("MARK_SHOT_PINNED_AUTO_OCR")).trimmed().toLower();
    if (autoOcr == QStringLiteral("1") || autoOcr == QStringLiteral("true") || autoOcr == QStringLiteral("yes")) {
        config->autoOcr = true;
    } else if (autoOcr == QStringLiteral("0") || autoOcr == QStringLiteral("false")
               || autoOcr == QStringLiteral("no")) {
        config->autoOcr = false;
    }
    const QString envOcrBackend = env.value(QStringLiteral("MARK_SHOT_OCR_BACKEND")).trimmed();
    if (!envOcrBackend.isEmpty()) {
        config->ocrBackend = envOcrBackend;
    }
    const QString envOcrCommand = env.value(QStringLiteral("MARK_SHOT_OCR_COMMAND")).trimmed();
    if (!envOcrCommand.isEmpty()) {
        config->ocrCommand = envOcrCommand;
    }
}

/**
 * 从环境变量覆盖置顶图片翻译配置。
 * @param env 系统环境变量。
 * @param config 需要更新的置顶图片配置。
 * @return 无返回值。
 */
void applyTranslationEnvironmentConfig(const QProcessEnvironment &env, PinnedWindowConfig *config)
{
    const QString envTargetLanguage = env.value(QStringLiteral("MARK_SHOT_TRANSLATION_TARGET_LANGUAGE")).trimmed();
    if (!envTargetLanguage.isEmpty()) {
        config->translationTargetLanguage = envTargetLanguage;
    }
    const QString envTranslationCommand = env.value(QStringLiteral("MARK_SHOT_TRANSLATION_COMMAND")).trimmed();
    if (!envTranslationCommand.isEmpty()) {
        config->translationCommand = envTranslationCommand;
    }
    const QString autoTranslate = env.value(QStringLiteral("MARK_SHOT_TRANSLATION_AUTO_AFTER_OCR")).trimmed().toLower();
    if (autoTranslate == QStringLiteral("1") || autoTranslate == QStringLiteral("true")
        || autoTranslate == QStringLiteral("yes")) {
        config->autoTranslateAfterOcr = true;
    } else if (autoTranslate == QStringLiteral("0") || autoTranslate == QStringLiteral("false")
               || autoTranslate == QStringLiteral("no")) {
        config->autoTranslateAfterOcr = false;
    }
}

/**
 * 规范化置顶图片配置中的空值。
 * @param config 需要规范化的置顶图片配置。
 * @return 无返回值。
 */
void normalizePinnedWindowConfig(PinnedWindowConfig *config)
{
    if (config->ocrBackend.isEmpty()) {
        config->ocrBackend = QStringLiteral("auto");
    }
    if (config->translationTargetLanguage.isEmpty()) {
        config->translationTargetLanguage = QStringLiteral("Simplified Chinese");
    }
}

}  // namespace

/**
 * 从应用配置根对象解析置顶图片窗口配置。
 * @param root 应用配置根对象。
 * @return 置顶图片窗口配置。
 */
PinnedWindowConfig pinnedWindowConfigFromRoot(const QJsonObject &root)
{
    PinnedWindowConfig config;
    // 1. 读取通用 OCR 和翻译配置
    applyOcrConfig(root, &config);
    applyTranslationConfig(root, &config);
    // 2. 读取置顶图片专属覆盖配置
    applyPinnedConfig(root, &config);
    normalizePinnedWindowConfig(&config);
    return config;
}

/**
 * 读取置顶图片窗口配置。
 * @return 置顶图片窗口配置。
 */
PinnedWindowConfig pinnedWindowConfig()
{
    PinnedWindowConfig config;
    QFile file(appConfigPath());
    if (file.exists() && file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
        if (parseError.error == QJsonParseError::NoError && document.isObject()) {
            config = pinnedWindowConfigFromRoot(document.object());
        }
    }

    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    // 1. 环境变量优先级高于配置文件
    applyOcrEnvironmentConfig(env, &config);
    applyTranslationEnvironmentConfig(env, &config);
    normalizePinnedWindowConfig(&config);
    return config;
}

}  // namespace markshot::shot
