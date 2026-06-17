#include "shot_window_module.h"

using namespace markshot::shot;

/**
 * 上传当前选区图片到图床服务，并将返回的 URL 复制到剪贴板。
 * @return 无返回值。
 */
void ShotWindow::uploadSelection()
{
    commitTextEditor();
    if (!hasUsableSelection()) {
        return;
    }

    const QString tempPath = saveSelectionToTempFile();
    if (tempPath.isEmpty()) {
        return;
    }

    const UploadConfig config = uploadConfig();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    QProcess process;
    process.setProcessEnvironment(config.env);
    if (config.command.isEmpty()) {
        process.setProgram(helperProgramPath(QStringLiteral("mark-shot-upload")));
        process.setArguments({QStringLiteral("--format"), QStringLiteral("json"), tempPath});
    } else {
        QString commandLine = config.command;
        const bool replaced = replaceExtensionImagePlaceholders(&commandLine, tempPath);
        if (!replaced) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(tempPath);
        }
        markshot::setShellCommand(&process, commandLine);
    }
    process.start();
    if (!process.waitForStarted(3000)) {
        QFile::remove(tempPath);
        QApplication::restoreOverrideCursor();
        showToast(config.command.isEmpty()
                      ? MS_TR("Upload helper not found")
                      : MS_TR("Upload failed"));
        return;
    }
    if (!process.waitForFinished(config.timeoutMs)) {
        process.kill();
        process.waitForFinished(1000);
        QFile::remove(tempPath);
        QApplication::restoreOverrideCursor();
        showToast(MS_TR("Upload timed out"));
        return;
    }

    QFile::remove(tempPath);
    QApplication::restoreOverrideCursor();

    const QByteArray output = process.readAllStandardOutput();
    const QByteArray errorOutput = process.readAllStandardError();
    if (process.exitStatus() != QProcess::NormalExit || process.exitCode() != 0) {
        showToast(MS_TR("Upload failed"));
        return;
    }

    QString url;
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
    if (parseError.error == QJsonParseError::NoError && document.isObject()) {
        url = document.object().value(QStringLiteral("url")).toString().trimmed();
    } else {
        // 兼容性处理:自定义命令可能直接输出纯 URL(非 JSON)。
        // 取 stdout 第一行非空内容,若以 http:// 或 https:// 开头则视为 URL。
        const QString text = QString::fromUtf8(output).trimmed();
        const QStringList lines = text.split(QLatin1Char('\n'), Qt::SkipEmptyParts);
        if (!lines.isEmpty()) {
            const QString first = lines.first().trimmed();
            if (first.startsWith(QStringLiteral("http://"), Qt::CaseInsensitive)
                || first.startsWith(QStringLiteral("https://"), Qt::CaseInsensitive)) {
                url = first;
            }
        }
    }

    if (url.isEmpty()) {
        showToast(MS_TR("Upload failed"));
        return;
    }

    markshot::copyTextToClipboard(url);
    if (!sendDesktopNotification(QStringLiteral("Mark Shot"), MS_TR("Image URL copied"), 2500)) {
        showToast(MS_TR("Image URL copied"));
    }
    QTimer::singleShot(150, this, [this] { close(); });
}
