#include "selection_translate/translate_result_bar.h"

#include "ui/i18n.h"
#include "ui/svg_icon.h"
#include "ui/theme.h"

#include <QHBoxLayout>
#include <QLabel>
#include <QMouseEvent>
#include <QPainter>
#include <QPushButton>

namespace markshot::selection_translate {

namespace {

/**
 * 创建浮条上的文字按钮。
 * @param text 按钮文字。
 * @param objectName 按钮对象名，便于测试和样式定位。
 * @param parent 父控件。
 * @return 新建的按钮。
 */
QPushButton *createBarButton(const QString &text, const QString &objectName, QWidget *parent)
{
    auto *button = new QPushButton(text, parent);
    button->setObjectName(objectName);
    button->setFocusPolicy(Qt::NoFocus);
    button->setCursor(Qt::PointingHandCursor);
    button->setIconSize(QSize(13, 13));
    return button;
}

}  // namespace

TranslateResultBar::TranslateResultBar(const markshot::ui::ToolbarPalette &palette, QWidget *parent)
    : QWidget(parent),
      m_palette(palette)
{
    setObjectName(QStringLiteral("translateResultBar"));
    setAttribute(Qt::WA_TranslucentBackground, true);
    setAttribute(Qt::WA_NoSystemBackground, true);
    setCursor(Qt::ArrowCursor);
    setFont(markshot::theme::uiFont(10));

    // 1. 左侧用翻译图标加语言名说明目标语言，右侧两个操作按钮
    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(8, 3, 3, 3);
    layout->setSpacing(2);
    auto *icon = new QLabel(this);
    icon->setPixmap(markshot::ui::makeSvgIcon(QStringLiteral("translate"), palette.mutedText).pixmap(QSize(13, 13)));
    layout->addWidget(icon);
    layout->addSpacing(2);
    m_label = new QLabel(this);
    m_label->setObjectName(QStringLiteral("translateLanguageLabel"));
    layout->addWidget(m_label);
    layout->addSpacing(4);
    m_copyButton = createBarButton(MS_TR("Copy translation"), QStringLiteral("translateCopyButton"), this);
    m_restoreButton = createBarButton(MS_TR("Restore original"), QStringLiteral("translateRestoreButton"), this);
    layout->addWidget(m_copyButton);
    layout->addWidget(m_restoreButton);
    connect(m_copyButton, &QPushButton::clicked, this, &TranslateResultBar::copyRequested);
    connect(m_restoreButton, &QPushButton::clicked, this, &TranslateResultBar::restoreRequested);

    // 2. 按钮与打开方式面板的列表项一致：平时透明，悬停浅色底
    using markshot::ui::cssColor;
    setStyleSheet(QStringLiteral(
        "QLabel { color: %1; background: transparent; }"
        "QPushButton {"
        " color: %2;"
        " background: transparent;"
        " border: 1px solid transparent;"
        " border-radius: 6px;"
        " padding: 2px 6px;"
        "}"
        "QPushButton:hover { background: %3; border-color: %4; }"
        "QPushButton:pressed { background: %4; }")
        .arg(cssColor(palette.mutedText), cssColor(palette.text), cssColor(palette.softHoverFill),
             cssColor(palette.softHoverBorder)));
    // 3. 图标跟随文字颜色，亮色与暗色主题下都清晰
    m_copyButton->setIcon(markshot::ui::makeSvgIcon(QStringLiteral("copy"), palette.text));
    m_restoreButton->setIcon(markshot::ui::makeSvgIcon(QStringLiteral("undo"), palette.text));
    hide();
}

void TranslateResultBar::setLanguageLabel(const QString &languageLabel)
{
    m_label->setText(languageLabel);
    setToolTip(MS_TR("Translated to %1").arg(languageLabel));
    adjustSize();
}

void TranslateResultBar::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event);
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(m_palette.panelBorder, 1.0));
    painter.setBrush(m_palette.panelFill);
    painter.drawRoundedRect(QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5), 8.0, 8.0);
}

void TranslateResultBar::mousePressEvent(QMouseEvent *event)
{
    event->accept();
}

void TranslateResultBar::mouseReleaseEvent(QMouseEvent *event)
{
    event->accept();
}

}  // namespace markshot::selection_translate
