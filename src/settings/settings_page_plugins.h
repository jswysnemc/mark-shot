#pragma once

#include "settings/settings_config.h"

#include <QWidget>

class QComboBox;
class QPlainTextEdit;
class QPushButton;
class QVBoxLayout;
class QWidget;

namespace markshot::settings {

class SettingsPagePlugins final : public QWidget {
public:
    /// @brief 创建插件管理设置页。
    /// @param parent 父控件。
    explicit SettingsPagePlugins(QWidget *parent = nullptr);

    /// @brief 将配置加载到页面控件。
    /// @param config 设置配置。
    void setConfig(const SettingsConfig &config);

    /// @brief 将页面控件值写回配置。
    /// @param config 需要更新的设置配置。
    void updateConfig(SettingsConfig *config) const;

private:
    /// @brief 构建 provider 选择卡片。
    /// @param layout 页面根布局。
    void buildProviderCard(QVBoxLayout *layout);

    /// @brief 构建插件目录卡片。
    /// @param layout 页面根布局。
    void buildDirectoriesCard(QVBoxLayout *layout);

    /// @brief 构建插件诊断卡片。
    /// @param layout 页面根布局。
    void buildDiagnosticsCard(QVBoxLayout *layout);

    /// @brief 刷新诊断表格。
    void refreshDiagnostics();

    QComboBox *m_ocrProvider = nullptr;
    QComboBox *m_translationProvider = nullptr;
    QComboBox *m_codeScanProvider = nullptr;
    QPlainTextEdit *m_directories = nullptr;
    QPushButton *m_openUserDirectory = nullptr;
    QWidget *m_diagnosticsContainer = nullptr;
    QVBoxLayout *m_diagnosticsLayout = nullptr;
};

}  // namespace markshot::settings
