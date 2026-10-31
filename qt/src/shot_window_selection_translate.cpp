#include "shot_window_selection_translate.h"

#include "shot_window_module.h"

#include "debug_log.h"
#include "providers/ocr/ocr_provider_factory.h"
#include "providers/provider_task.h"
#include "providers/translate/translate_provider_factory.h"
#include "selection_translate/selection_translate_result.h"
#include "translation_language_options.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QPainter>
#include <QScopeGuard>
#include <QScopedValueRollback>
#include <QTemporaryFile>

#include <algorithm>

using namespace markshot::shot;
using markshot::selection_translate::SelectionTranslateResult;

namespace {

/// 关闭时框选页不提供一键翻译
inline constexpr bool FEATURE_SELECTION_TRANSLATE_ENABLED = true;

/**
 * 把译文画进冻结截图的选区。
 * @param frame 冻结截图，就地修改。
 * @param selectionRect 选区在冻结图中的像素矩形。
 * @param tokens 相对选区左上角的译文框。
 * @return 无返回值。
 */
void bakeTranslationTokens(QImage *frame, const QRect &selectionRect, const QVector<markshot::ocr::Token> &tokens)
{
    if (!frame || selectionRect.isEmpty() || tokens.isEmpty()) {
        return;
    }
    QPainter painter(frame);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setRenderHint(QPainter::TextAntialiasing, true);
    for (const markshot::ocr::Token &token : tokens) {
        if (token.text.trimmed().isEmpty() || token.imageRect.isEmpty()) {
            continue;
        }
        // 1. OCR 坐标相对裁剪图，平移到冻结图后再限制在选区内
        const QRectF textRect = token.imageRect.translated(selectionRect.topLeft())
                                    .adjusted(-2.0, -1.0, 2.0, 1.0)
                                    .intersected(QRectF(selectionRect));
        if (textRect.isEmpty()) {
            continue;
        }
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(255, 255, 255, 235));
        painter.drawRoundedRect(textRect, 2.0, 2.0);
        QFont font = painter.font();
        font.setPixelSize(std::clamp(qRound(textRect.height() * 0.62), 8, 48));
        painter.setFont(font);
        painter.setPen(QColor(17, 24, 39));
        painter.drawText(textRect.adjusted(2.0, 0.0, -2.0, 0.0),
                         Qt::AlignLeft | Qt::AlignVCenter | Qt::TextWordWrap,
                         token.text);
    }
}

/**
 * 把保存的区域像素原样写回冻结截图。
 * @param frame 冻结截图，就地修改。
 * @param rect 区域在冻结图中的像素矩形。
 * @param pixels 区域像素。
 * @return 无返回值。
 */
void paintRegion(QImage *frame, const QRect &rect, const QImage &pixels)
{
    if (!frame || pixels.isNull()) {
        return;
    }
    QPainter painter(frame);
    painter.setCompositionMode(QPainter::CompositionMode_Source);
    painter.drawImage(rect.topLeft(), pixels);
}

/**
 * 组装翻译插件使用的 token JSON。
 * @param tokens OCR token。
 * @param targetLanguage 目标语言。
 * @return 紧凑 JSON。
 */
QByteArray translationInputJson(const QVector<markshot::ocr::Token> &tokens, const QString &targetLanguage)
{
    QJsonArray tokenArray;
    for (const markshot::ocr::Token &token : tokens) {
        QJsonObject object;
        object.insert(QStringLiteral("text"), token.text);
        object.insert(QStringLiteral("box"), markshot::ocr::rectToJson(token.imageRect));
        object.insert(QStringLiteral("line"), token.line);
        object.insert(QStringLiteral("index"), token.index);
        object.insert(QStringLiteral("confidence"), token.confidence);
        tokenArray.append(object);
    }
    QJsonObject root;
    root.insert(QStringLiteral("targetLanguage"), targetLanguage);
    root.insert(QStringLiteral("tokens"), tokenArray);
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

/**
 * 查找目标语言的界面名称。
 * @param value 实际传给翻译器的语言名称。
 * @return 内置语言返回当前界面语言的名称，自定义语言原样返回。
 */
QString languageLabel(const QString &value)
{
    for (const markshot::TranslationLanguageOption &option : markshot::translationLanguageOptions()) {
        if (option.value == value) {
            return option.label;
        }
    }
    return value;
}

}  // namespace

