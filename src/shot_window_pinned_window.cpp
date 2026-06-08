#include "shot_window_module.h"
namespace {
using namespace markshot::shot;
/// @brief Pinned image window providing OCR, text selection, and translation.
class PinnedImageWindow final : public QWidget {
public:
    /// @brief Bounding structure representing a single OCR token.
    struct OcrToken {
        /// @brief Text content of the recognized token.
        QString text;
        /// @brief Bounding rectangle of the token within the image.
        QRectF imageRect;
        /// @brief Line index of the token.
        int line = 0;
        /// @brief Index of the token within the line.
        int index = 0;
        /// @brief Confidence score of the recognized token.
        qreal confidence = 0.0;
    };
    explicit PinnedImageWindow(QImage image)
        : m_pixmap(QPixmap::fromImage(std::move(image)))
        , m_imageSize(m_pixmap.size())
        , m_displayBaseSize(displayBaseSizeForPixmap())
        , m_config(pinnedWindowConfig())
    {
        setWindowTitle(MS_TR("Pinned Mark Shot"));
        setAttribute(Qt::WA_DeleteOnClose);
        setAttribute(Qt::WA_ShowWithoutActivating);
        setWindowFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
        setFocusPolicy(Qt::StrongFocus);
        setMouseTracking(true);
        setCursor(Qt::OpenHandCursor);
        QSize targetSize = m_displayBaseSize;
        QScreen *screen = QGuiApplication::screenAt(QCursor::pos());
        if (!screen) {
            screen = QApplication::primaryScreen();
        }
        if (screen) {
            const QSize maxSize = screen->availableGeometry().size() * 0.9;
            if (targetSize.width() > maxSize.width() || targetSize.height() > maxSize.height()) {
                targetSize.scale(maxSize, Qt::KeepAspectRatio);
            }
            m_scale = static_cast<qreal>(targetSize.width()) / std::max(1, m_displayBaseSize.width());
            setFixedSize(targetSize);
            move(screen->availableGeometry().center() - rect().center());
        } else {
            setFixedSize(targetSize);
        }
        if (m_config.autoOcr) {
            QTimer::singleShot(0, this, [this] { startOcr(); });
        }
    }
    ~PinnedImageWindow() override
    {
        cancelTranslation();
        cancelOcr();
    }
protected:
    /// @brief Handles window state changes and raises the pinned window after focus changes.
    /// @param event Current window event.
    /// @return Whether QWidget handled the event.
    bool event(QEvent *event) override
    {
        const bool shouldRaise = event->type() == QEvent::WindowDeactivate
            || event->type() == QEvent::ActivationChange
            || event->type() == QEvent::Show;
        const bool handled = QWidget::event(event);
        if (shouldRaise) {
            QTimer::singleShot(0, this, [this] { raisePinnedWindow(); });
        }
        return handled;
    }
    void paintEvent(QPaintEvent *) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
        painter.drawPixmap(rect(), m_pixmap);
        if (m_translationActive) {
            drawTranslationOverlay(painter);
        }
        auto drawBorder = [this, &painter] {
            if (!m_config.borderEnabled || !m_config.borderColor.isValid() || m_config.borderWidth <= 0.0) {
                return;
            }
            painter.save();
            painter.setRenderHint(QPainter::Antialiasing, false);
            painter.setBrush(Qt::NoBrush);
            painter.setPen(QPen(m_config.borderColor, m_config.borderWidth));
            const qreal inset = m_config.borderWidth / 2.0;
            painter.drawRect(QRectF(rect()).adjusted(inset, inset, -inset, -inset));
            painter.restore();
        };
        if (!hasTextSelection()) {
            drawBorder();
            return;
        }
        painter.setRenderHint(QPainter::Antialiasing, false);
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(72, 132, 245, 96));
        const auto [first, last] = selectionRange();
        const QVector<OcrToken> &tokens = activeTokens();
        for (int i = first; i <= last; ++i) {
            painter.drawRect(imageToWidget(tokens.at(i).imageRect).intersected(QRectF(rect())));
        }
        drawBorder();
    }
    void mousePressEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
            if (const std::optional<int> token = tokenAt(widgetToImage(event->position()))) {
                m_selectingText = true;
                m_selectionAnchor = *token;
                m_selectionFocus = *token;
                setCursor(Qt::IBeamCursor);
                update();
                event->accept();
                return;
            }
            clearTextSelection();
            m_dragOffset = event->globalPosition().toPoint() - frameGeometry().topLeft();
            setCursor(Qt::ClosedHandCursor);
            if (QWindow *window = windowHandle()) {
                if (window->startSystemMove()) {
                    event->accept();
                    return;
                }
            }
            event->accept();
            return;
        }
        QWidget::mousePressEvent(event);
    }
    void mouseMoveEvent(QMouseEvent *event) override
    {
        if (m_selectingText) {
            const QPointF imagePoint = widgetToImage(event->position());
            const std::optional<int> token = tokenAt(imagePoint).has_value()
                ? tokenAt(imagePoint)
                : closestToken(imagePoint);
            if (token && m_selectionFocus != *token) {
                m_selectionFocus = *token;
                update();
            }
            event->accept();
            return;
        }
        if (event->buttons().testFlag(Qt::LeftButton)) {
            move(event->globalPosition().toPoint() - m_dragOffset);
            event->accept();
            return;
        }
        updateCursorForPosition(event->position());
        QWidget::mouseMoveEvent(event);
    }
    void mouseReleaseEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
            if (m_selectingText) {
                m_selectingText = false;
                updateCursorForPosition(event->position());
                event->accept();
                return;
            }
            updateCursorForPosition(event->position());
            event->accept();
            return;
        }
        QWidget::mouseReleaseEvent(event);
    }
    void wheelEvent(QWheelEvent *event) override
    {
        const QPoint delta = event->angleDelta();
        const QPoint pixelDelta = event->pixelDelta();
        if (delta.y() == 0 && pixelDelta.y() == 0) {
            QWidget::wheelEvent(event);
            return;
        }
        qreal wheelSteps = static_cast<qreal>(delta.y()) / 120.0;
        if (qFuzzyIsNull(wheelSteps) && pixelDelta.y() != 0) {
            wheelSteps = static_cast<qreal>(pixelDelta.y()) / 80.0;
        }
        const qreal factor = std::pow(1.08, wheelSteps);
        resizeByScale(m_scale * factor, event->globalPosition().toPoint(), event->position());
        event->accept();
    }
    void mouseDoubleClickEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
            if (const std::optional<int> token = tokenAt(widgetToImage(event->position()))) {
                m_selectionAnchor = *token;
                m_selectionFocus = *token;
                update();
                event->accept();
                return;
            }
            close();
            event->accept();
            return;
        }
        QWidget::mouseDoubleClickEvent(event);
    }
    void contextMenuEvent(QContextMenuEvent *event) override
    {
        /// @brief Event filter to intercept non-left click mouse events on the menu.
        class LeftClickMenuFilter final : public QObject {
        public:
            explicit LeftClickMenuFilter(QObject *parent = nullptr) : QObject(parent) {}
        protected:
            bool eventFilter(QObject *obj, QEvent *event) override
            {
                if (event->type() == QEvent::MouseButtonPress || event->type() == QEvent::MouseButtonRelease || event->type() == QEvent::MouseButtonDblClick) {
                    QMouseEvent *me = static_cast<QMouseEvent *>(event);
                    if (me->button() != Qt::LeftButton) {
                        QWidget *widget = qobject_cast<QWidget *>(obj);
                        if (widget && widget->rect().contains(me->position().toPoint())) {
                            return true;
                        }
                    }
                }
                return QObject::eventFilter(obj, event);
            }
        };
        QMenu menu(this);
        LeftClickMenuFilter filter(&menu);
        menu.installEventFilter(&filter);
        menu.addAction(MS_TR("Rotate Left"), this, [this] { rotateImage(-90); });
        menu.addAction(MS_TR("Rotate Right"), this, [this] { rotateImage(90); });
        menu.addSeparator();
        menu.addAction(MS_TR("Zoom In"), this, [this, event] {
            resizeByScale(m_scale * 1.18, event->globalPos(), rect().center());
        });
        menu.addAction(MS_TR("Zoom Out"), this, [this, event] {
            resizeByScale(m_scale / 1.18, event->globalPos(), rect().center());
        });
        menu.addAction(MS_TR("Reset Size"), this, [this] {
            resizeByScale(1.0, frameGeometry().center(), QPointF(width() / 2.0, height() / 2.0));
        });
        menu.addSeparator();
        menu.addAction(MS_TR("Copy"), this, [this] {
            markshot::copyImageToClipboard(m_pixmap.toImage());
        });
        QAction *copySelectedTextAction = menu.addAction(MS_TR("Copy Selected Text"), this, [this] {
            markshot::copyTextToClipboard(selectedText());
        });
        copySelectedTextAction->setEnabled(hasTextSelection());
        QAction *copyTextAction = menu.addAction(MS_TR("Copy Image Text"), this, [this] {
            copyImageText();
        });
        copyTextAction->setEnabled(m_config.ocrEnabled);
        QAction *translateAction = menu.addAction(MS_TR("Translate"), this, [this] {
            requestTranslation();
        });
        translateAction->setEnabled(canRequestTranslation());
        QAction *toggleTranslationAction = menu.addAction(
            m_translationActive ? MS_TR("Show Original Text") : MS_TR("Show Translated Text"),
            this,
            [this] { setTranslationActive(!m_translationActive); });
        toggleTranslationAction->setEnabled(!m_translationOverlayTokens.isEmpty() && !m_translationProcess);
        menu.addAction(MS_TR("Save As"), this, [this] { saveImageAs(); });
        menu.addSeparator();
        menu.addAction(MS_TR("Close"), this, &QWidget::close);
        menu.exec(event->globalPos());
    }
    void keyPressEvent(QKeyEvent *event) override
    {
        if (event->matches(QKeySequence::Copy) && hasTextSelection()) {
            markshot::copyTextToClipboard(selectedText());
            event->accept();
            return;
        }
        if (event->key() == Qt::Key_Escape) {
            close();
            return;
        }
        QWidget::keyPressEvent(event);
    }
