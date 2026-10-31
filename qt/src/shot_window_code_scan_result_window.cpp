#include "shot_window_module.h"
#include "ui/toolbar_panel_style.h"

namespace {

/**
 * 显示二维码和条形码识别结果的窗口。
 */
class CodeScanResultWindow final : public QWidget {
public:
    explicit CodeScanResultWindow(QString text)
    {
        setWindowTitle(MS_TR("Code Scan Result"));
        setAttribute(Qt::WA_DeleteOnClose);
        setWindowFlags(Qt::Window | Qt::WindowStaysOnTopHint);
        setObjectName(QStringLiteral("extensionPanel"));
        setStyleSheet(markshot::theme::openWithPanelStyleSheet());

        auto *layout = new QVBoxLayout(this);
        layout->setContentsMargins(12, 12, 12, 12);
        layout->setSpacing(10);

        auto *titleLabel = new QLabel(MS_TR("Code Scan Result"), this);
        titleLabel->setObjectName(QStringLiteral("ocrResultTitle"));
        titleLabel->setFont(markshot::theme::uiFont(13, QFont::DemiBold));
        layout->addWidget(titleLabel);

        auto *hintLabel = new QLabel(MS_TR("Review the recognized QR code or barcode content before copying."), this);
        hintLabel->setWordWrap(true);
        layout->addWidget(hintLabel);

        m_editor = new QTextEdit(this);
        m_editor->setAcceptRichText(false);
        m_editor->setMinimumSize(420, 220);
        m_editor->setPlainText(std::move(text));
        layout->addWidget(m_editor);

        auto *actions = new QHBoxLayout();
        actions->setSpacing(8);
        auto *copyButton = new QPushButton(MS_TR("Copy"), this);
        auto *closeButton = new QPushButton(MS_TR("Close"), this);
        for (QPushButton *button : {copyButton, closeButton}) {
            button->setObjectName(QStringLiteral("ocrPanelButton"));
            button->setStyleSheet(markshot::theme::ocrPanelButtonStyleSheet());
        }
        connect(copyButton, &QPushButton::clicked, this, [this] {
            markshot::copyTextToClipboard(m_editor->toPlainText());
        });
        connect(closeButton, &QPushButton::clicked, this, &QWidget::close);
        actions->addWidget(copyButton);
        actions->addWidget(closeButton);
        layout->addLayout(actions);

        resize(sizeHint());
        if (QScreen *primary = QGuiApplication::primaryScreen()) {
            const QRect geometry = primary->availableGeometry();
            move(geometry.center() - rect().center());
        }
        m_editor->setFocus(Qt::MouseFocusReason);
    }

protected:
    /**
     * 处理窗口键盘输入。
     * @param event 键盘事件。
     * @return 无返回值。
     */
    void keyPressEvent(QKeyEvent *event) override
    {
        if (event->key() == Qt::Key_Escape) {
            close();
            event->accept();
            return;
        }
        QWidget::keyPressEvent(event);
    }

private:
    QTextEdit *m_editor = nullptr;
};

}  // namespace

namespace markshot::shot {

/**
 * 创建扫码结果窗口。
 * @param text 要展示的识别结果文本。
 * @return 新建的扫码结果窗口。
 */
QWidget *createCodeScanResultWindow(QString text)
{
    return new CodeScanResultWindow(std::move(text));
}

void placeCodeScanResultWindow(QWidget *window, QScreen *screen)
{
    if (!window) {
        return;
    }
    // 1. 优先放到选区所在屏幕，避免多屏时结果总是出现在主屏
    QScreen *target = screen;
    if (!target) {
        target = QGuiApplication::screenAt(QCursor::pos());
    }
    if (!target) {
        target = QGuiApplication::primaryScreen();
    }
    if (!target) {
        return;
    }
    const QRect available = target->availableGeometry();
    window->move(available.center() - window->rect().center());
}

}  // namespace markshot::shot