void ShotWindow::translateSelection()
{
    if (!FEATURE_SELECTION_TRANSLATE_ENABLED) {
        return;
    }
    ShotWindowSelectionTranslate(*this).openLanguagePopup();
}

ShotWindowSelectionTranslate::ShotWindowSelectionTranslate(ShotWindow &window)
    : m_window(window)
{
}

void ShotWindowSelectionTranslate::translateTo(const QString &targetLanguage)
{
    m_window.commitTextEditor();
    if (targetLanguage.trimmed().isEmpty() || !m_window.hasUsableSelection()
        || m_window.m_mode != ShotWindow::Mode::Editing) {
        return;
    }
    const PinnedWindowConfig config = pinnedWindowConfig();
    if (!config.ocrEnabled) {
        m_window.showToast(MS_TR("OCR helper not found"));
        return;
    }
    const QRect imageRect = m_window.normalizedSelection().toAlignedRect().intersected(m_window.m_frozenFrame.rect());
    if (imageRect.isEmpty()) {
        return;
    }

    // 1. 【框选翻译】【目标语言】选中的语言作为下次默认值，保存失败不影响本次翻译
    QString error;
    if (!markshot::saveTranslationTargetLanguage(targetLanguage, &error) && !error.isEmpty()) {
        markshot::debugLog("config", "【框选翻译】【目标语言】cannot save translation.targetLanguage: %s",
                           error.toUtf8().constData());
    }

    // 2. 【框选翻译】【重新翻译】已有译文时先写回原文，识别的是原文而不是上一次的译文
    std::optional<SelectionTranslateResult> previous = std::move(m_window.m_controls.m_selectionTranslation);
    m_window.m_controls.m_selectionTranslation.reset();
    if (previous) {
        paintRegion(&m_window.m_frozenFrame, previous->imageRect, previous->originalPixels);
    }
    // 本次没有写回新译文时恢复上一次译文；无论成败都刷新画布与浮条
    const auto finish = qScopeGuard([this, &previous] {
        if (!m_window.m_controls.m_selectionTranslation && previous) {
            paintRegion(&m_window.m_frozenFrame, previous->imageRect, previous->translatedPixels);
            m_window.m_controls.m_selectionTranslation = std::move(previous);
        }
        m_window.m_sharpViewportCache = {};
        updateResultBar();
        m_window.update();
    });

    // 3. 【框选翻译】【识别】导出选区后识别文字，坐标保持在裁剪图内
    const QString tempPath = m_window.saveSelectionToTempFile();
    if (tempPath.isEmpty()) {
        return;
    }
    const auto removeTemp = qScopeGuard([&tempPath] { QFile::remove(tempPath); });
    const auto restoreCursor = qScopeGuard([this] { m_window.updateCursor(); });
    const QScopedValueRollback<bool> busy(m_window.m_operationBusy, true);
    m_window.updateCursor();
    const QSize imageSize = QImage(tempPath).size();
    const std::optional<QVector<markshot::ocr::Token>> sourceTokens = recognize(config, tempPath, imageSize);
    if (!sourceTokens) {
        return;
    }

    // 4. 【框选翻译】【翻译】翻译失败时保留原图
    const std::optional<QVector<markshot::ocr::Token>> translated =
        translate(config, *sourceTokens, targetLanguage, imageSize);
    if (!translated) {
        return;
    }

    // 5. 【框选翻译】【写回】保存原文像素后写入译文，复制和保存会带上译文
    SelectionTranslateResult result;
    result.imageRect = imageRect;
    result.originalPixels = m_window.m_frozenFrame.copy(imageRect);
    bakeTranslationTokens(&m_window.m_frozenFrame, imageRect, *translated);
    result.translatedPixels = m_window.m_frozenFrame.copy(imageRect);
    result.translatedText = markshot::ocr::tokensText(*translated);
    result.languageLabel = languageLabel(targetLanguage);
    m_window.m_controls.m_selectionTranslation = std::move(result);
}

void ShotWindowSelectionTranslate::restoreOriginal()
{
    std::optional<SelectionTranslateResult> &result = m_window.m_controls.m_selectionTranslation;
    if (!result) {
        return;
    }
    paintRegion(&m_window.m_frozenFrame, result->imageRect, result->originalPixels);
    result.reset();
    m_window.m_sharpViewportCache = {};
    updateResultBar();
    m_window.update();
}

