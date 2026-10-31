#include "shot_window_selection_translate.h"

#include "shot_window_module.h"

#include "providers/ocr/ocr_provider_factory.h"
#include "providers/provider_task.h"
#include "providers/translate/translate_provider_factory.h"
#include "translation_layout/translation_exchange.h"

#include <QTemporaryFile>

#include <algorithm>

using namespace markshot::shot;

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

std::optional<QVector<QString>> ShotWindowSelectionTranslate::translateBlocks(
    const PinnedWindowConfig &config, const QVector<markshot::translation_layout::SourceBlock> &blocks,
    const QString &targetLanguage, QSize imageSize) const
{
    // 1. 段落写入临时 JSON，命令行翻译器和内置翻译器读同一份输入
    QTemporaryFile inputFile(QDir::temp().filePath(QStringLiteral("mark-shot-selection-translate-XXXXXX.json")));
    inputFile.setAutoRemove(true);
    if (!inputFile.open()) {
        m_window.showToast(MS_TR("Translation failed"));
        return std::nullopt;
    }
    const QByteArray inputJson = markshot::translation_layout::translationInputJson(blocks, targetLanguage);
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

    // 4. 译文按段落对应回去，全部为空时视为没有可翻译的文字
    QVector<QString> texts = markshot::translation_layout::translatedBlockTexts(result.output, blocks, imageSize);
    if (std::all_of(texts.cbegin(), texts.cend(), [](const QString &text) { return text.trimmed().isEmpty(); })) {
        m_window.showToast(MS_TR("No text to translate"));
        return std::nullopt;
    }
    return texts;
}
