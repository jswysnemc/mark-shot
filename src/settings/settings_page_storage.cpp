#include "settings/settings_page_storage.h"

#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QCheckBox>
#include <QComboBox>
#include <QDoubleSpinBox>
#include <QFormLayout>
#include <QFrame>
#include <QLineEdit>
#include <QSpinBox>
#include <QVBoxLayout>

namespace markshot::settings {

SettingsPageStorage::SettingsPageStorage(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = createSettingsPageLayout(this);

    QFrame *saveCard = createSettingsCard(MS_TR("Saving"),
                                          MS_TR("Configure the default file name template for saved screenshots."),
                                          this);
    QFormLayout *saveForm = settingsCardForm(saveCard);
    m_savePathTemplate = addTextRow(saveForm,
                                    MS_TR("Path Template"),
                                    QStringLiteral("{pictures}/mark-shot/mark-shot-{datetime}.png"));
    layout->addWidget(saveCard);

    QFrame *clipboardCard = createSettingsCard(MS_TR("Clipboard Image"),
                                               MS_TR("Choose how copied images are placed into the clipboard."),
                                               this);
    QFormLayout *clipboardForm = settingsCardForm(clipboardCard);
    m_clipboardMode = addComboRow(clipboardForm, MS_TR("Clipboard Mode"));
    m_clipboardMode->addItem(MS_TR("PNG Image"), static_cast<int>(ClipboardImageMode::ImagePng));
    m_clipboardMode->addItem(MS_TR("File URL"), static_cast<int>(ClipboardImageMode::Url));
    m_clipboardMode->addItem(MS_TR("Auto by Size"), static_cast<int>(ClipboardImageMode::Threshold));
    m_clipboardThresholdM = addSpinRow(clipboardForm, MS_TR("Threshold"), 1, 1024, QStringLiteral(" MiB"));
    layout->addWidget(clipboardCard);

    QFrame *exportCard = createSettingsCard(MS_TR("Screenshot Export Appearance"),
                                            MS_TR("Add a macOS-style transparent canvas and soft shadow to shared screenshots."),
                                            this);
    QFormLayout *exportForm = settingsCardForm(exportCard);
    m_exportImageFrameEnabled = addSwitchRow(exportForm,
                                             MS_TR("Mac-style Frame"),
                                             MS_TR("Apply only to saved, copied, uploaded, Open With, and extension-command images."));
    m_exportPadding = addSpinRow(exportForm, MS_TR("Transparent Padding"), 0, 256, QStringLiteral(" px"));
    m_exportCornerRadius = addSpinRow(exportForm, MS_TR("Corner Radius"), 0, 128, QStringLiteral(" px"));
    m_exportShadowRadius = addSpinRow(exportForm, MS_TR("Shadow Blur"), 0, 128, QStringLiteral(" px"));
    m_exportShadowOffsetY = addSpinRow(exportForm, MS_TR("Shadow Drop"), 0, 128, QStringLiteral(" px"));
    m_exportShadowOpacity = addDoubleRow(exportForm, MS_TR("Shadow Opacity"), 0.0, 1.0, 2);
    m_exportShadowOpacity->setSingleStep(0.05);
    layout->addWidget(exportCard);
    layout->addStretch();
}

void SettingsPageStorage::setConfig(const SettingsConfig &config)
{
    m_savePathTemplate->setText(config.storage.savePathTemplate);
    const int modeIndex = m_clipboardMode->findData(static_cast<int>(config.storage.clipboardImageMode));
    m_clipboardMode->setCurrentIndex(modeIndex >= 0 ? modeIndex : 0);
    m_clipboardThresholdM->setValue(config.storage.clipboardThresholdM);
    m_exportImageFrameEnabled->setChecked(config.storage.exportImageEffect.enabled);
    m_exportPadding->setValue(config.storage.exportImageEffect.padding);
    m_exportCornerRadius->setValue(qRound(config.storage.exportImageEffect.cornerRadius));
    m_exportShadowRadius->setValue(config.storage.exportImageEffect.shadowRadius);
    m_exportShadowOffsetY->setValue(config.storage.exportImageEffect.shadowOffsetY);
    m_exportShadowOpacity->setValue(config.storage.exportImageEffect.shadowOpacity);
}

void SettingsPageStorage::updateConfig(SettingsConfig *config) const
{
    if (!config) {
        return;
    }

    config->storage.savePathTemplate = m_savePathTemplate->text().trimmed();
    config->storage.clipboardImageMode =
        static_cast<ClipboardImageMode>(m_clipboardMode->currentData().toInt());
    config->storage.clipboardThresholdM = m_clipboardThresholdM->value();
    config->storage.exportImageEffect.enabled = m_exportImageFrameEnabled->isChecked();
    config->storage.exportImageEffect.padding = m_exportPadding->value();
    config->storage.exportImageEffect.cornerRadius = m_exportCornerRadius->value();
    config->storage.exportImageEffect.shadowRadius = m_exportShadowRadius->value();
    config->storage.exportImageEffect.shadowOffsetY = m_exportShadowOffsetY->value();
    config->storage.exportImageEffect.shadowOpacity = m_exportShadowOpacity->value();
}

}  // namespace markshot::settings
