#include "pinned_window/pinned_image_window.h"

#include "ocr_result.h"
#include "pinned_window/pinned_text_selection_metrics.h"
#include "providers/provider_task.h"
#include "providers/translate/translate_provider_factory.h"
#include "shell_command.h"
#include "translation_layout/block_layout.h"
#include "translation_layout/block_painter.h"
#include "translation_layout/translation_exchange.h"
#include "translation_layout/translation_seed.h"
#include "ui/theme.h"

#include <QDir>
#include <QFile>
#include <QPainter>
#include <QStandardPaths>
#include <QTemporaryFile>

#include <algorithm>

namespace markshot::shot {

namespace layout = markshot::translation_layout;

void PinnedImageWindow::startTranslation(bool activateWhenFinished, bool showBusyCursor)
{
    if (m_ocrTokens.isEmpty()) {
        return;
    }
    cancelTranslation();
    clearTextSelection();
    // 1. 【置顶图片】【段落翻译】相邻行合并成段落后整句翻译，译文按段排回原文位置
    m_translationSourceBlocks = layout::groupParagraphs(sharedOcrTokens(m_ocrTokens));
    if (m_translationSourceBlocks.isEmpty()) {
        return;
    }
    m_activateTranslationWhenFinished = activateWhenFinished;

    QTemporaryFile inputFile(QDir(QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
                                 ? QDir::tempPath()
                                 : QStandardPaths::writableLocation(QStandardPaths::TempLocation))
                                 .filePath(QStringLiteral("mark-shot-translate-XXXXXX.json")));
    inputFile.setAutoRemove(false);
    if (!inputFile.open()) {
        m_activateTranslationWhenFinished = true;
        return;
    }
    m_translationInputPath = inputFile.fileName();
    const QByteArray inputJson =
        layout::translationInputJson(m_translationSourceBlocks, m_config.translationTargetLanguage);
    inputFile.write(inputJson);
    inputFile.close();

    // 2. 组装翻译请求，provider 优先链由工厂解析
    markshot::providers::TranslateTaskRequest request;
    request.inputJson = inputJson;
    request.inputPath = m_translationInputPath;
    request.targetLanguage = m_config.translationTargetLanguage;
    request.configPath = appConfigPath();
    request.provider = m_config.translationProvider;
    if (!m_config.translationCommand.isEmpty()) {
        QString commandLine = m_config.translationCommand;
        bool replaced = false;
        replaceShellPlaceholder(&commandLine, QStringLiteral("{input}"), m_translationInputPath, &replaced);
        replaceShellPlaceholder(&commandLine, QStringLiteral("{inputPath}"), m_translationInputPath, &replaced);
        replaceShellPlaceholder(&commandLine, QStringLiteral("{targetLanguage}"), m_config.translationTargetLanguage, &replaced);
        replaceShellPlaceholder(&commandLine, QStringLiteral("{config}"), appConfigPath(), &replaced);
        if (!replaced) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(m_translationInputPath);
        }
        request.commandLine = commandLine;
    } else {
        request.helperProgram = defaultTranslationHelperProgram();
    }

    // 3. 异步启动任务，超时由任务内部处理
    markshot::providers::ProviderTask *task = markshot::providers::createTranslateTask(request, this);
    m_translationTask = task;
    connect(task,
            &markshot::providers::ProviderTask::finished,
            this,
            [this, task](const markshot::providers::TaskResult &result) {
                finishTranslation(task, result.ok ? result.output : QByteArray());
            });
    if (showBusyCursor) {
        setTranslationBusyCursor(true);
    }
    task->start(m_config.translationTimeoutMs);
    update();
}

QString PinnedImageWindow::defaultTranslationHelperProgram() const
{
    return helperProgramPath(QStringLiteral("mark-shot-translate"));
}

void PinnedImageWindow::cancelTranslation()
{
    if (m_translationTask) {
        disconnect(m_translationTask, nullptr, this, nullptr);
        m_translationTask->cancel();
        m_translationTask->deleteLater();
        m_translationTask = nullptr;
    }
    if (!m_translationInputPath.isEmpty()) {
        QFile::remove(m_translationInputPath);
        m_translationInputPath.clear();
    }
    m_activateTranslationWhenFinished = true;
    setTranslationBusyCursor(false);
}

void PinnedImageWindow::finishTranslation(markshot::providers::ProviderTask *task, const QByteArray &output)
{
    if (task != m_translationTask) {
        return;
    }
    const QVector<QString> texts =
        output.isEmpty() ? QVector<QString>() : layout::translatedBlockTexts(output, m_translationSourceBlocks, m_imageSize);
    if (std::any_of(texts.cbegin(), texts.cend(), [](const QString &text) { return !text.trimmed().isEmpty(); })) {
        // 1. 【置顶图片】【译文排版】在原图上取色并按段落排版，覆盖层随贴图缩放
        setTranslationBlocks(layout::layoutTranslatedBlocks(m_pixmap.toImage(), m_translationSourceBlocks, texts,
                                                            markshot::theme::uiFont()));
        if (!m_translationBlocks.isEmpty()) {
            m_translationActive = m_activateTranslationWhenFinished;
            clearTextSelection();
            updateCursorForPosition(mapFromGlobal(QCursor::pos()));
            update();
        }
    }
    m_translationTask = nullptr;
    if (!m_translationInputPath.isEmpty()) {
        QFile::remove(m_translationInputPath);
        m_translationInputPath.clear();
    }
    setTranslationBusyCursor(false);
    m_activateTranslationWhenFinished = true;
    task->deleteLater();
}

