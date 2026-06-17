#include "shot_window_module.h"

#include "app_config_store.h"
#include "debug_log.h"
#include "pinned_window_top.h"

namespace {

using namespace markshot::shot;

/// @brief Window displaying OCR results, allowing reviewing, editing, and translation.
class OcrResultWindow final : public QWidget {
public:
    explicit OcrResultWindow(QString text)
        : m_config(pinnedWindowConfig())
    {
        setWindowTitle(MS_TR("OCR Result"));
        setAttribute(Qt::WA_DeleteOnClose);
        // 置顶状态由配置驱动,构造末尾统一通过 applyPinnedWindowTopState 应用
        setWindowFlags(Qt::Window | Qt::FramelessWindowHint);
        setFocusPolicy(Qt::StrongFocus);
        setMouseTracking(true);
        setObjectName(QStringLiteral("extensionPanel"));
        setStyleSheet(markshot::theme::openWithPanelStyleSheet());

        auto *layout = new QVBoxLayout(this);
        layout->setContentsMargins(10, 10, 10, 10);
        layout->setSpacing(6);

        m_titleBar = new QWidget(this);
        m_titleBar->setCursor(Qt::SizeAllCursor);
        m_titleBar->setMinimumHeight(26);
        auto *titleLayout = new QHBoxLayout(m_titleBar);
        titleLayout->setContentsMargins(0, 0, 0, 0);
        titleLayout->setSpacing(6);
        m_titleLabel = new QLabel(MS_TR("OCR Result"), m_titleBar);
        m_titleLabel->setObjectName(QStringLiteral("ocrResultTitle"));
        m_titleLabel->setAlignment(Qt::AlignVCenter | Qt::AlignLeft);
        m_titleLabel->setCursor(Qt::SizeAllCursor);
        titleLayout->addWidget(m_titleLabel);
        titleLayout->addStretch();

        // 置顶切换按钮:与贴图窗口同款,复用 Pin 工具图标
        m_pinButton = new QPushButton(m_titleBar);
        m_pinButton->setObjectName(QStringLiteral("ocrPinButton"));
        m_pinButton->setCheckable(true);
        m_pinButton->setChecked(m_config.alwaysOnTop);
        m_pinButton->setIcon(markshot::ui::makeToolIcon(ShotWindow::Action::Pin));
        m_pinButton->setIconSize(QSize(16, 16));
        m_pinButton->setFixedSize(24, 24);
        m_pinButton->setStyleSheet(markshot::theme::ocrPinButtonStyleSheet());
        m_pinButton->setToolTip(MS_TR("Always on Top"));
        m_pinButton->setFocusPolicy(Qt::NoFocus);
        connect(m_pinButton, &QPushButton::toggled, this, [this](bool checked) {
            setAlwaysOnTop(checked);
        });
        titleLayout->addWidget(m_pinButton);

        m_titleBar->installEventFilter(this);
        m_titleLabel->installEventFilter(this);
        layout->addWidget(m_titleBar);

        m_editor = new QTextEdit(this);
        m_editor->setObjectName(QStringLiteral("ocrEditor"));
        m_editor->setAcceptRichText(false);
        m_editor->setPlaceholderText(MS_TR("OCR text appears here"));
        m_editor->setStyleSheet(markshot::theme::ocrEditorStyleSheet());
        m_editor->setMinimumHeight(200);
        m_editor->setPlainText(std::move(text));
        m_editor->setContextMenuPolicy(Qt::CustomContextMenu);
        connect(m_editor, &QTextEdit::customContextMenuRequested, this, [this](const QPoint &position) {
            showEditorContextMenu(m_editor->mapToGlobal(position));
        });
        layout->addWidget(m_editor);

        auto *actions = new QHBoxLayout();
        actions->setSpacing(6);
        auto *copyButton = new QPushButton(MS_TR("Copy"), this);
        m_translateButton = new QPushButton(MS_TR("Translate"), this);
        auto *closeButton = new QPushButton(MS_TR("Close"), this);
        for (QPushButton *button : {copyButton, m_translateButton, closeButton}) {
            button->setObjectName(QStringLiteral("ocrPanelButton"));
            button->setStyleSheet(markshot::theme::ocrPanelButtonStyleSheet());
        }
        connect(copyButton, &QPushButton::clicked, this, [this] {
            markshot::copyTextToClipboard(m_editor->toPlainText());
            showToast(MS_TR("OCR text copied"));
        });
        connect(m_translateButton, &QPushButton::clicked, this, [this] {
            startTranslation();
        });
        connect(closeButton, &QPushButton::clicked, this, &QWidget::close);
        actions->addWidget(copyButton);
        actions->addWidget(m_translateButton);
        actions->addStretch();
        actions->addWidget(closeButton);
        layout->addLayout(actions);

        resize(initialWindowSize());
        centerOnPrimaryScreen();
        // 统一应用置顶状态,与贴图窗口共用 pinnedWindow.alwaysOnTop 配置
        applyPinnedWindowTopState(this, m_config.alwaysOnTop);
        m_editor->setFocus(Qt::MouseFocusReason);
    }

