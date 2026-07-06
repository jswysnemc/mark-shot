#include "settings/settings_page_capture.h"

#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QCheckBox>
#include <QComboBox>
#include <QFormLayout>
#include <QFrame>
#include <QVBoxLayout>

namespace markshot::settings {

SettingsPageCapture::SettingsPageCapture(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = createSettingsPageLayout(this);
    QFrame *captureCard = createSettingsCard(MS_TR("Capture"),
                                             MS_TR("Adjust how the frozen screenshot is captured before annotation starts."),
                                             this);
    QFormLayout *form = settingsCardForm(captureCard);
    m_includeCursor = addSwitchRow(form,
                                   MS_TR("Include Cursor"),
                                   MS_TR("Capture the mouse cursor in the frozen image when supported."));
    m_freezeScope = addComboRow(form, MS_TR("Freeze Scope"));
    m_freezeScope->addItem(MS_TR("All Screens"), static_cast<int>(CaptureFreezeScope::AllScreens));
    m_freezeScope->addItem(MS_TR("Cursor Screen"), static_cast<int>(CaptureFreezeScope::CursorScreen));
    m_kdeKwinScreenshot = addSwitchRow(form,
                                       MS_TR("KDE KWin Screenshot"),
                                       MS_TR("Use KWin ScreenShot2 on KDE Wayland when available."));
    m_hideOwnWindows = addSwitchRow(form,
                                    MS_TR("Hide Mark Shot Windows While Capturing"),
                                    MS_TR("Hide own windows from screenshots. Turn off to include them."));
    layout->addWidget(captureCard);
    layout->addStretch();
}

void SettingsPageCapture::setConfig(const SettingsConfig &config)
{
    m_includeCursor->setChecked(config.capture.includeCursor);
    const int index = m_freezeScope->findData(static_cast<int>(config.capture.freezeScope));
    m_freezeScope->setCurrentIndex(index >= 0 ? index : 0);
    m_kdeKwinScreenshot->setChecked(config.capture.kdeKwinScreenshotEnabled);
    m_hideOwnWindows->setChecked(config.capture.hideOwnWindows);
}

void SettingsPageCapture::updateConfig(SettingsConfig *config) const
{
    if (!config) {
        return;
    }

    config->capture.includeCursor = m_includeCursor->isChecked();
    config->capture.freezeScope =
        static_cast<CaptureFreezeScope>(m_freezeScope->currentData().toInt());
    config->capture.kdeKwinScreenshotEnabled = m_kdeKwinScreenshot->isChecked();
    config->capture.hideOwnWindows = m_hideOwnWindows->isChecked();
}

}  // namespace markshot::settings
