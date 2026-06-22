#include "settings/settings_dialog.h"

#include "settings/settings_design_tokens.h"
#include "settings/settings_navigation.h"
#include "settings/settings_page_advanced.h"
#include "settings/settings_page_annotation.h"
#include "settings/settings_page_capture.h"
#include "settings/settings_page_general.h"
#include "settings/settings_page_integrations.h"
#include "settings/settings_page_pinned.h"
#include "settings/settings_page_scroll.h"
#include "settings/settings_page_shortcuts.h"
#include "settings/settings_page_storage.h"
#include "ui/i18n.h"
#include "ui/icons.h"

#include <QDialogButtonBox>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QMessageBox>
#include <QPalette>
#include <QPointer>
#include <QPushButton>
#include <QScreen>
#include <QGuiApplication>
#include <QScrollArea>
#include <QStackedWidget>
#include <QVBoxLayout>

namespace markshot::settings {
namespace {

/// @brief 将设置页包装成可滚动页面。
/// @param stack 目标堆叠控件。
/// @param page 需要显示的设置页。
void addScrollablePage(QStackedWidget *stack, QWidget *page)
{
    auto *area = new QScrollArea(stack);
    area->setFrameShape(QFrame::NoFrame);
    area->setWidgetResizable(true);
    area->setWidget(page);
    stack->addWidget(area);
}

}  // namespace

SettingsDialog::SettingsDialog(QWidget *parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("settingsDialog"));
    setWindowTitle(MS_TR("Settings"));
    setWindowIcon(markshot::ui::applicationIcon());
    setMinimumSize(820, 600);
    resize(900, 640);

    QPalette pal;
    pal.setColor(QPalette::Window, tokens::kWindowBackground);
    pal.setColor(QPalette::WindowText, tokens::kTextPrimary);
    pal.setColor(QPalette::Base, tokens::kInputBackground);
    pal.setColor(QPalette::AlternateBase, tokens::kCardSurface);
    pal.setColor(QPalette::Text, tokens::kTextPrimary);
    pal.setColor(QPalette::Button, tokens::kCardSurface);
    pal.setColor(QPalette::ButtonText, tokens::kTextPrimary);
    pal.setColor(QPalette::ToolTipBase, tokens::kCardSurface);
    pal.setColor(QPalette::ToolTipText, tokens::kTextPrimary);
    pal.setColor(QPalette::BrightText, tokens::kAccent);
    pal.setColor(QPalette::Link, tokens::kAccent);
    pal.setColor(QPalette::Highlight, tokens::kAccent);
    pal.setColor(QPalette::HighlightedText, tokens::kAccentInk);
    qApp->setPalette(pal);

    setStyleSheet(tokens::settingsStyleSheet());

    auto *rootLayout = new QVBoxLayout(this);
    rootLayout->setContentsMargins(0, 0, 0, 0);
    rootLayout->setSpacing(0);

    auto *body = new QWidget(this);
    auto *bodyLayout = new QHBoxLayout(body);
    bodyLayout->setContentsMargins(0, 0, 0, 0);
    bodyLayout->setSpacing(0);

    // 侧栏导航：标题区 + 分组分类列表
    m_navigation = new SettingsNavigation(body);
    bodyLayout->addWidget(m_navigation);

    // 内容栈：9 个可滚动设置页
    m_stack = new QStackedWidget(body);
    m_generalPage = new SettingsPageGeneral(m_stack);
    m_capturePage = new SettingsPageCapture(m_stack);
    m_shortcutsPage = new SettingsPageShortcuts(m_stack);
    m_annotationPage = new SettingsPageAnnotation(m_stack);
    m_pinnedPage = new SettingsPagePinned(m_stack);
    m_integrationsPage = new SettingsPageIntegrations(m_stack);
    m_scrollPage = new SettingsPageScroll(m_stack);
    m_storagePage = new SettingsPageStorage(m_stack);
    m_advancedPage = new SettingsPageAdvanced(m_stack);
    addScrollablePage(m_stack, m_generalPage);
    addScrollablePage(m_stack, m_capturePage);
    addScrollablePage(m_stack, m_shortcutsPage);
    addScrollablePage(m_stack, m_annotationPage);
    addScrollablePage(m_stack, m_pinnedPage);
    addScrollablePage(m_stack, m_integrationsPage);
    addScrollablePage(m_stack, m_scrollPage);
    addScrollablePage(m_stack, m_storagePage);
    addScrollablePage(m_stack, m_advancedPage);
    bodyLayout->addWidget(m_stack, 1);
    rootLayout->addWidget(body, 1);

