#include "pinned_window/pinned_image_window.h"

#include "clipboard_image.h"
#include "ocr_result.h"
#include "providers/ocr/ocr_provider_factory.h"
#include "providers/provider_task.h"
#include "shell_command.h"
#include "ui/i18n.h"

#include <QCursor>
#include <QDir>
#include <QFile>
#include <QStandardPaths>
#include <QTemporaryFile>

namespace markshot::shot {

void PinnedImageWindow::startOcr()
{
    cancelOcr();
    if (!m_config.ocrEnabled) {
        m_translateAfterOcr = false;
        m_copyTextAfterOcr = false;
        return;
    }

    QTemporaryFile tempFile(QDir(QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
                                ? QDir::tempPath()
                                : QStandardPaths::writableLocation(QStandardPaths::TempLocation))
                                .filePath(QStringLiteral("mark-shot-pin-ocr-XXXXXX.png")));
    tempFile.setAutoRemove(false);
    if (!tempFile.open()) {
        m_translateAfterOcr = false;
        m_copyTextAfterOcr = false;
        return;
    }
    m_ocrTempPath = tempFile.fileName();
    if (!m_pixmap.save(&tempFile, "PNG")) {
        tempFile.close();
        QFile::remove(m_ocrTempPath);
        m_ocrTempPath.clear();
        m_translateAfterOcr = false;
        m_copyTextAfterOcr = false;
        return;
    }
    tempFile.close();

    // 1. 组装 OCR 请求，provider 优先链由工厂解析
    markshot::providers::OcrTaskRequest request;
    request.imagePath = m_ocrTempPath;
    request.backend = m_config.ocrBackend;
    request.provider = m_config.ocrProvider;
    if (!m_config.ocrCommand.isEmpty()) {
        QString commandLine = m_config.ocrCommand;
        const bool replaced = replaceExtensionImagePlaceholders(&commandLine, m_ocrTempPath);
        if (!replaced) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(m_ocrTempPath);
        }
        request.commandLine = commandLine;
    } else {
        request.helperProgram = defaultOcrHelperProgram();
    }

    // 2. 异步启动任务，超时由任务内部处理
    markshot::providers::ProviderTask *task = markshot::providers::createOcrTask(request, this);
    m_ocrTask = task;
    connect(task,
            &markshot::providers::ProviderTask::finished,
            this,
            [this, task](const markshot::providers::TaskResult &result) {
                finishOcr(task, result);
            });
    task->start(m_config.ocrTimeoutMs);
}

QString PinnedImageWindow::defaultOcrHelperProgram() const
{
    return helperProgramPath(QStringLiteral("mark-shot-ocr"));
}

void PinnedImageWindow::cancelOcr()
{
    if (m_ocrTask) {
        disconnect(m_ocrTask, nullptr, this, nullptr);
        m_ocrTask->cancel();
        m_ocrTask->deleteLater();
        m_ocrTask = nullptr;
    }
    if (!m_ocrTempPath.isEmpty()) {
        QFile::remove(m_ocrTempPath);
        m_ocrTempPath.clear();
    }
}

void PinnedImageWindow::finishOcr(markshot::providers::ProviderTask *task,
                                  const markshot::providers::TaskResult &result)
{
    if (task != m_ocrTask) {
        return;
    }

    if (result.ok && !result.output.isEmpty()) {
        applyOcrOutput(result.output, result.errorOutput);
    } else if (result.error == markshot::providers::TaskError::StartFailed
               && m_config.ocrCommand.isEmpty()) {
        notifyMissingOcrBackend();
        m_translateAfterOcr = false;
        m_copyTextAfterOcr = false;
    } else if (m_config.ocrCommand.isEmpty()
               && ocrOutputReportsMissingBackend(result.output, result.errorOutput, m_config.ocrBackend)) {
        notifyMissingOcrBackend();
        m_translateAfterOcr = false;
        m_copyTextAfterOcr = false;
    } else {
        m_translateAfterOcr = false;
        m_copyTextAfterOcr = false;
    }
    m_ocrTask = nullptr;
    if (!m_ocrTempPath.isEmpty()) {
        QFile::remove(m_ocrTempPath);
        m_ocrTempPath.clear();
    }
    task->deleteLater();
}

void PinnedImageWindow::applyOcrOutput(const QByteArray &output, const QByteArray &errorOutput)
{
    const QVector<OcrToken> tokens = tokensFromJsonOutput(output);
    if (tokens.isEmpty()) {
        if (m_config.ocrCommand.isEmpty()
            && ocrOutputReportsMissingBackend(output, errorOutput, m_config.ocrBackend)) {
            notifyMissingOcrBackend();
        }
        m_translateAfterOcr = false;
        m_copyTextAfterOcr = false;
        return;
    }

    m_ocrTokens = tokens;
    m_translatedTokens.clear();
    m_translationOverlayTokens.clear();
    m_translationActive = false;
    const bool translateAfterOcr = m_translateAfterOcr;
    const bool copyTextAfterOcr = m_copyTextAfterOcr;
    m_translateAfterOcr = false;
    m_copyTextAfterOcr = false;
    updateCursorForPosition(mapFromGlobal(QCursor::pos()));
    if (copyTextAfterOcr) {
        markshot::copyTextToClipboard(allText());
    }
    if (translateAfterOcr) {
        startTranslation(true);
    } else if (m_config.autoTranslateAfterOcr) {
        startTranslation(false, false);
    } else {
        update();
    }
}

void PinnedImageWindow::notifyMissingOcrBackend()
{
    if (m_ocrBackendWarningShown) {
        return;
    }
    m_ocrBackendWarningShown = true;
    sendDesktopNotification(QStringLiteral("Mark Shot"),
                            MS_TR("OCR backend not installed. Install rapidocr or tesseract."));
}

QVector<PinnedImageWindow::OcrToken> PinnedImageWindow::tokensFromJsonOutput(const QByteArray &output) const
{
    QVector<OcrToken> tokens;
    const QVector<markshot::ocr::Token> parsedTokens =
        markshot::ocr::tokensFromJsonOutput(output, m_imageSize);
    tokens.reserve(parsedTokens.size());
    for (const markshot::ocr::Token &parsedToken : parsedTokens) {
        tokens.append({parsedToken.text,
                       parsedToken.imageRect,
                       parsedToken.line,
                       parsedToken.index,
                       parsedToken.confidence});
    }
    return tokens;
}

}  // namespace markshot::shot
