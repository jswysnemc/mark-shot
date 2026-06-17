#include "ocr_result_window.h"

#include "app_config_store.h"
#include "clipboard_image.h"
#include "debug_log.h"
#include "pinned_window_top.h"
#include "shot_window.h"
#include "ui/i18n.h"
#include "ui/icons.h"
#include "ui/theme.h"

#include <QAction>
#include <QApplication>
#include <QBoxLayout>
#include <QClipboard>
#include <QComboBox>
#include <QEvent>
#include <QIcon>
#include <QKeyEvent>
#include <QKeySequence>
#include <QLabel>
#include <QMenu>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPen>
#include <QPixmap>
#include <QPushButton>
#include <QScreen>
#include <QSignalBlocker>
#include <QTextCursor>
#include <QTextDocument>
#include <QTextEdit>
#include <QTimer>
#include <QWindow>

#include <algorithm>

namespace markshot::shot {

namespace {

/// @brief 创建目标语言下拉按钮图标。
/// @return 下拉箭头图标。
QIcon makeTargetLanguagePopupIcon()
{
    QPixmap pixmap(24, 24);
    pixmap.fill(Qt::transparent);

    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(QColor(94, 234, 212), 2.2, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));

    QPainterPath arrow;
    arrow.moveTo(7.5, 9.0);
    arrow.lineTo(12.0, 14.0);
    arrow.lineTo(16.5, 9.0);
    painter.drawPath(arrow);

    return QIcon(pixmap);
}

template <typename Callback>
QAction *addEditorMenuAction(QWidget *owner,
                             QMenu *menu,
                             const QString &text,
                             const QKeySequence &shortcut,
                             bool enabled,
                             Callback callback)
{
    QAction *action = menu->addAction(text, owner, callback);
    action->setShortcut(shortcut);
    action->setShortcutVisibleInContextMenu(true);
    action->setEnabled(enabled);
    return action;
}

}  // namespace

OcrResultWindow::OcrResultWindow(QString text)
    : m_config(pinnedWindowConfig())
{
    setWindowTitle(MS_TR("OCR Result"));
    setAttribute(Qt::WA_DeleteOnClose);
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

    auto *languageLabel = new QLabel(MS_TR("Target Language"), m_titleBar);
    languageLabel->setObjectName(QStringLiteral("ocrLanguageLabel"));
    languageLabel->setStyleSheet(QStringLiteral(
        "QLabel#ocrLanguageLabel {"
        " color: #9CA3AF;"
        " font-size: 11px;"
        " font-weight: 600;"
        "}"));
    titleLayout->addWidget(languageLabel);

    auto *languageControl = new QWidget(m_titleBar);
    languageControl->setObjectName(QStringLiteral("ocrLanguageControl"));
    languageControl->setFixedHeight(30);
    languageControl->setStyleSheet(QStringLiteral(
        "QWidget#ocrLanguageControl {"
        " background: rgba(255, 255, 255, 12);"
        " border: 1px solid rgba(45, 212, 191, 70);"
        " border-radius: 8px;"
        "}"
        "QWidget#ocrLanguageControl:hover {"
        " border-color: rgba(45, 212, 191, 120);"
        "}"));
    auto *languageControlLayout = new QHBoxLayout(languageControl);
    languageControlLayout->setContentsMargins(0, 0, 0, 0);
    languageControlLayout->setSpacing(0);

    m_targetLanguageCombo = new QComboBox(languageControl);
    setupTargetLanguageCombo();
    languageControlLayout->addWidget(m_targetLanguageCombo);

    m_targetLanguagePopupButton = new QPushButton(languageControl);
    m_targetLanguagePopupButton->setObjectName(QStringLiteral("ocrLanguagePopupButton"));
    m_targetLanguagePopupButton->setFixedSize(28, 28);
    m_targetLanguagePopupButton->setIcon(makeTargetLanguagePopupIcon());
    m_targetLanguagePopupButton->setIconSize(QSize(18, 18));
    m_targetLanguagePopupButton->setFocusPolicy(Qt::NoFocus);
    m_targetLanguagePopupButton->setToolTip(MS_TR("Target Language"));
    m_targetLanguagePopupButton->setStyleSheet(QStringLiteral(
        "QPushButton#ocrLanguagePopupButton {"
        " background: transparent;"
        " border: 0;"
        " border-radius: 7px;"
        " padding: 0;"
        "}"
        "QPushButton#ocrLanguagePopupButton:hover {"
        " background: rgba(45, 212, 191, 22);"
        "}"
        "QPushButton#ocrLanguagePopupButton:pressed {"
        " background: rgba(45, 212, 191, 38);"
        "}"));
    connect(m_targetLanguagePopupButton, &QPushButton::clicked, this, [this] {
        if (m_targetLanguageCombo) {
            m_targetLanguageCombo->setFocus(Qt::MouseFocusReason);
            m_targetLanguageCombo->showPopup();
        }
    });
    languageControlLayout->addWidget(m_targetLanguagePopupButton);
    titleLayout->addWidget(languageControl);

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
    applyPinnedWindowTopState(this, m_config.alwaysOnTop);
    m_editor->setFocus(Qt::MouseFocusReason);
}

