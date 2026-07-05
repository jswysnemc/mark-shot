#pragma once

#include "settings/settings_config.h"

#include <QWidget>

class QCheckBox;
class QDoubleSpinBox;
class QLabel;
class QLineEdit;
class QPlainTextEdit;
class QSpinBox;

namespace markshot::settings {

class SettingsPageIntegrations final : public QWidget {
public:
    /// @brief 创建外部集成设置页。
    /// @param parent 父控件。
    explicit SettingsPageIntegrations(QWidget *parent = nullptr);

    /// @brief 将配置加载到页面控件。
    /// @param config 设置配置。
    void setConfig(const SettingsConfig &config);

    /// @brief 将页面控件值写回配置。
    /// @param config 需要更新的设置配置。
    void updateConfig(SettingsConfig *config) const;

private:
    /// @brief 刷新各能力实际生效的 provider 状态展示。
    /// @param config 设置配置。
    void refreshProviderStatus(const SettingsConfig &config);

    QLabel *m_ocrProviderStatus = nullptr;
    QLabel *m_translationProviderStatus = nullptr;
    QLabel *m_codeScanProviderStatus = nullptr;
    QLineEdit *m_codeScanCommand = nullptr;
    QSpinBox *m_codeScanTimeoutMs = nullptr;
    QLineEdit *m_uploadCommand = nullptr;
    QSpinBox *m_uploadTimeoutMs = nullptr;
    QPlainTextEdit *m_uploadEnv = nullptr;
    QCheckBox *m_ocrResultPanel = nullptr;
    QLineEdit *m_translationApiBase = nullptr;
    QLineEdit *m_translationApiKeyEnv = nullptr;
    QLineEdit *m_translationApiKey = nullptr;
    QLineEdit *m_translationModel = nullptr;
    QDoubleSpinBox *m_translationTemperature = nullptr;
    QPlainTextEdit *m_translationSystemPrompt = nullptr;
};

}  // namespace markshot::settings