std::optional<QVector<markshot::ocr::Token>> ShotWindowSelectionTranslate::recognize(
    const PinnedWindowConfig &config, const QString &imagePath, QSize imageSize) const
{
    // 1. 自定义 OCR 命令优先，未写图片占位符时把图片路径追加到末尾
    markshot::providers::OcrTaskRequest request;
    request.imagePath = imagePath;
    request.backend = config.ocrBackend;
    request.provider = config.ocrProvider;
    if (!config.ocrCommand.isEmpty()) {
        QString commandLine = config.ocrCommand;
        if (!replaceExtensionImagePlaceholders(&commandLine, imagePath)) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(imagePath);
        }
        request.commandLine = commandLine;
    } else {
        request.helperProgram = helperProgramPath(QStringLiteral("mark-shot-ocr"));
    }

    // 2. 等待识别结束，超时和失败分别提示
    markshot::providers::ProviderTask *task = markshot::providers::createOcrTask(request, &m_window);
    task->start(config.ocrTimeoutMs);
    const markshot::providers::TaskResult result = task->waitForResult();
    task->deleteLater();
    if (!result.ok) {
        m_window.showToast(result.error == markshot::providers::TaskError::Timeout ? MS_TR("OCR timed out")
                                                                                   : MS_TR("OCR failed"));
        return std::nullopt;
    }
    QVector<markshot::ocr::Token> tokens = markshot::ocr::tokensFromJsonOutput(result.output, imageSize);
    if (tokens.isEmpty()) {
        m_window.showToast(MS_TR("No text to translate"));
        return std::nullopt;
    }
    return tokens;
}

std::optional<QVector<markshot::ocr::Token>> ShotWindowSelectionTranslate::translate(
    const PinnedWindowConfig &config, const QVector<markshot::ocr::Token> &tokens, const QString &targetLanguage,
    QSize imageSize) const
{
    // 1. 识别结果写入临时 JSON，命令行翻译器和内置翻译器读同一份输入
    QTemporaryFile inputFile(QDir::temp().filePath(QStringLiteral("mark-shot-selection-translate-XXXXXX.json")));
    inputFile.setAutoRemove(true);
    if (!inputFile.open()) {
        m_window.showToast(MS_TR("Translation failed"));
        return std::nullopt;
    }
    const QByteArray inputJson = translationInputJson(tokens, targetLanguage);
    inputFile.write(inputJson);
    inputFile.close();

    // 2. 自定义翻译命令替换占位符，未写输入占位符时把输入路径追加到末尾
    markshot::providers::TranslateTaskRequest request;
    request.inputJson = inputJson;
    request.inputPath = inputFile.fileName();
    request.targetLanguage = targetLanguage;
    request.configPath = appConfigPath();
    request.provider = config.translationProvider;
    if (!config.translationCommand.isEmpty()) {
        QString commandLine = config.translationCommand;
        bool replaced = false;
        replaceShellPlaceholder(&commandLine, QStringLiteral("{input}"), inputFile.fileName(), &replaced);
        replaceShellPlaceholder(&commandLine, QStringLiteral("{inputPath}"), inputFile.fileName(), &replaced);
        replaceShellPlaceholder(&commandLine, QStringLiteral("{targetLanguage}"), targetLanguage, &replaced);
        replaceShellPlaceholder(&commandLine, QStringLiteral("{config}"), appConfigPath(), &replaced);
        if (!replaced) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(inputFile.fileName());
        }
        request.commandLine = commandLine;
    } else {
        request.helperProgram = helperProgramPath(QStringLiteral("mark-shot-translate"));
    }

    // 3. 等待翻译结束，超时和失败分别提示
    markshot::providers::ProviderTask *task = markshot::providers::createTranslateTask(request, &m_window);
    task->start(config.translationTimeoutMs);
    const markshot::providers::TaskResult result = task->waitForResult();
    task->deleteLater();
    if (!result.ok) {
        m_window.showToast(result.error == markshot::providers::TaskError::Timeout ? MS_TR("Translation timed out")
                                                                                   : MS_TR("Translation failed"));
        return std::nullopt;
    }
    QVector<markshot::ocr::Token> translated = markshot::ocr::tokensFromJsonOutput(result.output, imageSize);
    if (translated.isEmpty()) {
        m_window.showToast(MS_TR("No text to translate"));
        return std::nullopt;
    }
    return translated;
}