OcrResultWindow::~OcrResultWindow()
{
    cancelTranslation();
}

void OcrResultWindow::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton
        && event->position().y() <= 44.0
        && !titleControlContains(event->position().toPoint())) {
        beginWindowDrag(event);
        return;
    }
    QWidget::mousePressEvent(event);
}

void OcrResultWindow::mouseMoveEvent(QMouseEvent *event)
{
    if (updateWindowDrag(event)) {
        return;
    }
    QWidget::mouseMoveEvent(event);
}

void OcrResultWindow::mouseReleaseEvent(QMouseEvent *event)
{
    if (finishWindowDrag(event)) {
        return;
    }
    QWidget::mouseReleaseEvent(event);
}

void OcrResultWindow::keyPressEvent(QKeyEvent *event)
{
    if (event->key() == Qt::Key_Escape) {
        close();
        event->accept();
        return;
    }
    QWidget::keyPressEvent(event);
}

bool OcrResultWindow::eventFilter(QObject *watched, QEvent *event)
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

bool OcrResultWindow::titleControlContains(QPoint windowPoint) const
{
    const QVector<QWidget *> controls = {m_targetLanguageCombo, m_targetLanguagePopupButton, m_pinButton};
    for (QWidget *control : controls) {
        if (!control || !control->isVisible()) {
            continue;
        }
        const QRect controlRect(control->mapTo(const_cast<OcrResultWindow *>(this), QPoint(0, 0)),
                                control->size());
        if (controlRect.contains(windowPoint)) {
            return true;
        }
    }
    return false;
}

bool OcrResultWindow::beginWindowDrag(QMouseEvent *event)
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

bool OcrResultWindow::updateWindowDrag(QMouseEvent *event)
{
    if (!event || !m_dragging) {
        return false;
    }

    move(event->globalPosition().toPoint() - m_dragOffset);
    event->accept();
    return true;
}

bool OcrResultWindow::finishWindowDrag(QMouseEvent *event)
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

QSize OcrResultWindow::initialWindowSize() const
{
    QSize size(420, 520);
    if (QScreen *screen = QApplication::primaryScreen()) {
        const QSize available = screen->availableGeometry().size();
        size.setWidth(std::min(size.width(), std::max(320, qRound(available.width() * 0.9))));
        size.setHeight(std::min(size.height(), std::max(260, qRound(available.height() * 0.9))));
    }
    return size;
}

void OcrResultWindow::centerOnPrimaryScreen()
{
    if (QScreen *screen = QApplication::primaryScreen()) {
        move(screen->availableGeometry().center() - rect().center());
    }
}

void OcrResultWindow::showToast(const QString &text, int durationMs)
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

void OcrResultWindow::showEditorContextMenu(const QPoint &globalPosition)
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

    addEditorMenuAction(this, &menu, MS_TR("Undo"), QKeySequence::Undo,
                        !readOnly && m_editor->document()->isUndoAvailable(),
                        [this] { m_editor->undo(); });
    addEditorMenuAction(this, &menu, MS_TR("Redo"), QKeySequence::Redo,
                        !readOnly && m_editor->document()->isRedoAvailable(),
                        [this] { m_editor->redo(); });
    menu.addSeparator();
    addEditorMenuAction(this, &menu, MS_TR("Cut"), QKeySequence::Cut,
                        !readOnly && hasSelection,
                        [this] { m_editor->cut(); });
    addEditorMenuAction(this, &menu, MS_TR("Copy"), QKeySequence::Copy,
                        hasSelection,
                        [this] { m_editor->copy(); });
    addEditorMenuAction(this, &menu, MS_TR("Paste"), QKeySequence::Paste,
                        !readOnly && hasClipboardText,
                        [this] { m_editor->paste(); });
    addEditorMenuAction(this, &menu, MS_TR("Delete"), QKeySequence(Qt::Key_Delete),
                        !readOnly && hasSelection,
                        [this] {
                            QTextCursor selection = m_editor->textCursor();
                            selection.removeSelectedText();
                            m_editor->setTextCursor(selection);
                        });
    menu.addSeparator();
    addEditorMenuAction(this, &menu, MS_TR("Select All"), QKeySequence::SelectAll,
                        hasDocumentText,
                        [this] { m_editor->selectAll(); });

    menu.exec(globalPosition);
}

void OcrResultWindow::setAlwaysOnTop(bool alwaysOnTop)
{
    if (m_alwaysOnTop == alwaysOnTop) {
        return;
    }

    const bool previous = m_alwaysOnTop;
    m_alwaysOnTop = alwaysOnTop;
    m_config.alwaysOnTop = alwaysOnTop;

    QString error;
    if (!markshot::writeAppConfigValue({QStringLiteral("pinnedWindow"), QStringLiteral("alwaysOnTop")},
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

    applyPinnedWindowTopState(this, alwaysOnTop);
    if (alwaysOnTop) {
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

}  // namespace markshot::shot