    auto *footer = new QFrame(this);
    footer->setObjectName(QStringLiteral("settingsFooter"));
    auto *footerLayout = new QHBoxLayout(footer);
    footerLayout->setContentsMargins(18, 10, 18, 10);
    m_statusLabel = new QLabel(MS_TR("Some changes take effect after restarting Mark Shot."), footer);
    m_statusLabel->setObjectName(QStringLiteral("settingsStatus"));
    footerLayout->addWidget(m_statusLabel, 1);
    auto *buttons = new QDialogButtonBox(footer);
    QPushButton *applyButton = buttons->addButton(MS_TR("Apply"), QDialogButtonBox::ApplyRole);
    QPushButton *saveButton = buttons->addButton(MS_TR("Save"), QDialogButtonBox::AcceptRole);
    QPushButton *cancelButton = buttons->addButton(MS_TR("Cancel"), QDialogButtonBox::RejectRole);
    saveButton->setProperty("role", QStringLiteral("primary"));
    footerLayout->addWidget(buttons);
    rootLayout->addWidget(footer);

    // 1. 导航切换驱动内容栈翻页
    connect(m_navigation, &SettingsNavigation::navigationChanged, m_stack, &QStackedWidget::setCurrentIndex);
    // 2. 底部按钮：应用 / 保存 / 取消
    connect(applyButton, &QPushButton::clicked, this, [this] { saveConfig(false); });
    connect(saveButton, &QPushButton::clicked, this, [this] { saveConfig(true); });
    connect(cancelButton, &QPushButton::clicked, this, &QDialog::close);

    m_navigation->setCurrentLogicalRow(0);
    loadConfig();
}

void SettingsDialog::loadConfig()
{
    QString error;
    m_config = readSettingsConfig(&error);
    m_generalPage->setConfig(m_config);
    m_capturePage->setConfig(m_config);
    m_shortcutsPage->setConfig(m_config);
    m_annotationPage->setConfig(m_config);
    m_pinnedPage->setConfig(m_config);
    m_integrationsPage->setConfig(m_config);
    m_scrollPage->setConfig(m_config);
    m_storagePage->setConfig(m_config);
    m_advancedPage->setConfig(m_config);
    if (!error.isEmpty()) {
        m_statusLabel->setText(error);
    }
}

SettingsConfig SettingsDialog::collectConfig() const
{
    SettingsConfig config = m_config;
    m_generalPage->updateConfig(&config);
    m_capturePage->updateConfig(&config);
    m_shortcutsPage->updateConfig(&config);
    m_annotationPage->updateConfig(&config);
    m_pinnedPage->updateConfig(&config);
    m_integrationsPage->updateConfig(&config);
    m_scrollPage->updateConfig(&config);
    m_storagePage->updateConfig(&config);
    m_advancedPage->updateConfig(&config);
    return config;
}

void SettingsDialog::saveConfig(bool closeAfterSave)
{
    SettingsConfig nextConfig = collectConfig();
    QString error;
    if (!writeSettingsConfig(nextConfig, &error)) {
        QMessageBox::critical(this, MS_TR("Settings"), MS_TR("Failed to save settings: %1").arg(error));
        return;
    }

    m_config = nextConfig;
    m_statusLabel->setText(MS_TR("Settings saved. Some changes take effect after restarting Mark Shot."));
    if (closeAfterSave) {
        close();
    }
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
