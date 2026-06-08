#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

void ShotWindow::runExtensionCommand(const ExtensionCommand &command)
{
    commitTextEditor();
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }

    QString commandLine = command.command;
    if (commandLine.contains(QStringLiteral("{slurp}"))) {
        const QString geometry = slurpSelectionGeometry();
        if (geometry.isEmpty()) {
            return;
        }
        replaceExtensionSlurpPlaceholder(&commandLine, geometry);
    }

    bool replacedImagePlaceholder = false;
    QString imagePath;
    if (command.saveImage) {
        imagePath = saveSelectionToTempFile();
        if (imagePath.isEmpty()) {
            return;
        }
        replacedImagePlaceholder = replaceExtensionImagePlaceholders(&commandLine, imagePath);
        if (!replacedImagePlaceholder) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(imagePath);
        }
    }

    if (commandLine.trimmed().isEmpty()) {
        return;
    }

    const QString workingDirectory = command.workingDirectory.isEmpty()
        ? QString()
        : expandUserPath(command.workingDirectory);

    if (command.closeOnStart) {
        hide();
        QApplication::processEvents();
    }

    const bool started = QProcess::startDetached(markshot::commandShellProgram(),
                                                 markshot::commandShellArguments(commandLine),
                                                 workingDirectory);
    if (started && command.closeOnStart) {
        close();
        return;
    }

    if (!started && command.closeOnStart) {
        show();
        raise();
        activateWindow();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        updateExtensionPanelGeometry();
    }
}

void ShotWindow::startScrollCapture()
{
    commitTextEditor();
    if (!hasUsableSelection()) {
        return;
    }

    const QRect geometry = selectionGlobalRect();
    if (geometry.isEmpty()) {
        return;
    }

    if (isGnomeWaylandSession() && !hasGnomeScrollHelper()) {
        QMessageBox::information(
            this,
            MS_TR("Scroll Capture"),
            MS_TR("Scroll capture is not supported on GNOME Wayland."));
        return;
    }

    const QString outputName = m_outputName;
    const markshot::scroll::ScrollSessionUiConfig uiConfig = scrollSessionUiConfig();
    QScreen *targetScreen = screen();
    QPointer<ShotWindow> self(this);

    // On X11, QScreen::grabWindow captures visible top-level windows. Hide the
    // selection UI and give the compositor one repaint before seeding the scroll
    // stitcher, otherwise the first frame can contain our own toolbar/overlay.
    hide();
    QApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
    QTimer::singleShot(120, qApp, [self, geometry, outputName, targetScreen, uiConfig] {
        auto *window =
            new markshot::scroll::ScrollSessionWindow(geometry, outputName, targetScreen, uiConfig);
        window->show();
        window->raise();
        window->activateWindow();
        if (self) {
            self->close();
        }
    });
}

void ShotWindow::pinSelection()
{
    commitTextEditor();
    if (!hasUsableSelection()) {
        return;
    }

    QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }
    const QRect logicalSelection = selectionGlobalRect();
    if (!logicalSelection.isEmpty()) {
        const qreal dprX = static_cast<qreal>(output.width()) / std::max(1, logicalSelection.width());
        const qreal dprY = static_cast<qreal>(output.height()) / std::max(1, logicalSelection.height());
        const qreal dpr = std::max<qreal>(1.0, (dprX + dprY) / 2.0);
        output.setDevicePixelRatio(dpr);
    }

    auto *window = createPinnedImageWindow(output);
    window->show();
    window->raise();
    window->activateWindow();
    close();
}

void ShotWindow::ocrCopySelection()
{
    commitTextEditor();
    if (!hasUsableSelection()) {
        return;
    }

    const QString tempPath = saveSelectionToTempFile();
    if (tempPath.isEmpty()) {
        return;
    }

    const PinnedWindowConfig config = pinnedWindowConfig();
    if (!config.ocrEnabled) {
        QFile::remove(tempPath);
        return;
    }

    QApplication::setOverrideCursor(Qt::WaitCursor);

    QProcess process;
    if (!config.ocrCommand.isEmpty()) {
        QString commandLine = config.ocrCommand;
        const bool replaced = replaceExtensionImagePlaceholders(&commandLine, tempPath);
        if (!replaced) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(tempPath);
        }
        markshot::setShellCommand(&process, commandLine);
    } else {
        process.setProgram(helperProgramPath(QStringLiteral("mark-shot-ocr")));
        process.setArguments({QStringLiteral("--format"),
                              QStringLiteral("json"),
                              QStringLiteral("--backend"),
                              config.ocrBackend,
                              tempPath});
    }
    process.start();
    if (!process.waitForStarted(3000)) {
        QFile::remove(tempPath);
        QApplication::restoreOverrideCursor();
        showToast(config.ocrCommand.isEmpty()
                      ? MS_TR("OCR helper not found")
                      : MS_TR("OCR failed"));
        return;
    }
    if (!process.waitForFinished(config.ocrTimeoutMs)) {
        process.kill();
        process.waitForFinished(1000);
        QFile::remove(tempPath);
        QApplication::restoreOverrideCursor();
        showToast(MS_TR("OCR timed out"));
        return;
    }

    QFile::remove(tempPath);
    QApplication::restoreOverrideCursor();

    const QByteArray output = process.readAllStandardOutput();
    const QByteArray errorOutput = process.readAllStandardError();
    if (process.exitStatus() != QProcess::NormalExit || process.exitCode() != 0) {
        showToast(config.ocrCommand.isEmpty()
                      && ocrOutputReportsMissingBackend(output, errorOutput, config.ocrBackend)
                      ? MS_TR("OCR backend not installed. Install rapidocr or tesseract.")
                      : MS_TR("OCR failed"));
        return;
    }

    const markshot::ocr::ParsedOutput parsedOcr = markshot::ocr::parseOutput(output);
    if (!parsedOcr.validJson) {
        showToast(config.ocrCommand.isEmpty()
                      && ocrOutputReportsMissingBackend(output, errorOutput, config.ocrBackend)
                      ? MS_TR("OCR backend not installed. Install rapidocr or tesseract.")
                      : MS_TR("OCR failed"));
        return;
    }

    if (parsedOcr.tokens.isEmpty()) {
        showToast(config.ocrCommand.isEmpty()
                      && ocrOutputReportsMissingBackend(output, errorOutput, config.ocrBackend)
                      ? MS_TR("OCR backend not installed. Install rapidocr or tesseract.")
                      : MS_TR("No text recognized"));
        return;
    }

    const QString result = markshot::ocr::tokensText(parsedOcr.tokens);

    if (ocrResultPanelEnabled()) {
        auto *window = createOcrResultWindow(result);
        window->show();
        window->raise();
        window->activateWindow();
        close();
        return;
    }

    markshot::copyTextToClipboard(result);
    if (!sendDesktopNotification(QStringLiteral("Mark Shot"), MS_TR("OCR text copied"), 2500)) {
        showToast(MS_TR("OCR text copied"));
    }
    QTimer::singleShot(150, this, [this] { close(); });
}

