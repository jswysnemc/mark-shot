#include "settings/settings_dialog.h"

#include "app_config_store.h"
#include "settings/application_theme.h"
#include "settings/settings_design_tokens.h"
#include "settings/settings_field_texts.h"
#include "settings/settings_inline_icons.h"
#include "settings/settings_navigation.h"
#include "settings/settings_saved_notifier.h"
#include "settings/settings_wheel_guard.h"
#include "ui/i18n.h"
#include "ui/application_icon.h"
#include "ui/icons.h"
#include "ui/theme.h"
#include "ui/interface_theme_config.h"

#include <QFrame>
#include <QHBoxLayout>
#include <QJsonObject>
#include <QLabel>
#include <QCheckBox>
#include <QComboBox>
#include <QSpinBox>
#include <QDoubleSpinBox>
#include <QKeySequenceEdit>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QResizeEvent>
#include <QPointer>
#include <QPushButton>
#include <QScreen>
#include <QGuiApplication>
#include <QScrollArea>
#include <QStackedWidget>
#include <QStyle>
#include <QVBoxLayout>

// ===== 新增 import =====
#include <QPainter>

namespace markshot::settings {
namespace {

/// @brief 【设置】【按需创建】添加空滚动容器，内容在首次访问时创建。
/// @param stack 目标堆叠控件。
/// @return 无返回值。
void addScrollablePage(QStackedWidget *stack)
{
    auto *area = new QScrollArea(stack);
    area->setFrameShape(QFrame::NoFrame);
    area->setWidgetResizable(true);
    stack->addWidget(area);
}

/**
 * 读取配置中的设置界面主题。
 * @return 配置的界面主题模式。
 */
markshot::ui::UiThemeMode configuredSettingsThemeMode()
{
    bool ok = false;
    const QJsonObject root = markshot::readAppConfigRoot(&ok);
    return ok ? markshot::ui::uiThemeModeFromConfigRoot(root)
              : markshot::ui::UiThemeMode::System;
}

}  // namespace

SettingsDialog::SettingsDialog(QWidget *parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("settingsDialog"));
    setWindowTitle(MS_TR("Settings"));
    setWindowIcon(markshot::ui::applicationIcon());
    setFont(markshot::theme::uiFont(10));
    setMinimumSize(420, 400);
    resize(860, 620);

    applyTheme(configuredSettingsThemeMode());

    // 滚轮防护：未聚焦的下拉框/数值框不再被滚轮误改内容，页面照常滚动。
    installSettingsWheelGuard(this);

    auto *rootLayout = new QVBoxLayout(this);
    rootLayout->setContentsMargins(0, 0, 0, 0);
    rootLayout->setSpacing(0);
    applySettingsWindowBorderMarginIfEnabled(rootLayout);

    auto *body = new QWidget(this);
    auto *bodyLayout = new QHBoxLayout(body);
    m_bodyLayout = bodyLayout;
    bodyLayout->setContentsMargins(0, 0, 0, 0);
    bodyLayout->setSpacing(0);

    // 侧栏导航：标题区 + 分组分类列表
    m_navigation = new SettingsNavigation(body);
    bodyLayout->addWidget(m_navigation);

    // 1. 【设置】【按需创建】保留全部导航位置，未访问页面只分配空滚动容器
    m_stack = new QStackedWidget(body);
    for (int index = 0; index < 11; ++index) addScrollablePage(m_stack);
    bodyLayout->addWidget(m_stack, 1);
    rootLayout->addWidget(body, 1);

