#include "settings/settings_page_advanced.h"

#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QCheckBox>
#include <QFormLayout>
#include <QFrame>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QSpinBox>
#include <QVBoxLayout>

namespace markshot::settings {

SettingsPageAdvanced::SettingsPageAdvanced(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = createSettingsPageLayout(this);

    // 1. 调试日志
    QFrame *debugCard = createSettingsCard(MS_TR("Debug"),
                                           MS_TR("Configure diagnostic logging for troubleshooting."),
                                           this);
    QFormLayout *debugForm = settingsCardForm(debugCard);
    m_debugEnabled = addSwitchRow(debugForm, MS_TR("Debug Logging"), MS_TR("Enable debug log output."));
    m_debugLogPath = addTextRow(debugForm, MS_TR("Debug Log Path"), QStringLiteral("~/mark-shot-debug.log"));
    layout->addWidget(debugCard);

    // 2. 窗口检测
    QFrame *windowCard = createSettingsCard(MS_TR("Window Detection"),
                                            MS_TR("Configure the external helper used to detect windows under the selection."),
                                            this);
    QFormLayout *windowForm = settingsCardForm(windowCard);
    m_windowDetectionEnabled = addSwitchRow(windowForm,
                                            MS_TR("Window Detection Enabled"),
                                            MS_TR("Run the configured helper before region selection."));
    m_windowDetectionCommand = addTextRow(windowForm,
                                          MS_TR("Window Detection Command"),
                                          QStringLiteral("mark-shot-window-detection-niri"));
    m_windowDetectionWorkingDirectory = addTextRow(windowForm, MS_TR("Working Directory"), QStringLiteral("~/"));
    m_windowDetectionTimeoutMs = addSpinRow(windowForm, MS_TR("Window Detection Timeout"), 100, 30000, QStringLiteral(" ms"));
    m_windowDetectionEnv = addPlainTextRow(windowForm,
                                           MS_TR("Window Detection Environment"),
                                           QStringLiteral("KEY=value"));
    layout->addWidget(windowCard);

    // 3. 应用环境变量
    QFrame *envCard = createSettingsCard(MS_TR("Application Environment"),
                                         MS_TR("Environment variables applied when Mark Shot starts."),
                                         this);
    QFormLayout *envForm = settingsCardForm(envCard);
    m_appEnv = addPlainTextRow(envForm, MS_TR("Application Environment"), QStringLiteral("KEY=value"));
    layout->addWidget(envCard);
    layout->addStretch();
}

void SettingsPageAdvanced::setConfig(const SettingsConfig &config)
{
    m_debugEnabled->setChecked(config.advanced.debugEnabled);
    m_debugLogPath->setText(config.advanced.debugLogPath);
    m_windowDetectionEnabled->setChecked(config.advanced.windowDetectionEnabled);
    m_windowDetectionCommand->setText(config.advanced.windowDetectionCommand);
    m_windowDetectionWorkingDirectory->setText(config.advanced.windowDetectionWorkingDirectory);
    m_windowDetectionTimeoutMs->setValue(config.advanced.windowDetectionTimeoutMs);
    m_windowDetectionEnv->setPlainText(envMapToText(config.advanced.windowDetectionEnv));
    m_appEnv->setPlainText(envMapToText(config.advanced.appEnv));
}

void SettingsPageAdvanced::updateConfig(SettingsConfig *config) const
{
    if (!config) {
        return;
    }

    config->advanced.debugEnabled = m_debugEnabled->isChecked();
    config->advanced.debugLogPath = m_debugLogPath->text().trimmed();
    config->advanced.windowDetectionEnabled = m_windowDetectionEnabled->isChecked();
    config->advanced.windowDetectionCommand = m_windowDetectionCommand->text().trimmed();
    config->advanced.windowDetectionWorkingDirectory = m_windowDetectionWorkingDirectory->text().trimmed();
    config->advanced.windowDetectionTimeoutMs = m_windowDetectionTimeoutMs->value();
    config->advanced.windowDetectionEnv = envMapFromText(m_windowDetectionEnv->toPlainText());
    config->advanced.appEnv = envMapFromText(m_appEnv->toPlainText());
}

}  // namespace markshot::settings