    ~OcrResultWindow() override
    {
        cancelTranslation();
    }

protected:
    void mousePressEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton && event->position().y() <= 44.0) {
            beginWindowDrag(event);
            return;
        }
        QWidget::mousePressEvent(event);
    }

    void mouseMoveEvent(QMouseEvent *event) override
    {
        if (updateWindowDrag(event)) {
            return;
        }
        QWidget::mouseMoveEvent(event);
    }

    void mouseReleaseEvent(QMouseEvent *event) override
    {
        if (finishWindowDrag(event)) {
            return;
        }
        QWidget::mouseReleaseEvent(event);
    }

    void keyPressEvent(QKeyEvent *event) override
    {
        if (event->key() == Qt::Key_Escape) {
            close();
            event->accept();
            return;
        }
        QWidget::keyPressEvent(event);
    }

    bool eventFilter(QObject *watched, QEvent *event) override
    {
        if (watched == m_titleBar || watched == m_titleLabel) {
            if (event->type() == QEvent::MouseButtonPress) {
                auto *mouseEvent = static_cast<QMouseEvent *>(event);
                if (mouseEvent->button() == Qt::LeftButton) {
                    return beginWindowDrag(mouseEvent);
                }
            } else if (event->type() == QEvent::MouseMove && m_dragging) {
                return updateWindowDrag(static_cast<QMouseEvent *>(event));
            } else if (event->type() == QEvent::MouseButtonRelease && m_dragging) {
                return finishWindowDrag(static_cast<QMouseEvent *>(event));
            }
        }
        return QWidget::eventFilter(watched, event);
    }