    auto *footer = new QFrame(this);
    footer->setObjectName(QStringLiteral("settingsFooter"));
    auto *footerLayout = new QHBoxLayout(footer);
    footerLayout->setContentsMargins(16, 10, 16, 10);
    footerLayout->setSpacing(8);
    // 1. 【设置】【撤销入口】标准次级按钮加恢复图标，与纯文本的关闭按钮区分
    m_revertButton = new QPushButton(texts::revertUnsaved(), footer);
    m_revertButton->setObjectName(QStringLiteral("settingsRevert"));
    m_revertButton->setProperty("role", QStringLiteral("secondary"));
    m_revertButton->setIconSize(QSize(16, 16));
    m_revertButton->setCursor(Qt::PointingHandCursor);
    m_revertButton->setAutoDefault(false);
    m_revertButton->setEnabled(false);
    footerLayout->addWidget(m_revertButton);
    m_statusLabel = new QLabel(footer);
    m_statusLabel->setObjectName(QStringLiteral("settingsStatus"));
    m_statusLabel->setWordWrap(true);
    m_statusLabel->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred);
    footerLayout->addWidget(m_statusLabel, 1);
    auto *closeButton = new QPushButton(MS_TR("Close"), footer);
    closeButton->setProperty("role", QStringLiteral("quiet"));
    closeButton->setAutoDefault(false);
    footerLayout->addWidget(closeButton);
    m_saveButton = new QPushButton(MS_TR("Save"), footer);
    m_saveButton->setObjectName(QStringLiteral("settingsSave"));
    m_saveButton->setProperty("role", QStringLiteral("primary"));
    m_saveButton->setCursor(Qt::PointingHandCursor);
    // 2. 【设置】【键盘保存】青色主按钮，无修改时禁用；输入框中的 Enter 只执行保存
    m_saveButton->setDefault(true);
    m_saveButton->setEnabled(false);
    footerLayout->addWidget(m_saveButton);
    rootLayout->addWidget(footer);

    // 1. 【设置】【导航】切换分类时保留各页尚未保存的输入
    connect(m_navigation, &SettingsNavigation::navigationChanged, m_stack, &QStackedWidget::setCurrentIndex);
    // 2. 【设置】【保存与撤销】反馈保留在操作栏，撤销回到最近一次保存的配置
    connect(m_saveButton, &QPushButton::clicked, this, &SettingsDialog::saveConfig);
    connect(m_revertButton, &QPushButton::clicked, this, &SettingsDialog::revertChanges);
    connect(closeButton, &QPushButton::clicked, this, &QDialog::close);

    connect(m_stack, &QStackedWidget::currentChanged, this, &SettingsDialog::ensureDeferredPage);
    loadConfig();
    ensureDeferredPage(0);
    m_navigation->setCurrentLogicalRow(0);
}

void SettingsDialog::loadConfig()
{
    QString error;
    m_config = readSettingsConfig(&error);
    applyConfigToPages(m_config);
    if (!error.isEmpty()) {
        setStatus(error, true);
    }
    applyTheme(m_config.general.uiThemeMode);
}

void SettingsDialog::saveConfig()
{
    SettingsConfig nextConfig = collectConfig();
    QString error;
    if (!writeSettingsConfig(nextConfig, &error)) {
        setStatus(MS_TR("Failed to save settings: %1").arg(error), true);
        return;
    }

    m_config = nextConfig;
    // 1. 【设置】【保存基线】撤销修改恢复到刚保存的值
    applyConfigToPages(nextConfig);
    applyTheme(m_config.general.uiThemeMode);
    // 2. 【设置】【保存通知】通知常驻组件重新读取配置，例如托盘全局快捷键
    SettingsSavedNotifier::instance().notifySaved();
    setStatus(MS_TR("Changes saved"));
    m_statusLabel->setToolTip(MS_TR("Some changes take effect after restarting Mark Shot."));
}

void SettingsDialog::applyTheme(markshot::ui::UiThemeMode mode)
{
    const markshot::ui::UiThemeMode effectiveMode = markshot::ui::effectiveUiThemeMode(mode);
    const QPalette palette = applyApplicationTheme(effectiveMode);
    setStyleSheet(tokens::settingsStyleSheet(effectiveMode));
    rememberSettingsWindowBorderColorIfEnabled(effectiveMode);
    // 1. 【设置】【撤销入口】恢复图标跟随主题文本色，禁用态由图标自动降低透明度
    if (m_revertButton) {
        m_revertButton->setIcon(settingsGlyphIcon(SettingsGlyph::Reset, palette.color(QPalette::ButtonText)));
    }
}

void SettingsDialog::resizeEvent(QResizeEvent *event)
{
    QDialog::resizeEvent(event);
    if (m_navigation && m_bodyLayout) {
        const bool compact = width() < 720;
        m_navigation->setCompact(compact);
        m_bodyLayout->setDirection(compact ? QBoxLayout::TopToBottom : QBoxLayout::LeftToRight);
    }
}

void SettingsDialog::trackSettingChanges(QWidget *root)
{
    if (!root) root = m_stack;
    for (QLineEdit *edit : root->findChildren<QLineEdit *>()) {
        if (!edit->isReadOnly()) {
            connect(edit, &QLineEdit::textChanged, this, &SettingsDialog::markSettingsChanged);
        }
    }
    for (QPlainTextEdit *edit : root->findChildren<QPlainTextEdit *>()) {
        if (!edit->isReadOnly()) {
            connect(edit, &QPlainTextEdit::textChanged, this, &SettingsDialog::markSettingsChanged);
        }
    }
    for (QComboBox *combo : root->findChildren<QComboBox *>()) {
        connect(combo, &QComboBox::currentIndexChanged, this, &SettingsDialog::markSettingsChanged);
    }
    for (QCheckBox *box : root->findChildren<QCheckBox *>()) {
        connect(box, &QCheckBox::toggled, this, &SettingsDialog::markSettingsChanged);
    }
    for (QSpinBox *spin : root->findChildren<QSpinBox *>()) {
        connect(spin, &QSpinBox::valueChanged, this, &SettingsDialog::markSettingsChanged);
    }
    for (QDoubleSpinBox *spin : root->findChildren<QDoubleSpinBox *>()) {
        connect(spin, &QDoubleSpinBox::valueChanged, this, &SettingsDialog::markSettingsChanged);
    }
    for (QKeySequenceEdit *edit : root->findChildren<QKeySequenceEdit *>()) {
        connect(edit, &QKeySequenceEdit::keySequenceChanged, this, &SettingsDialog::markSettingsChanged);
    }
    for (QPushButton *button : root->findChildren<QPushButton *>()) {
        connect(button, &QPushButton::clicked, this, &SettingsDialog::markSettingsChanged);
    }
}

