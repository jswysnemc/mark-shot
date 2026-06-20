#include "settings/settings_page_general.h"

#include "autostart/autostart_manager.h"
#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QCheckBox>
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
                                             MS_TR("Configure tray startup and global shortcuts."),
                                             this);
    QFormLayout *startupForm = settingsCardForm(startupCard);
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
    config->general.launchOnStartup = m_launchOnStartup->isEnabled() && m_launchOnStartup->isChecked();
    config->general.hotkeysEnabled = m_hotkeysEnabled->isChecked();
    config->general.captureHotkey = m_captureHotkey->keySequence();
    config->general.fullscreenHotkey = m_fullscreenHotkey->keySequence();
}

}  // namespace markshot::settings