private:
    bool beginWindowDrag(QMouseEvent *event)
    {
        if (!event || event->button() != Qt::LeftButton) {
            return false;
        }

        if (QWindow *window = windowHandle()) {
            if (window->startSystemMove()) {
                event->accept();
                return true;
            }
        }

        m_dragging = true;
        m_dragOffset = event->globalPosition().toPoint() - frameGeometry().topLeft();
        setCursor(Qt::SizeAllCursor);
        grabMouse();
        event->accept();
        return true;
    }

    bool updateWindowDrag(QMouseEvent *event)
    {
        if (!event || !m_dragging) {
            return false;
        }

        move(event->globalPosition().toPoint() - m_dragOffset);
        event->accept();
        return true;
    }

    bool finishWindowDrag(QMouseEvent *event)
    {
        if (!event || event->button() != Qt::LeftButton || !m_dragging) {
            return false;
        }

        m_dragging = false;
        if (QWidget::mouseGrabber() == this) {
            releaseMouse();
        }
        unsetCursor();
        event->accept();
        return true;
    }

    QSize initialWindowSize() const
    {
        QSize size(420, 520);
        if (QScreen *screen = QApplication::primaryScreen()) {
            const QSize available = screen->availableGeometry().size();
            size.setWidth(std::min(size.width(), std::max(320, qRound(available.width() * 0.9))));
            size.setHeight(std::min(size.height(), std::max(260, qRound(available.height() * 0.9))));
        }
        return size;
    }

    void centerOnPrimaryScreen()
    {
        if (QScreen *screen = QApplication::primaryScreen()) {
            move(screen->availableGeometry().center() - rect().center());
        }
    }

    void showToast(const QString &text, int durationMs = 2000)
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
        label->move((width() - label->width()) / 2, height() - label->height() - 24);
        label->show();
        QTimer::singleShot(durationMs, label, &QObject::deleteLater);
    }

    template <typename Callback>
    QAction *addEditorMenuAction(QMenu *menu,
                                 const QString &text,
                                 const QKeySequence &shortcut,
                                 bool enabled,
                                 Callback callback)
    {
        /// @brief The newly created action for the context menu.
        QAction *action = menu->addAction(text, this, callback);
        action->setShortcut(shortcut);
        action->setShortcutVisibleInContextMenu(true);
        action->setEnabled(enabled);
        return action;
    }

    void showEditorContextMenu(const QPoint &globalPosition)
    {
        if (!m_editor) {
            return;
        }

        QMenu menu(this);
        const QTextCursor cursor = m_editor->textCursor();
        const bool readOnly = m_editor->isReadOnly();
        const bool hasSelection = cursor.hasSelection();
        const bool hasDocumentText = !m_editor->document()->isEmpty();
        const bool hasClipboardText = !QApplication::clipboard()->text().isEmpty();

        addEditorMenuAction(&menu,
                            MS_TR("Undo"),
                            QKeySequence::Undo,
                            !readOnly && m_editor->document()->isUndoAvailable(),
                            [this] { m_editor->undo(); });
        addEditorMenuAction(&menu,
                            MS_TR("Redo"),
                            QKeySequence::Redo,
                            !readOnly && m_editor->document()->isRedoAvailable(),
                            [this] { m_editor->redo(); });
        menu.addSeparator();
        addEditorMenuAction(&menu,
                            MS_TR("Cut"),
                            QKeySequence::Cut,
                            !readOnly && hasSelection,
                            [this] { m_editor->cut(); });
        addEditorMenuAction(&menu,
                            MS_TR("Copy"),
                            QKeySequence::Copy,
                            hasSelection,
                            [this] { m_editor->copy(); });
        addEditorMenuAction(&menu,
                            MS_TR("Paste"),
                            QKeySequence::Paste,
                            !readOnly && hasClipboardText,
                            [this] { m_editor->paste(); });
        addEditorMenuAction(&menu,
                            MS_TR("Delete"),
                            QKeySequence(Qt::Key_Delete),
                            !readOnly && hasSelection,
                            [this] {
                                QTextCursor selection = m_editor->textCursor();
                                selection.removeSelectedText();
                                m_editor->setTextCursor(selection);
                            });
        menu.addSeparator();
        addEditorMenuAction(&menu,
                            MS_TR("Select All"),
                            QKeySequence::SelectAll,
                            hasDocumentText,
                            [this] { m_editor->selectAll(); });

        menu.exec(globalPosition);
    }

    void startTranslation()
    {
        if (!m_editor || !m_translateButton || m_translationProcess) {
            return;
        }

        const QString text = m_editor->toPlainText().trimmed();
        if (text.isEmpty()) {
            showToast(MS_TR("No text to translate"));
            return;
        }

        QJsonArray tokens;
        const QStringList lines = text.split(QLatin1Char('\n'));
        int lineIndex = 0;
        for (const QString &rawLine : lines) {
            const QString line = rawLine.trimmed();
            if (line.isEmpty()) {
                ++lineIndex;
                continue;
            }

            QJsonObject object;
            object.insert(QStringLiteral("text"), line);
            object.insert(QStringLiteral("box"),
                          QJsonArray{0, static_cast<double>(lineIndex) * 24.0, 1000.0, 20.0});
            object.insert(QStringLiteral("line"), lineIndex);
            object.insert(QStringLiteral("index"), 0);
            object.insert(QStringLiteral("confidence"), 1.0);
            tokens.append(object);
            ++lineIndex;
        }

        if (tokens.isEmpty()) {
            showToast(MS_TR("No text to translate"));
            return;
        }

        QJsonObject root;
        root.insert(QStringLiteral("targetLanguage"), m_config.translationTargetLanguage);
        root.insert(QStringLiteral("tokens"), tokens);

        QTemporaryFile inputFile(
            QDir(QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
                     ? QDir::tempPath()
                     : QStandardPaths::writableLocation(QStandardPaths::TempLocation))
                .filePath(QStringLiteral("mark-shot-ocr-result-translate-XXXXXX.json")));
        inputFile.setAutoRemove(false);
        if (!inputFile.open()) {
            showToast(MS_TR("Translation failed"));
            return;
        }

        m_translationInputPath = inputFile.fileName();
        inputFile.write(QJsonDocument(root).toJson(QJsonDocument::Compact));
        inputFile.close();

        auto *process = new QProcess(this);
        m_translationProcess = process;
        process->setProcessEnvironment(QProcessEnvironment::systemEnvironment());

        if (!m_config.translationCommand.isEmpty()) {
            QString commandLine = m_config.translationCommand;
            bool replaced = false;
            replaceShellPlaceholder(&commandLine, QStringLiteral("{input}"), m_translationInputPath, &replaced);
            replaceShellPlaceholder(&commandLine, QStringLiteral("{inputPath}"), m_translationInputPath, &replaced);
            replaceShellPlaceholder(&commandLine,
                                    QStringLiteral("{targetLanguage}"),
                                    m_config.translationTargetLanguage,
                                    &replaced);
            replaceShellPlaceholder(&commandLine, QStringLiteral("{config}"), appConfigPath(), &replaced);
            if (!replaced) {
                commandLine += QLatin1Char(' ');
                commandLine += shellQuote(m_translationInputPath);
            }

            markshot::setShellCommand(process, commandLine);
        } else {
            process->setProgram(helperProgramPath(QStringLiteral("mark-shot-translate")));
            process->setArguments({QStringLiteral("--input"),
                                   m_translationInputPath,
                                   QStringLiteral("--target-language"),
                                   m_config.translationTargetLanguage,
                                   QStringLiteral("--config"),
                                   appConfigPath()});
        }

        m_translateButton->setEnabled(false);
        m_translateButton->setText(MS_TR("Translating..."));

        connect(process, &QProcess::errorOccurred, this, [this, process] {
            if (process == m_translationProcess && process->state() == QProcess::NotRunning) {
                finishTranslation(process, QByteArray());
            }
        });
        connect(process,
                QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
                this,
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

        process->start();
    }

    void finishTranslation(QProcess *process, const QByteArray &output)
    {
        if (process != m_translationProcess) {
            return;
        }

        QStringList translatedLines;
        if (!output.isEmpty()) {
            QJsonParseError parseError;
            const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
            if (parseError.error == QJsonParseError::NoError && document.isObject()) {
                const QJsonArray tokenArray =
                    document.object().value(QStringLiteral("tokens")).toArray();
                translatedLines.reserve(tokenArray.size());
                for (const QJsonValue &value : tokenArray) {
                    if (!value.isObject()) {
                        continue;
                    }
                    translatedLines.append(value.toObject()
                                               .value(QStringLiteral("text"))
                                               .toString()
                                               .trimmed());
                }
            }
        }

        const QString translatedText = translatedLines.join(QLatin1Char('\n')).trimmed();
        if (!translatedText.isEmpty() && m_editor) {
            m_editor->setPlainText(translatedText);
        } else {
            showToast(MS_TR("Translation failed"));
        }

        finishTranslationCleanup(process);
    }

    void cancelTranslation()
    {
        if (m_translationProcess) {
            disconnect(m_translationProcess, nullptr, this, nullptr);
            if (m_translationProcess->state() != QProcess::NotRunning) {
                m_translationProcess->kill();
            }
            QProcess *process = m_translationProcess;
            m_translationProcess = nullptr;
            process->deleteLater();
        }

        if (!m_translationInputPath.isEmpty()) {
            QFile::remove(m_translationInputPath);
            m_translationInputPath.clear();
        }
        resetTranslationUi();
    }

    void finishTranslationCleanup(QProcess *process)
    {
        m_translationProcess = nullptr;
        if (!m_translationInputPath.isEmpty()) {
            QFile::remove(m_translationInputPath);
            m_translationInputPath.clear();
        }
        resetTranslationUi();
        process->deleteLater();
    }

    void resetTranslationUi()
    {
        if (m_translateButton) {
            m_translateButton->setEnabled(true);
            m_translateButton->setText(MS_TR("Translate"));
        }
    }

    /// @brief 切换置顶状态,与贴图窗口共用 pinnedWindow.alwaysOnTop 配置。
    /// @param alwaysOnTop 是否保持窗口置顶。
    void setAlwaysOnTop(bool alwaysOnTop)
    {
        if (m_alwaysOnTop == alwaysOnTop) {
            return;
        }

        const bool previous = m_alwaysOnTop;
        m_alwaysOnTop = alwaysOnTop;
        m_config.alwaysOnTop = alwaysOnTop;

        // 1. 持久化到 pinnedWindow.alwaysOnTop,与贴图窗口共享同一开关
        QString error;
        if (!markshot::writeAppConfigValue(
                {QStringLiteral("pinnedWindow"), QStringLiteral("alwaysOnTop")},
                QJsonValue(alwaysOnTop),
                &error)) {
            m_alwaysOnTop = previous;
            m_config.alwaysOnTop = previous;
            if (m_pinButton) {
                QSignalBlocker blocker(m_pinButton);
                m_pinButton->setChecked(previous);
            }
            if (!error.isEmpty()) {
                markshot::debugLog("config",
                                   "cannot save pinnedWindow.alwaysOnTop: %s",
                                   error.toUtf8().constData());
            }
            return;
        }

        // 2. 应用平台置顶状态:Qt hint / layer-shell / GNOME DBus / Windows API 全链路
        applyPinnedWindowTopState(this, alwaysOnTop);
        if (alwaysOnTop) {
            // 3. 置顶时多次延迟抬升,规避合成器时序导致的层级丢失
            for (int delayMs : {0, 80, 250, 600}) {
                QTimer::singleShot(delayMs, this, [this] {
                    if (m_alwaysOnTop) {
                        raisePinnedWindowOnPlatform(this);
                    }
                });
            }
        }
        if (m_pinButton) {
            m_pinButton->setToolTip(alwaysOnTop ? MS_TR("Always on Top: On")
                                                : MS_TR("Always on Top: Off"));
        }
    }

    /// @brief Title bar widget.
    QWidget *m_titleBar = nullptr;
    /// @brief Title label widget.
    QLabel *m_titleLabel = nullptr;
    /// @brief Text editor containing the recognized OCR text.
    QTextEdit *m_editor = nullptr;
    /// @brief Button to trigger text translation.
    QPushButton *m_translateButton = nullptr;
    /// @brief Toggle button controlling always-on-top in the title bar.
    QPushButton *m_pinButton = nullptr;
    /// @brief Subprocess used for running translation tasks.
    QProcess *m_translationProcess = nullptr;
    /// @brief Path to the temporary translation input text file.
    QString m_translationInputPath;
    /// @brief Mouse drag offset vector.
    QPoint m_dragOffset;
    /// @brief Configuration settings for the OCR result window.
    PinnedWindowConfig m_config;
    /// @brief Current always-on-top state mirrored from m_config for fast checks.
    bool m_alwaysOnTop = true;
    /// @brief Flag indicating if the window is currently being dragged.
    bool m_dragging = false;
};



}  // namespace

namespace markshot::shot {

QWidget *createOcrResultWindow(QString text)
{
    return new OcrResultWindow(std::move(text));
}

}  // namespace markshot::shot