void ShotWindow::showToast(const QString &text, int durationMs)
{
    auto *label = new QLabel(text, this);
    label->setAlignment(Qt::AlignCenter);
    label->setFont(markshot::theme::uiFont(12, QFont::DemiBold));
    label->setStyleSheet(QStringLiteral(
        "background: rgba(8, 13, 19, 220);"
        "color: rgba(204, 251, 241, 238);"
        "border-radius: 14px;"
        "padding: 8px 22px;"));
    label->adjustSize();
    label->move((width() - label->width()) / 2, height() - label->height() - 80);
    label->show();
    QTimer::singleShot(durationMs, label, &QObject::deleteLater);
}

QImage ShotWindow::renderedSelection() const
{
    const QRect sourceBounds(QPoint(0, 0), m_frozenFrame.size());
    const QRect selectionRect = normalizedSelection().toAlignedRect().intersected(sourceBounds);
    if (selectionRect.isEmpty()) {
        return {};
    }

    QImage output = m_frozenFrame.copy(selectionRect).convertToFormat(QImage::Format_ARGB32_Premultiplied);
    QPainter painter(&output);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.translate(-selectionRect.topLeft());
    for (const Annotation &annotation : m_annotations) {
        drawAnnotation(painter, annotation, false);
    }
    painter.end();
    return output;
}

QString ShotWindow::defaultSavePath() const
{
    const QString filename = QStringLiteral("mark-shot-%1.png").arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-hhmmss")));
    return QDir(markShotPicturesDir()).filePath(filename);
}

void ShotWindow::saveSelection()
{
    commitTextEditor();

    if (!hasUsableSelection()) {
        return;
    }

    const QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }

    const QString path = defaultSavePath();
    if (output.save(path, "PNG")) {
        const QString message = MS_TR("Saved to %1").arg(path);
        // Keyboard save should finish without another dialog round-trip.
        if (!sendDesktopNotification(QStringLiteral("Mark Shot"), message, 3000)) {
            showToast(message, 2500);
        }
        QTimer::singleShot(150, this, [this] { close(); });
        return;
    }

    showToast(MS_TR("Save failed"), 2500);
}

void ShotWindow::saveSelectionAs()
{
    commitTextEditor();

    if (!hasUsableSelection()) {
        return;
    }

    const QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }

    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }

    hide();

    auto *dialog = new QFileDialog(nullptr, MS_TR("Save Screenshot"));
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    dialog->setAcceptMode(QFileDialog::AcceptSave);
    dialog->setFileMode(QFileDialog::AnyFile);
    dialog->setNameFilter(MS_TR("PNG Images (*.png)"));
    dialog->setDefaultSuffix(QStringLiteral("png"));
    dialog->setOption(QFileDialog::DontUseNativeDialog, true);
    dialog->selectFile(defaultSavePath());

    connect(dialog, &QFileDialog::accepted, this, [this, dialog, output] {
        const QStringList files = dialog->selectedFiles();
        if (!files.isEmpty() && output.save(files.first(), "PNG")) {
            const QString message = MS_TR("Saved to %1").arg(files.first());
            // Prefer desktop notifications because the window may close immediately after saving.
            if (!sendDesktopNotification(QStringLiteral("Mark Shot"), message, 3000)) {
                showToast(message, 2500);
            }
            QTimer::singleShot(150, this, [this] { close(); });
            return;
        }
        showToast(MS_TR("Save failed"), 2500);
        show();
        raise();
        activateWindow();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
    });
    connect(dialog, &QFileDialog::rejected, this, [this] {
        show();
        raise();
        activateWindow();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
    });
    dialog->open();
}

void ShotWindow::copySelection()
{
    commitTextEditor();

    if (!hasUsableSelection()) {
        return;
    }

    QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }

    markshot::copyImageToClipboard(output);

    close();
}