private:
    void rotateImage(qreal degrees)
    {
        const QPoint center = frameGeometry().center();
        m_pixmap = m_pixmap.transformed(QTransform().rotate(degrees), Qt::SmoothTransformation);
        m_imageSize = m_pixmap.size();
        m_displayBaseSize = displayBaseSizeForPixmap();
        clearTextSelection();
        m_ocrTokens.clear();
        m_translatedTokens.clear();
        m_translationOverlayTokens.clear();
        m_translationActive = false;
        m_translateAfterOcr = false;
        m_copyTextAfterOcr = false;
        cancelTranslation();
        resizeByScale(m_scale, center, QPointF(width() / 2.0, height() / 2.0));
        update();
        if (m_config.autoOcr) {
            startOcr();
        }
    }
    void saveImageAs()
    {
        const QString filename = QStringLiteral("mark-shot-pin-%1.png").arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-hhmmss")));
        const QString path = QFileDialog::getSaveFileName(this,
                                                          MS_TR("Save Pinned Image"),
                                                          QDir(markShotPicturesDir()).filePath(filename),
                                                          MS_TR("PNG Images (*.png)"));
        if (!path.isEmpty()) {
            m_pixmap.save(path, "PNG");
        }
    }
    /// @brief Computes the device-independent base display size for the pinned window.
    /// @return Logical display size for the current image.
    QSize displayBaseSizeForPixmap() const
    {
        const QSizeF logicalSize = m_pixmap.deviceIndependentSize();
        return QSize(std::max(1, qRound(logicalSize.width())),
                     std::max(1, qRound(logicalSize.height())));
    }
    /// @brief Raises the pinned window to the top of the current window stack.
    /// @return Nothing.
    void raisePinnedWindow()
    {
        raise();
        if (QWindow *window = windowHandle()) {
            window->raise();
        }
    }
    void resizeByScale(qreal scale, QPoint globalAnchor, QPointF localAnchor)
    {
        scale = std::clamp(scale, 0.1, 6.0);
        QSize targetSize(qMax(24, qRound(m_displayBaseSize.width() * scale)),
                         qMax(24, qRound(m_displayBaseSize.height() * scale)));
        targetSize.scale(targetSize, Qt::KeepAspectRatio);
        const qreal xRatio = width() > 0 ? localAnchor.x() / width() : 0.5;
        const qreal yRatio = height() > 0 ? localAnchor.y() / height() : 0.5;
        const QPoint topLeft(globalAnchor.x() - qRound(targetSize.width() * xRatio),
                             globalAnchor.y() - qRound(targetSize.height() * yRatio));
        m_scale = static_cast<qreal>(targetSize.width()) / std::max(1, m_displayBaseSize.width());
        setMinimumSize(QSize(24, 24));
        setMaximumSize(QSize(QWIDGETSIZE_MAX, QWIDGETSIZE_MAX));
        setGeometry(QRect(topLeft, targetSize));
        setFixedSize(targetSize);
    }
    void startOcr()
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
        auto *process = new QProcess(this);
        m_ocrProcess = process;
        if (!m_config.ocrCommand.isEmpty()) {
            QString commandLine = m_config.ocrCommand;
            const bool replaced = replaceExtensionImagePlaceholders(&commandLine, m_ocrTempPath);
            if (!replaced) {
                commandLine += QLatin1Char(' ');
                commandLine += shellQuote(m_ocrTempPath);
            }
            markshot::setShellCommand(process, commandLine);
        } else {
            process->setProgram(defaultOcrHelperProgram());
            process->setArguments({QStringLiteral("--format"),
                                   QStringLiteral("json"),
                                   QStringLiteral("--backend"),
                                   m_config.ocrBackend,
                                   m_ocrTempPath});
        }
        connect(process, &QProcess::errorOccurred, this, [this, process](QProcess::ProcessError error) {
            if (process == m_ocrProcess && process->state() == QProcess::NotRunning) {
                finishOcr(process,
                          QByteArray(),
                          process->readAllStandardError(),
                          -1,
                          QProcess::CrashExit,
                          error);
            }
        });
        connect(process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
                [this, process](int exitCode, QProcess::ExitStatus exitStatus) {
            finishOcr(process,
                      process->readAllStandardOutput(),
                      process->readAllStandardError(),
                      exitCode,
                      exitStatus,
                      process->error());
        });
        QTimer::singleShot(m_config.ocrTimeoutMs, process, [process] {
            if (process->state() != QProcess::NotRunning) {
                process->kill();
            }
        });
        process->start();
    }
    QString defaultOcrHelperProgram() const
    {
        return helperProgramPath(QStringLiteral("mark-shot-ocr"));
    }
    void cancelOcr()
    {
        if (m_ocrProcess) {
            disconnect(m_ocrProcess, nullptr, this, nullptr);
            if (m_ocrProcess->state() != QProcess::NotRunning) {
                m_ocrProcess->kill();
            }
            m_ocrProcess->deleteLater();
            m_ocrProcess = nullptr;
        }
        if (!m_ocrTempPath.isEmpty()) {
            QFile::remove(m_ocrTempPath);
            m_ocrTempPath.clear();
        }
    }
    void finishOcr(QProcess *process,
                   const QByteArray &output,
                   const QByteArray &errorOutput,
                   int exitCode,
                   QProcess::ExitStatus exitStatus,
                   QProcess::ProcessError processError)
    {
        if (process != m_ocrProcess) {
            return;
        }
        const bool success = exitStatus == QProcess::NormalExit && exitCode == 0;
        if (success && !output.isEmpty()) {
            applyOcrOutput(output, errorOutput);
        } else if (processError == QProcess::FailedToStart && m_config.ocrCommand.isEmpty()) {
            notifyMissingOcrBackend();
            m_translateAfterOcr = false;
            m_copyTextAfterOcr = false;
        } else if (m_config.ocrCommand.isEmpty()
                   && ocrOutputReportsMissingBackend(output, errorOutput, m_config.ocrBackend)) {
            notifyMissingOcrBackend();
            m_translateAfterOcr = false;
            m_copyTextAfterOcr = false;
        } else {
            m_translateAfterOcr = false;
            m_copyTextAfterOcr = false;
        }
        m_ocrProcess = nullptr;
        if (!m_ocrTempPath.isEmpty()) {
            QFile::remove(m_ocrTempPath);
            m_ocrTempPath.clear();
        }
        process->deleteLater();
    }
    void applyOcrOutput(const QByteArray &output, const QByteArray &errorOutput)
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
    void notifyMissingOcrBackend()
    {
        if (m_ocrBackendWarningShown) {
            return;
        }
        m_ocrBackendWarningShown = true;
        sendDesktopNotification(QStringLiteral("Mark Shot"),
                                MS_TR("OCR backend not installed. Install rapidocr or tesseract."));
    }
    QVector<OcrToken> tokensFromJsonOutput(const QByteArray &output) const
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
    void startTranslation(bool activateWhenFinished = true, bool showBusyCursor = true)
    {
        if (m_ocrTokens.isEmpty()) {
            return;
        }
        cancelTranslation();
        clearTextSelection();
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
        inputFile.write(translationInputJson());
        inputFile.close();
        auto *process = new QProcess(this);
        m_translationProcess = process;
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
            markshot::setShellCommand(process, commandLine);
        } else {
            process->setProgram(defaultTranslationHelperProgram());
            process->setArguments({QStringLiteral("--input"),
                                   m_translationInputPath,
                                   QStringLiteral("--target-language"),
                                   m_config.translationTargetLanguage,
                                   QStringLiteral("--config"),
                                   appConfigPath()});
        }
        connect(process, &QProcess::errorOccurred, this, [this, process] {
            if (process == m_translationProcess && process->state() == QProcess::NotRunning) {
                finishTranslation(process, QByteArray());
            }
        });
        connect(process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
                [this, process](int exitCode, QProcess::ExitStatus exitStatus) {
            const QByteArray output = exitStatus == QProcess::NormalExit && exitCode == 0
                ? process->readAllStandardOutput()
                : QByteArray();
            finishTranslation(process, output);
        });
        QTimer::singleShot(m_config.translationTimeoutMs, process, [process] {
            if (process->state() != QProcess::NotRunning) {
                process->kill();
            }
        });
        if (showBusyCursor) {
            setTranslationBusyCursor(true);
        }
        process->start();
        update();
    }
    QByteArray translationInputJson() const
    {
        QJsonArray tokens;
        for (const OcrToken &token : m_ocrTokens) {
            QJsonObject object;
            object.insert(QStringLiteral("text"), token.text);
            object.insert(QStringLiteral("box"), rectToJson(token.imageRect));
            object.insert(QStringLiteral("line"), token.line);
            object.insert(QStringLiteral("index"), token.index);
            object.insert(QStringLiteral("confidence"), token.confidence);
            tokens.append(object);
        }
        QJsonObject root;
        root.insert(QStringLiteral("targetLanguage"), m_config.translationTargetLanguage);
        root.insert(QStringLiteral("tokens"), tokens);
        return QJsonDocument(root).toJson(QJsonDocument::Compact);
    }
    QJsonArray rectToJson(QRectF rect) const
    {
        return markshot::ocr::rectToJson(rect);
    }
    QString defaultTranslationHelperProgram() const
    {
        return helperProgramPath(QStringLiteral("mark-shot-translate"));
    }
    void cancelTranslation()
    {
        if (m_translationProcess) {
            disconnect(m_translationProcess, nullptr, this, nullptr);
            if (m_translationProcess->state() != QProcess::NotRunning) {
                m_translationProcess->kill();
            }
            m_translationProcess->deleteLater();
            m_translationProcess = nullptr;
        }
        if (!m_translationInputPath.isEmpty()) {
            QFile::remove(m_translationInputPath);
            m_translationInputPath.clear();
        }
        m_activateTranslationWhenFinished = true;
        setTranslationBusyCursor(false);
    }
    void finishTranslation(QProcess *process, const QByteArray &output)
    {
        if (process != m_translationProcess) {
            return;
        }
        if (!output.isEmpty()) {
            const QVector<OcrToken> tokens = tokensFromJsonOutput(output);
            if (!tokens.isEmpty()) {
                m_translationOverlayTokens = tokens;
                m_translatedTokens = selectableTranslationTokens(tokens);
                m_translationActive = m_activateTranslationWhenFinished;
                clearTextSelection();
                updateCursorForPosition(mapFromGlobal(QCursor::pos()));
                update();
            }
        }
        m_translationProcess = nullptr;
        if (!m_translationInputPath.isEmpty()) {
            QFile::remove(m_translationInputPath);
            m_translationInputPath.clear();
        }
        setTranslationBusyCursor(false);
        m_activateTranslationWhenFinished = true;
        process->deleteLater();
    }
    QPointF widgetToImage(QPointF point) const
    {
        if (width() <= 0 || height() <= 0 || m_imageSize.isEmpty()) {
            return {};
        }
        return QPointF(point.x() * static_cast<qreal>(m_imageSize.width()) / static_cast<qreal>(width()),
                       point.y() * static_cast<qreal>(m_imageSize.height()) / static_cast<qreal>(height()));
    }
    QRectF imageToWidget(QRectF imageRect) const
    {
        if (m_imageSize.isEmpty()) {
            return {};
        }
        const qreal sx = static_cast<qreal>(width()) / static_cast<qreal>(m_imageSize.width());
        const qreal sy = static_cast<qreal>(height()) / static_cast<qreal>(m_imageSize.height());
        return QRectF(imageRect.left() * sx,
                      imageRect.top() * sy,
                      imageRect.width() * sx,
                      imageRect.height() * sy);
    }
    std::optional<int> tokenAt(QPointF imagePoint) const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        for (int i = 0; i < tokens.size(); ++i) {
            const QRectF hitRect = tokens.at(i).imageRect.adjusted(-2.0, -2.0, 2.0, 2.0);
            if (hitRect.contains(imagePoint)) {
                return i;
            }
        }
        return std::nullopt;
    }
    std::optional<int> closestToken(QPointF imagePoint) const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        if (tokens.isEmpty()) {
            return std::nullopt;
        }
        int bestIndex = 0;
        qreal bestDistance = std::numeric_limits<qreal>::max();
        for (int i = 0; i < tokens.size(); ++i) {
            const QRectF rect = tokens.at(i).imageRect;
            const qreal dx = imagePoint.x() < rect.left()
                ? rect.left() - imagePoint.x()
                : imagePoint.x() > rect.right() ? imagePoint.x() - rect.right() : 0.0;
            const qreal dy = imagePoint.y() < rect.top()
                ? rect.top() - imagePoint.y()
                : imagePoint.y() > rect.bottom() ? imagePoint.y() - rect.bottom() : 0.0;
            const qreal distance = dx * dx + dy * dy;
            if (distance < bestDistance) {
                bestDistance = distance;
                bestIndex = i;
            }
        }
        return bestIndex;
    }
    void updateCursorForPosition(QPointF widgetPoint)
    {
        if (tokenAt(widgetToImage(widgetPoint))) {
            setCursor(Qt::IBeamCursor);
        } else {
            setCursor(Qt::OpenHandCursor);
        }
    }
    bool hasTextSelection() const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        return m_selectionAnchor >= 0
            && m_selectionFocus >= 0
            && m_selectionAnchor < tokens.size()
            && m_selectionFocus < tokens.size();
    }
    std::pair<int, int> selectionRange() const
    {
        const int first = std::min(m_selectionAnchor, m_selectionFocus);
        const int last = std::max(m_selectionAnchor, m_selectionFocus);
        return {first, last};
    }
    void clearTextSelection()
    {
        if (m_selectionAnchor < 0 && m_selectionFocus < 0) {
            return;
        }
        m_selectionAnchor = -1;
        m_selectionFocus = -1;
        update();
    }
    QString selectedText() const
    {
        if (!hasTextSelection()) {
            return {};
        }
        const auto [first, last] = selectionRange();
        return tokenRangeText(first, last);
    }
    QString allText() const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        if (tokens.isEmpty()) {
            return {};
        }
        return tokenRangeText(0, tokens.size() - 1);
    }
    void copyImageText()
    {
        if (!m_config.ocrEnabled) {
            return;
        }
        if (!activeTokens().isEmpty()) {
            markshot::copyTextToClipboard(allText());
            return;
        }
        m_copyTextAfterOcr = true;
        if (!m_ocrProcess) {
            startOcr();
        }
    }
    QString tokenRangeText(int first, int last) const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        return markshot::ocr::tokenRangeText(sharedOcrTokens(tokens), first, last);
    }
    QVector<markshot::ocr::Token> sharedOcrTokens(const QVector<OcrToken> &tokens) const
    {
        QVector<markshot::ocr::Token> sharedTokens;
        sharedTokens.reserve(tokens.size());
        for (const OcrToken &token : tokens) {
            sharedTokens.append({token.text,
                                 token.imageRect,
                                 token.line,
                                 token.index,
                                 token.confidence});
        }
        return sharedTokens;
    }
    const QVector<OcrToken> &activeTokens() const
    {
        return m_translationActive ? m_translatedTokens : m_ocrTokens;
    }
    bool canRequestTranslation() const
    {
        return m_config.ocrEnabled && !m_translationProcess;
    }
    void requestTranslation()
    {
        if (!canRequestTranslation()) {
            return;
        }
        if (!m_translationOverlayTokens.isEmpty()) {
            setTranslationActive(true);
            return;
        }
        if (m_ocrTokens.isEmpty()) {
            m_translateAfterOcr = true;
            if (!m_ocrProcess) {
                startOcr();
            }
            return;
        }
        m_translateAfterOcr = false;
        startTranslation(true);
    }
    void setTranslationActive(bool active)
    {
        if (active && m_translationOverlayTokens.isEmpty()) {
            return;
        }
        m_translationActive = active;
        clearTextSelection();
        updateCursorForPosition(mapFromGlobal(QCursor::pos()));
        update();
    }
    void setTranslationBusyCursor(bool active)
    {
        if (m_translationBusyCursor == active) {
            return;
        }
        m_translationBusyCursor = active;
        if (active) {
            QApplication::setOverrideCursor(Qt::WaitCursor);
        } else {
            QApplication::restoreOverrideCursor();
            updateCursorForPosition(mapFromGlobal(QCursor::pos()));
        }
    }
    QVector<OcrToken> selectableTranslationTokens(const QVector<OcrToken> &tokens) const
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
    QVector<OcrToken> splitTokenForSelection(const OcrToken &token) const
    {
        QVector<OcrToken> splitTokens;
        if (token.text.size() <= 1) {
            splitTokens.append(token);
            return splitTokens;
        }
        qreal totalWeight = 0.0;
        QVector<qreal> weights;
        weights.reserve(token.text.size());
        for (const QChar ch : token.text) {
            const qreal weight = selectionCharacterWeight(ch);
            weights.append(weight);
            totalWeight += weight;
        }
        if (totalWeight <= 0.0 || token.imageRect.width() <= 0.0) {
            splitTokens.append(token);
            return splitTokens;
        }
        qreal offset = 0.0;
        for (int i = 0; i < token.text.size(); ++i) {
            const qreal nextOffset = offset + token.imageRect.width() * weights.at(i) / totalWeight;
            OcrToken splitToken = token;
            splitToken.text = token.text.mid(i, 1);
            splitToken.imageRect = QRectF(token.imageRect.left() + offset,
                                          token.imageRect.top(),
                                          std::max<qreal>(1.0, nextOffset - offset),
                                          token.imageRect.height());
            splitTokens.append(splitToken);
            offset = nextOffset;
        }
        return splitTokens;
    }
    qreal selectionCharacterWeight(QChar ch) const
    {
        if (ch.isSpace()) {
            return 0.45;
        }
        if (markshot::ocr::isNoLeadingSpacePunctuation(ch)) {
            return 0.65;
        }
        if (ch.unicode() < 0x80) {
            return 0.75;
        }
        return 1.0;
    }
    void drawTranslationOverlay(QPainter &painter) const
    {
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setRenderHint(QPainter::TextAntialiasing, true);
        for (const OcrToken &token : m_translationOverlayTokens) {
            const QRectF textRect = imageToWidget(token.imageRect).adjusted(-3.0, -2.0, 3.0, 2.0);
            if (textRect.isEmpty()) {
                continue;
            }
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(255, 255, 255, 232));
            painter.drawRoundedRect(textRect, 2.0, 2.0);
            QFont font = painter.font();
            const int pixelSize = std::clamp(qRound(textRect.height() * 0.62), 8, 28);
            font.setPixelSize(pixelSize);
            QTextDocument document;
            document.setDefaultFont(font);
            document.setDocumentMargin(1.0);
            document.setTextWidth(textRect.width());
            QTextOption option;
            option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
            document.setDefaultTextOption(option);
            document.setPlainText(token.text);
            painter.save();
            painter.setClipRect(textRect);
            painter.translate(textRect.topLeft());
            document.drawContents(&painter, QRectF(QPointF(0.0, 0.0), textRect.size()));
            painter.restore();
        }
        painter.restore();
    }
    /// @brief Pixmap of the pinned image.
    QPixmap m_pixmap;
    /// @brief Size of the original pinned image.
    QSize m_imageSize;
    /// @brief Device-independent base size used for pinned window scaling.
    QSize m_displayBaseSize;
    /// @brief Current zoom/scale factor of the pinned window.
    qreal m_scale = 1.0;
    /// @brief Offset for dragging the window.
    QPoint m_dragOffset;
    /// @brief Configuration settings for the pinned window.
    PinnedWindowConfig m_config;
    /// @brief OCR tokens recognized from the image.
    QVector<OcrToken> m_ocrTokens;
    /// @brief Translated OCR tokens.
    QVector<OcrToken> m_translatedTokens;
    /// @brief Bounding tokens for the translation overlay.
    QVector<OcrToken> m_translationOverlayTokens;
    /// @brief Subprocess used for OCR.
    QProcess *m_ocrProcess = nullptr;
    /// @brief Subprocess used for text translation.
    QProcess *m_translationProcess = nullptr;
    /// @brief Path to the temporary OCR file.
    QString m_ocrTempPath;
    /// @brief Path to the temporary translation input file.
    QString m_translationInputPath;
    /// @brief Anchor index of the current text selection.
    int m_selectionAnchor = -1;
    /// @brief Focus index of the current text selection.
    int m_selectionFocus = -1;
    /// @brief Flag indicating if the user is currently selecting text.
    bool m_selectingText = false;
    /// @brief Flag indicating if the translation is active.
    bool m_translationActive = false;
    /// @brief Flag indicating if translation should be performed after OCR.
    bool m_translateAfterOcr = false;
    /// @brief Flag indicating if text should be copied to clipboard after OCR.
    bool m_copyTextAfterOcr = false;
    /// @brief Flag indicating if the translation busy cursor is active.
    bool m_translationBusyCursor = false;
    /// @brief Flag indicating if translation should be activated when finished.
    bool m_activateTranslationWhenFinished = true;
    /// @brief Flag indicating if the OCR backend warning has been shown.
    bool m_ocrBackendWarningShown = false;
};
}  // namespace

namespace markshot::shot {

QWidget *createPinnedImageWindow(QImage image)
{
    return new PinnedImageWindow(std::move(image));
}

}  // namespace markshot::shot
