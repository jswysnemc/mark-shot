#include "settings/settings_page_integrations.h"

#include "providers/code_scan/code_scan_provider_factory.h"
#include "providers/ocr/ocr_provider_factory.h"
#include "providers/translate/translate_provider_factory.h"
#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QCheckBox>
#include <QDoubleSpinBox>
#include <QFormLayout>
#include <QFrame>
#include <QLabel>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QSpinBox>
#include <QVBoxLayout>

namespace markshot::settings {

SettingsPageIntegrations::SettingsPageIntegrations(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = createSettingsPageLayout(this);

    // 1. Provider 状态卡片：展示各能力当前实际生效的执行方
    QFrame *providerCard = createSettingsCard(MS_TR("Provider Status"),
                                              MS_TR("Shows which provider each capability currently resolves to: "
                                                    "custom command, plugin, builtin, or the legacy helper."),
                                              this);
    QFormLayout *providerForm = settingsCardForm(providerCard);
    m_ocrProviderStatus = new QLabel(this);
    providerForm->addRow(MS_TR("OCR"), m_ocrProviderStatus);
    m_translationProviderStatus = new QLabel(this);
    providerForm->addRow(MS_TR("Translation"), m_translationProviderStatus);
    m_codeScanProviderStatus = new QLabel(this);
    providerForm->addRow(MS_TR("Code Scanner"), m_codeScanProviderStatus);
    layout->addWidget(providerCard);

    QFrame *codeCard = createSettingsCard(MS_TR("Code Scanner"),
                                          MS_TR("Configure the external helper used to recognize QR codes and barcodes."),
                                          this);
    QFormLayout *codeForm = settingsCardForm(codeCard);
    m_codeScanCommand = addTextRow(codeForm, MS_TR("Scan Command"), QStringLiteral("mark-shot-code-scan {image}"));
    m_codeScanTimeoutMs = addSpinRow(codeForm, MS_TR("Scan Timeout"), 1000, 300000, QStringLiteral(" ms"));
    layout->addWidget(codeCard);

    QFrame *uploadCard = createSettingsCard(MS_TR("Image Upload"),
                                            MS_TR("Configure the external helper used to upload screenshots."),
                                            this);
    QFormLayout *uploadForm = settingsCardForm(uploadCard);
    m_uploadCommand = addTextRow(uploadForm, MS_TR("Upload Command"), QStringLiteral("mark-shot-upload {image}"));
    m_uploadTimeoutMs = addSpinRow(uploadForm, MS_TR("Upload Timeout"), 1000, 300000, QStringLiteral(" ms"));
    m_uploadEnv = addPlainTextRow(uploadForm,
                                  MS_TR("Upload Environment"),
                                  QStringLiteral("TOKEN=example"));
    layout->addWidget(uploadCard);

    QFrame *translationCard = createSettingsCard(MS_TR("OCR and Translation Integration"),
                                                 MS_TR("Configure OCR result panels and API-based translation helpers."),
                                                 this);
    QFormLayout *translationForm = settingsCardForm(translationCard);
    m_ocrResultPanel = addSwitchRow(translationForm,
                                    MS_TR("OCR Result Panel"),
                                    MS_TR("Show an editable OCR result panel before copying text."));
    m_translationApiBase = addTextRow(translationForm,
                                      MS_TR("Translation API Base"),
                                      QStringLiteral("https://api.openai.com/v1"));
    m_translationApiKeyEnv = addTextRow(translationForm,
                                        MS_TR("API Key Environment"),
                                        QStringLiteral("OPENAI_API_KEY"));
    m_translationApiKey = addTextRow(translationForm, MS_TR("API Key"), QStringLiteral("sk-..."));
    m_translationApiKey->setEchoMode(QLineEdit::PasswordEchoOnEdit);
    m_translationModel = addTextRow(translationForm, MS_TR("Translation Model"), QStringLiteral("gpt-4o-mini"));
    m_translationTemperature = addDoubleRow(translationForm, MS_TR("Temperature"), 0.0, 2.0, 2);
    m_translationSystemPrompt = addPlainTextRow(translationForm,
                                                MS_TR("System Prompt"),
                                                MS_TR("Optional translation system prompt."));
    layout->addWidget(translationCard);
    layout->addStretch();
}

void SettingsPageIntegrations::setConfig(const SettingsConfig &config)
{
    m_codeScanCommand->setText(config.integrations.codeScanCommand);
    m_codeScanTimeoutMs->setValue(config.integrations.codeScanTimeoutMs);
    m_uploadCommand->setText(config.integrations.uploadCommand);
    m_uploadTimeoutMs->setValue(config.integrations.uploadTimeoutMs);
    m_uploadEnv->setPlainText(envMapToText(config.integrations.uploadEnv));
    m_ocrResultPanel->setChecked(config.integrations.ocrResultPanelEnabled);
    m_translationApiBase->setText(config.integrations.translationApiBase);
    m_translationApiKeyEnv->setText(config.integrations.translationApiKeyEnv);
    m_translationApiKey->setText(config.integrations.translationApiKey);
    m_translationModel->setText(config.integrations.translationModel);
    m_translationTemperature->setValue(config.integrations.translationTemperature);
    m_translationSystemPrompt->setPlainText(config.integrations.translationSystemPrompt);
    refreshProviderStatus(config);
}

void SettingsPageIntegrations::refreshProviderStatus(const SettingsConfig &config)
{
    // 1. 按 auto 链解析各能力实际生效的 provider 并展示
    markshot::providers::OcrTaskRequest ocrRequest;
    m_ocrProviderStatus->setText(markshot::providers::resolvedOcrProviderName(ocrRequest));

    markshot::providers::TranslateTaskRequest translateRequest;
    m_translationProviderStatus->setText(
        markshot::providers::resolvedTranslateProviderName(translateRequest));

    markshot::providers::CodeScanTaskRequest codeScanRequest;
    codeScanRequest.commandLine = config.integrations.codeScanCommand.trimmed();
    m_codeScanProviderStatus->setText(
        markshot::providers::resolvedCodeScanProviderName(codeScanRequest));
}

void SettingsPageIntegrations::updateConfig(SettingsConfig *config) const
{
    if (!config) {
        return;
    }

    config->integrations.codeScanCommand = m_codeScanCommand->text().trimmed();
    config->integrations.codeScanTimeoutMs = m_codeScanTimeoutMs->value();
    config->integrations.uploadCommand = m_uploadCommand->text().trimmed();
    config->integrations.uploadTimeoutMs = m_uploadTimeoutMs->value();
    config->integrations.uploadEnv = envMapFromText(m_uploadEnv->toPlainText());
    config->integrations.ocrResultPanelEnabled = m_ocrResultPanel->isChecked();
    config->integrations.translationApiBase = m_translationApiBase->text().trimmed();
    config->integrations.translationApiKeyEnv = m_translationApiKeyEnv->text().trimmed();
    config->integrations.translationApiKey = m_translationApiKey->text().trimmed();
    config->integrations.translationModel = m_translationModel->text().trimmed();
    config->integrations.translationTemperature = m_translationTemperature->value();
    config->integrations.translationSystemPrompt = m_translationSystemPrompt->toPlainText();
}

}  // namespace markshot::settings