bool PinnedImageWindow::canRequestTranslation() const
{
    return m_config.ocrEnabled && !m_translationTask;
}

void PinnedImageWindow::requestTranslation()
{
    if (!canRequestTranslation()) {
        return;
    }
    if (!m_translationBlocks.isEmpty()) {
        setTranslationActive(true);
        return;
    }
    if (m_ocrTokens.isEmpty()) {
        m_translateAfterOcr = true;
        if (!m_ocrTask) {
            startOcr();
        }
        return;
    }
    m_translateAfterOcr = false;
    startTranslation(true);
}

void PinnedImageWindow::setTranslationActive(bool active)
{
    if (active && m_translationBlocks.isEmpty()) {
        return;
    }
    m_translationActive = active;
    clearTextSelection();
    updateCursorForPosition(mapFromGlobal(QCursor::pos()));
    update();
}

void PinnedImageWindow::setTranslationBusyCursor(bool active)
{
    if (m_translationBusyCursor == active) {
        return;
    }
    m_translationBusyCursor = active;
    // 1. 【置顶图片】【翻译状态】等待反馈限制在当前窗口，其他窗口继续使用正常交互光标
    updateCursorForPosition(mapFromGlobal(QCursor::pos()));
}

QVector<PinnedImageWindow::OcrToken>
PinnedImageWindow::selectableTranslationTokens(const QVector<OcrToken> &tokens) const
{
    QVector<OcrToken> selectableTokens;
    int selectableIndex = 0;
    for (const OcrToken &token : tokens) {
        const QVector<OcrToken> splitTokens = splitTokenForSelection(token);
        for (OcrToken splitToken : splitTokens) {
            splitToken.index = selectableIndex++;
            selectableTokens.append(splitToken);
        }
    }
    return selectableTokens;
}

QVector<PinnedImageWindow::OcrToken> PinnedImageWindow::splitTokenForSelection(const OcrToken &token) const
{
    QVector<OcrToken> splitTokens;
    if (token.text.size() <= 1) {
        splitTokens.append(token);
        return splitTokens;
    }

    const qreal totalWeight = pinnedTextSelectionTextWeight(token.text);
    if (totalWeight <= 0.0 || token.imageRect.width() <= 0.0) {
        splitTokens.append(token);
        return splitTokens;
    }

    for (int i = 0; i < token.text.size(); ++i) {
        OcrToken splitToken = token;
        splitToken.text = token.text.mid(i, 1);
        splitToken.imageRect = pinnedTextSelectionCharacterRect(token.imageRect, token.text, i);
        // 1. 零宽字符并入前一个字，避免空框被命中区域撑开后挡住相邻文字
        if (splitToken.imageRect.isEmpty()) {
            if (!splitTokens.isEmpty()) {
                splitTokens.last().text += splitToken.text;
            }
            continue;
        }
        splitTokens.append(splitToken);
    }
    return splitTokens;
}

void PinnedImageWindow::setTranslationBlocks(QVector<layout::TranslatedBlock> blocks)
{
    m_translationBlocks = std::move(blocks);
    QVector<OcrToken> lineTokens;
    for (const markshot::ocr::Token &token : layout::translatedLineTokens(m_translationBlocks)) {
        lineTokens.append({token.text, token.imageRect, token.line, token.index, token.confidence});
    }
    m_translatedTokens = selectableTranslationTokens(lineTokens);
}

void PinnedImageWindow::applyTranslationSeed(const layout::TranslationSeed &seed)
{
    if (seed.sourceTokens.isEmpty()) {
        return;
    }
    // 1. 【置顶图片】【译文交接】直接使用截图窗口的识别结果，自动识别不再启动
    cancelOcr();
    cancelTranslation();
    m_textSelectionOcrAttempted = true;
    m_ocrTokens.clear();
    for (const markshot::ocr::Token &token : seed.sourceTokens) {
        m_ocrTokens.append({token.text, token.imageRect, token.line, token.index, token.confidence});
    }
    m_ocrSelectionTokens = selectableTranslationTokens(m_ocrTokens);

    // 2. 已排好的译文作为覆盖层，原文与译文之间沿用右键菜单切换
    setTranslationBlocks(seed.blocks);
    m_translationActive = seed.showTranslation && !m_translationBlocks.isEmpty();
    clearTextSelection();
    if (m_translationBlocks.isEmpty() && m_config.autoTranslateAfterOcr) {
        startTranslation(true, false);
    }
    update();
}

void PinnedImageWindow::drawTranslationOverlay(QPainter &painter) const
{
    if (m_translationBlocks.isEmpty() || m_imageSize.isEmpty()) {
        return;
    }
    // 1. 画笔换到图片像素坐标，译文随贴图缩放；底板和文字颜色取自原图，不受全局深色主题影响（issue #81）
    const QRectF displayRect = displayedImageRect();
    painter.save();
    painter.setClipRect(displayRect, Qt::IntersectClip);
    painter.translate(displayRect.topLeft());
    painter.scale(displayRect.width() / m_imageSize.width(), displayRect.height() / m_imageSize.height());
    layout::paintTranslatedBlocks(painter, m_translationBlocks);
    painter.restore();
}

}  // namespace markshot::shot
