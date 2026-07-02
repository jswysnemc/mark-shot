#pragma once

#include "settings/settings_config.h"

#include <QWidget>

#include <array>

class QFormLayout;
class QKeySequenceEdit;

namespace markshot::settings {

class SettingsPageShortcuts final : public QWidget {
public:
    /// @brief 创建快捷键设置页。
    /// @param parent 父控件。
    explicit SettingsPageShortcuts(QWidget *parent = nullptr);

    /// @brief 将配置加载到页面控件。
    /// @param config 设置配置。
    void setConfig(const SettingsConfig &config);

    /// @brief 将页面控件值写回配置。
    /// @param config 需要更新的设置配置。
    void updateConfig(SettingsConfig *config) const;

private:
    /// @brief 初始化工具快捷键输入项。
    /// @param form 目标表单布局。
    void addToolShortcutRows(QFormLayout *form);

    /// @brief 初始化动作快捷键输入项。
    /// @param form 目标表单布局。
    void addActionShortcutRows(QFormLayout *form);

    std::array<QKeySequenceEdit *, static_cast<int>(ShotWindow::Tool::Laser) + 1> m_toolEdits = {};
    std::array<QKeySequenceEdit *, static_cast<int>(ShotWindow::Action::Cancel) + 1> m_actionEdits = {};
    QKeySequenceEdit *m_startupColorPicker = nullptr;
    QKeySequenceEdit *m_startupRuler = nullptr;
    QKeySequenceEdit *m_startupCodeScanner = nullptr;
    QKeySequenceEdit *m_startupDisplayCapture = nullptr;
    QKeySequenceEdit *m_startupGifRecorder = nullptr;
    QKeySequenceEdit *m_startupVideoRecorder = nullptr;
};

}  // namespace markshot::settings
