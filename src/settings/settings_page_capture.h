#pragma once

#include "settings/settings_config.h"

#include <QWidget>

class QCheckBox;
class QComboBox;

namespace markshot::settings {

class SettingsPageCapture final : public QWidget {
public:
    /// @brief 创建截图设置页。
    /// @param parent 父控件。
    explicit SettingsPageCapture(QWidget *parent = nullptr);

    /// @brief 将配置加载到页面控件。
    /// @param config 设置配置。
    void setConfig(const SettingsConfig &config);

    /// @brief 将页面控件值写回配置。
    /// @param config 需要更新的设置配置。
    void updateConfig(SettingsConfig *config) const;

private:
    QCheckBox *m_includeCursor = nullptr;
    QCheckBox *m_kdeKwinScreenshot = nullptr;
    QCheckBox *m_hideOwnWindows = nullptr;
    QComboBox *m_freezeScope = nullptr;
};

}  // namespace markshot::settings
