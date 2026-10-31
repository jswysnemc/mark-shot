#include "selection_translate/translate_result_bar.h"

#include "ui/i18n.h"
#include "ui/svg_icon.h"
#include "ui/theme.h"

#include <QButtonGroup>
#include <QHBoxLayout>
#include <QLabel>
#include <QMouseEvent>
#include <QPainter>
#include <QPushButton>

namespace markshot::selection_translate {

namespace {

/**
 * 创建浮条上的按钮。
 * @param text 按钮文字，纯图标按钮传空字符串。
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

    // 1. 左侧翻译图标加目标语言按钮，点击后打开语言列表
    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(8, 3, 3, 3);
    layout->setSpacing(2);
    auto *icon = new QLabel(this);
    icon->setPixmap(markshot::ui::makeSvgIcon(QStringLiteral("translate"), palette.mutedText).pixmap(QSize(13, 13)));
    layout->addWidget(icon);
    m_languageButton = createBarButton(QString(), QStringLiteral("translateLanguageButton"), this);
    m_languageButton->setLayoutDirection(Qt::RightToLeft);
    m_languageButton->setIcon(markshot::ui::makeSvgIcon(QStringLiteral("chevron-down"), palette.mutedText));
    m_languageButton->setToolTip(MS_TR("Translate to"));
    layout->addWidget(m_languageButton);
    layout->addSpacing(4);

    // 2. 原文与译文两个互斥按钮，只切换显示，不退出翻译模式
    m_originalButton = createBarButton(MS_TR("Original"), QStringLiteral("translateOriginalButton"), this);
    m_translatedButton = createBarButton(MS_TR("Translated"), QStringLiteral("translateTranslatedButton"), this);
    auto *viewGroup = new QButtonGroup(this);
    for (QPushButton *button : {m_originalButton, m_translatedButton}) {
        button->setCheckable(true);
        viewGroup->addButton(button);
        layout->addWidget(button);
    }
    m_translatedButton->setChecked(true);
    layout->addSpacing(4);

    // 3. 右侧复制与退出两个图标按钮
    auto *copyButton = createBarButton(QString(), QStringLiteral("translateCopyButton"), this);
    copyButton->setToolTip(MS_TR("Copy translation"));
    auto *exitButton = createBarButton(QString(), QStringLiteral("translateExitButton"), this);
    exitButton->setToolTip(MS_TR("Exit translation"));
    layout->addWidget(copyButton);
    layout->addWidget(exitButton);
    connect(m_languageButton, &QPushButton::clicked, this, &TranslateResultBar::languageRequested);
    connect(m_originalButton, &QPushButton::clicked, this, [this] { emit viewChanged(false); });
    connect(m_translatedButton, &QPushButton::clicked, this, [this] { emit viewChanged(true); });
    connect(copyButton, &QPushButton::clicked, this, &TranslateResultBar::copyRequested);
    connect(exitButton, &QPushButton::clicked, this, &TranslateResultBar::exitRequested);

    // 4. 按钮平时透明、悬停浅色底；当前显示的原文或译文按钮用选中色
    using markshot::ui::cssColor;
    setStyleSheet(QStringLiteral(
        "QPushButton {"
        " color: %1;"
        " background: transparent;"
        " border: 1px solid transparent;"
        " border-radius: 6px;"
        " padding: 2px 6px;"
        "}"
        "QPushButton:hover { background: %2; border-color: %3; }"
        "QPushButton:pressed { background: %3; }"
        "QPushButton:checked { color: %4; background: %5; border-color: %6; }")
        .arg(cssColor(palette.text), cssColor(palette.softHoverFill), cssColor(palette.softHoverBorder),
             cssColor(palette.strongText), cssColor(palette.activeFill), cssColor(palette.activeBorder)));
    // 5. 图标跟随文字颜色，亮色与暗色主题下都清晰
    copyButton->setIcon(markshot::ui::makeSvgIcon(QStringLiteral("copy"), palette.text));
    exitButton->setIcon(markshot::ui::makeSvgIcon(QStringLiteral("cancel"), palette.text));
    hide();
}

void TranslateResultBar::setLanguageLabel(const QString &languageLabel)
{
    m_languageButton->setText(languageLabel);
    adjustSize();
}

void TranslateResultBar::setShowingTranslation(bool showingTranslation)
{
    (showingTranslation ? m_translatedButton : m_originalButton)->setChecked(true);
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