void SettingsDialog::markSettingsChanged()
{
    if (m_applyingConfig) {
        return;
    }
    // 1. 【设置】【变更反馈】仅比较可保存的配置值，排除下载和按钮文案等独立状态
    const bool wasModified = m_saveButton->isEnabled();
    const bool modified = settingsConfigToJson(collectConfig()) != m_savedValues;
    m_revertButton->setEnabled(modified);
    m_saveButton->setEnabled(modified);
    if (modified) {
        setStatus(MS_TR("Unsaved changes"));
    } else if (wasModified) {
        setStatus({});
    }
}

void SettingsDialog::setStatus(const QString &text, bool error)
{
    m_statusLabel->setText(text);
    m_statusLabel->setToolTip(text);
    m_statusLabel->setProperty("tone", error ? QStringLiteral("error") : QString());
    m_statusLabel->style()->unpolish(m_statusLabel);
    m_statusLabel->style()->polish(m_statusLabel);
}

void SettingsDialog::revertChanges()
{
    applyConfigToPages(m_config);
    applyTheme(m_config.general.uiThemeMode);
    setStatus(MS_TR("Changes reverted"));
}

namespace {

constexpr bool FEATURE_SETTINGS_WINDOW_BORDER_ENABLED = true;

}  // namespace

void SettingsDialog::applySettingsWindowBorderMarginIfEnabled(QBoxLayout *layout)
{
    if (!FEATURE_SETTINGS_WINDOW_BORDER_ENABLED || !layout) {
        return;
    }
    // 1. 【设置】【窗口边框】内容内缩 1px，四边留给描边
    layout->setContentsMargins(1, 1, 1, 1);
}

void SettingsDialog::rememberSettingsWindowBorderColorIfEnabled(markshot::ui::UiThemeMode mode)
{
    if (!FEATURE_SETTINGS_WINDOW_BORDER_ENABLED) {
        return;
    }
    const bool light = markshot::ui::effectiveUiThemeMode(mode) == markshot::ui::UiThemeMode::Light;
    m_windowBorderColor = light ? QColor(0xCB, 0xD5, 0xE1) : tokens::kCardBorder;
}

void SettingsDialog::paintEvent(QPaintEvent *event)
{
    QDialog::paintEvent(event);
    paintSettingsWindowBorderIfEnabled();
}

void SettingsDialog::paintSettingsWindowBorderIfEnabled()
{
    if (!FEATURE_SETTINGS_WINDOW_BORDER_ENABLED || width() < 2 || height() < 2) {
        return;
    }
    // 1. 【设置】【窗口边框】四边各画 1px，不依赖会被窗口裁切的样式表边框
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.setPen(Qt::NoPen);
    painter.setBrush(m_windowBorderColor);
    painter.fillRect(QRect(0, 0, width(), 1), m_windowBorderColor);
    painter.fillRect(QRect(0, height() - 1, width(), 1), m_windowBorderColor);
    painter.fillRect(QRect(0, 0, 1, height()), m_windowBorderColor);
    painter.fillRect(QRect(width() - 1, 0, 1, height()), m_windowBorderColor);
}

void showSettingsDialog(QWidget *parent)
{
    static QPointer<SettingsDialog> dialog;
    if (!dialog) {
        dialog = new SettingsDialog(nullptr);
        dialog->setAttribute(Qt::WA_DeleteOnClose, true);
    }

    if (parent && parent->screen()) {
        const QRect available = parent->screen()->availableGeometry();
        dialog->move(available.center() - dialog->rect().center());
    } else if (QScreen *screen = QGuiApplication::primaryScreen()) {
        const QRect available = screen->availableGeometry();
        dialog->move(available.center() - dialog->rect().center());
    }

    dialog->show();
    dialog->raise();
    dialog->activateWindow();
}

}  // namespace markshot::settings
