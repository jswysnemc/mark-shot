#include "settings/settings_page_general.h"

#include "autostart/autostart_manager.h"
#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"
#include "ui/interface_theme_config.h"

#include <QCheckBox>
#include <QComboBox>
#include <QFormLayout>
#include <QFrame>
#include <QKeySequenceEdit>
#include <QVBoxLayout>

namespace markshot::settings {

SettingsPageGeneral::SettingsPageGeneral(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = createSettingsPageLayout(this);

    QFrame *startupCard = createSettingsCard(MS_TR("General"),
                                             MS_TR("Configure interface language, theme, tray startup, and global shortcuts."),
                                             this);
    QFormLayout *startupForm = settingsCardForm(startupCard);
    m_uiLanguage = addComboRow(startupForm, MS_TR("Interface Language"));
    m_uiLanguage->addItem(MS_TR("Follow System"),
                          QVariant::fromValue(static_cast<int>(markshot::ui::UiLanguageMode::System)));
    m_uiLanguage->addItem(MS_TR("English"),
                          QVariant::fromValue(static_cast<int>(markshot::ui::UiLanguageMode::English)));
    m_uiLanguage->addItem(MS_TR("Simplified Chinese"),
                          QVariant::fromValue(static_cast<int>(markshot::ui::UiLanguageMode::Chinese)));
    m_uiTheme = addComboRow(startupForm, MS_TR("Interface Theme"));
    m_uiTheme->addItem(MS_TR("Follow System"),
                       QVariant::fromValue(static_cast<int>(markshot::ui::UiThemeMode::System)));
    m_uiTheme->addItem(MS_TR("Dark"),
                       QVariant::fromValue(static_cast<int>(markshot::ui::UiThemeMode::Dark)));
    m_uiTheme->addItem(MS_TR("Light"),
                       QVariant::fromValue(static_cast<int>(markshot::ui::UiThemeMode::Light)));
    m_trayEnabled = addSwitchRow(startupForm,
                                 MS_TR("Start in Tray"),
                                 MS_TR("Launch Mark Shot directly into the system tray."));
    m_launchOnStartup = addSwitchRow(startupForm,
                                     MS_TR("Launch on Startup"),
                                     MS_TR("Start Mark Shot automatically after signing in."));
    m_hotkeysEnabled = addSwitchRow(startupForm,
                                    MS_TR("Global Hotkeys"),
                                    MS_TR("Register global capture shortcuts when the tray starts."));
    layout->addWidget(startupCard);

    QFrame *hotkeyCard = createSettingsCard(MS_TR("Hotkeys"),
                                            MS_TR("Use the desktop portal on supported Linux desktops and RegisterHotKey on Windows."),
                                            this);
    QFormLayout *hotkeyForm = settingsCardForm(hotkeyCard);
    m_captureHotkey = addShortcutRow(hotkeyForm, MS_TR("Capture Hotkey"));
    m_fullscreenHotkey = addShortcutRow(hotkeyForm, MS_TR("Fullscreen Hotkey"));
    layout->addWidget(hotkeyCard);
    layout->addStretch();
}

void SettingsPageGeneral::setConfig(const SettingsConfig &config)
{
    const int languageIndex =
        m_uiLanguage->findData(QVariant::fromValue(static_cast<int>(config.general.uiLanguageMode)));
    m_uiLanguage->setCurrentIndex(languageIndex >= 0 ? languageIndex : 0);
    const int themeIndex =
        m_uiTheme->findData(QVariant::fromValue(static_cast<int>(config.general.uiThemeMode)));
    m_uiTheme->setCurrentIndex(themeIndex >= 0 ? themeIndex : 0);
    m_trayEnabled->setChecked(config.general.trayEnabled);
    m_launchOnStartup->setEnabled(autostart::isSupported());
    m_launchOnStartup->setChecked(m_launchOnStartup->isEnabled() && config.general.launchOnStartup);
    m_hotkeysEnabled->setChecked(config.general.hotkeysEnabled);
    m_captureHotkey->setKeySequence(config.general.captureHotkey);
    m_fullscreenHotkey->setKeySequence(config.general.fullscreenHotkey);
}

void SettingsPageGeneral::updateConfig(SettingsConfig *config) const
{
    if (!config) {
        return;
    }

    config->general.trayEnabled = m_trayEnabled->isChecked();
    config->general.uiLanguageMode =
        static_cast<markshot::ui::UiLanguageMode>(m_uiLanguage->currentData().toInt());
    config->general.uiThemeMode =
        static_cast<markshot::ui::UiThemeMode>(m_uiTheme->currentData().toInt());
    config->general.launchOnStartup = m_launchOnStartup->isEnabled() && m_launchOnStartup->isChecked();
    config->general.hotkeysEnabled = m_hotkeysEnabled->isChecked();
    config->general.captureHotkey = m_captureHotkey->keySequence();
    config->general.fullscreenHotkey = m_fullscreenHotkey->keySequence();
}

}  // namespace markshot::settings
