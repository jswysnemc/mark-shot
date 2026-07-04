#pragma once

#include "settings/settings_config.h"

#include <QWidget>

class QCheckBox;
class QComboBox;
class QKeySequenceEdit;

namespace markshot::settings {

class SettingsPageGeneral final : public QWidget {
public:
    /// @brief 创建通用设置页。
    /// @param parent 父控件。
    explicit SettingsPageGeneral(QWidget *parent = nullptr);

    /// @brief 将配置加载到页面控件。
    /// @param config 设置配置。
    void setConfig(const SettingsConfig &config);

    /// @brief 将页面控件值写回配置。
    /// @param config 需要更新的设置配置。
    void updateConfig(SettingsConfig *config) const;

private:
    QComboBox *m_uiLanguage = nullptr;
    QComboBox *m_uiTheme = nullptr;
    QCheckBox *m_trayEnabled = nullptr;
    QCheckBox *m_launchOnStartup = nullptr;
    QCheckBox *m_hotkeysEnabled = nullptr;
    QKeySequenceEdit *m_captureHotkey = nullptr;
    QKeySequenceEdit *m_fullscreenHotkey = nullptr;
};

}  // namespace markshot::settings
