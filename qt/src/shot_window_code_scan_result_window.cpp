#include "shot_window_module.h"
#include "ui/toolbar_panel_style.h"

namespace {

/**
 * 【扫码】【主题】生成扫码结果窗口的输入框与按钮样式，暗色与原 OCR 按钮配色一致。
 * @param palette 截图会话的工具栏配色。
 * @return 追加在面板样式之后的样式表文本。
 */
QString codeScanResultStyleSheet(const markshot::ui::ToolbarPalette &palette)
{
    using markshot::ui::cssColor;
    return QStringLiteral(
               "QTextEdit {"
               " color: %1;"
               " background: %2;"
               " border: 1px solid %3;"
               " border-radius: 8px;"
               " padding: 6px 8px;"
               " selection-background-color: %4;"
               "}"
               "QTextEdit:focus { border-color: %5; }"
               "QPushButton#ocrPanelButton {"
               " color: %6;"
               " text-align: center;"
               " background: transparent;"
               " border: 1px solid transparent;"
               " border-radius: 8px;"
               " padding: 8px 10px;"
               " min-height: 22px;"
               "}"
               "QPushButton#ocrPanelButton:hover { background: %7; border-color: %8; }")
        .arg(cssColor(palette.strongText), cssColor(palette.controlFill), cssColor(palette.controlBorder),
             cssColor(palette.selectionFill), cssColor(palette.focusBorder), cssColor(palette.text),
             cssColor(palette.softHoverFill), cssColor(palette.softHoverBorder));
}

/**
 * 显示二维码和条形码识别结果的窗口。
 */
class CodeScanResultWindow final : public QWidget {
public:
    /**
     * 创建扫码结果窗口。
     * @param text 识别结果文本。
     * @param palette 截图会话的工具栏配色，窗口外框、输入框与按钮跟随同一主题。
     */
    CodeScanResultWindow(QString text, const markshot::ui::ToolbarPalette &palette)
    {
        setWindowTitle(MS_TR("Code Scan Result"));
        setAttribute(Qt::WA_DeleteOnClose);
        setWindowFlags(Qt::Window | Qt::WindowStaysOnTopHint);
        setObjectName(QStringLiteral("extensionPanel"));
        setStyleSheet(markshot::theme::openWithPanelStyleSheet(palette) + codeScanResultStyleSheet(palette));

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
 * @param palette 截图会话的工具栏配色。
 * @return 新建的扫码结果窗口。
 */
QWidget *createCodeScanResultWindow(QString text, const markshot::ui::ToolbarPalette &palette)
{
    return new CodeScanResultWindow(std::move(text), palette);
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
