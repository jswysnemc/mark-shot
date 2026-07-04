#include "shot_window_module.h"

#include "providers/code_scan/code_scan_provider_factory.h"
#include "providers/provider_task.h"

using namespace markshot::shot;

/**
 * 对当前选区执行二维码和条形码识别。
 * @return 无返回值。
 */
void ShotWindow::scanCodeSelection()
{
    commitTextEditor();
    if (!hasUsableSelection()) {
        return;
    }

    const QString tempPath = saveSelectionToTempFile();
    if (tempPath.isEmpty()) {
        return;
    }

    const CodeScanConfig config = codeScanConfig();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    // 1. 组装扫码请求，provider 优先链由工厂解析
    markshot::providers::CodeScanTaskRequest request;
    request.imagePath = tempPath;
    request.provider = config.provider;
    if (config.command.isEmpty()) {
        request.helperProgram = helperProgramPath(QStringLiteral("mark-shot-code-scan"));
    } else {
        QString commandLine = config.command;
        const bool replaced = replaceExtensionImagePlaceholders(&commandLine, tempPath);
        if (!replaced) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(tempPath);
        }
        request.commandLine = commandLine;
    }

    // 2. 同步等待任务结果，行为与旧版阻塞式调用一致
    markshot::providers::ProviderTask *task = markshot::providers::createCodeScanTask(request, this);
    task->start(config.timeoutMs);
    const markshot::providers::TaskResult result = task->waitForResult();
    task->deleteLater();

    QFile::remove(tempPath);
    QApplication::restoreOverrideCursor();

    if (result.error == markshot::providers::TaskError::StartFailed) {
        showToast(config.command.isEmpty()
                      ? MS_TR("Code scanner helper not found")
                      : MS_TR("Code scan failed"));
        return;
    }
    if (result.error == markshot::providers::TaskError::Timeout) {
        showToast(MS_TR("Code scan timed out"));
        return;
    }

    const QByteArray output = result.output;
    const QByteArray errorOutput = result.errorOutput;
    if (!result.ok) {
        showToast(markshot::code_scan::outputReportsMissingBackend(output, errorOutput)
                      ? MS_TR("Code scanner backend not installed. Install zxing-cpp.")
                      : MS_TR("Code scan failed"));
        return;
    }

    const markshot::code_scan::ParsedOutput parsed = markshot::code_scan::parseOutput(output);
    if (!parsed.validJson) {
        showToast(MS_TR("Code scan failed"));
        return;
    }
    if (parsed.results.isEmpty()) {
        showToast(MS_TR("No code recognized"));
        return;
    }

    auto *window = createCodeScanResultWindow(markshot::code_scan::resultsText(parsed.results));
    window->show();
    window->raise();
    window->activateWindow();
    close();
}
