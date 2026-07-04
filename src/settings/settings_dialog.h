#pragma once

#include "settings/settings_config.h"

#include <QDialog>

class QLabel;
class QStackedWidget;
class QWidget;

namespace markshot::settings {

class SettingsNavigation;
class SettingsPageAnnotation;
class SettingsPageAdvanced;
class SettingsPageCapture;
class SettingsPageGeneral;
class SettingsPageIntegrations;
class SettingsPagePinned;
class SettingsPageScroll;
class SettingsPageShortcuts;
class SettingsPageStorage;

class SettingsDialog final : public QDialog {
public:
    /// @brief 创建设置窗口。
    /// @param parent 父控件。
    explicit SettingsDialog(QWidget *parent = nullptr);

private:
    /// @brief 从配置文件加载设置并更新所有页面。
    void loadConfig();

    /// @brief 从所有页面收集控件值。
    /// @return 设置结构。
    SettingsConfig collectConfig() const;

    /// @brief 保存当前设置。
    /// @param closeAfterSave 保存成功后是否关闭窗口。
    void saveConfig(bool closeAfterSave);

    /// @brief 应用设置界面主题。
    /// @param mode 配置中的界面主题模式。
    void applyTheme(markshot::ui::UiThemeMode mode);

    SettingsNavigation *m_navigation = nullptr;
    QStackedWidget *m_stack = nullptr;
    QLabel *m_statusLabel = nullptr;
    SettingsConfig m_config;
    SettingsPageGeneral *m_generalPage = nullptr;
    SettingsPageCapture *m_capturePage = nullptr;
    SettingsPageShortcuts *m_shortcutsPage = nullptr;
    SettingsPageAnnotation *m_annotationPage = nullptr;
    SettingsPagePinned *m_pinnedPage = nullptr;
    SettingsPageIntegrations *m_integrationsPage = nullptr;
    SettingsPageScroll *m_scrollPage = nullptr;
    SettingsPageStorage *m_storagePage = nullptr;
    SettingsPageAdvanced *m_advancedPage = nullptr;
};

/// @brief 显示全局设置窗口，重复调用会复用现有窗口。
/// @param parent 用于定位窗口的父控件。
void showSettingsDialog(QWidget *parent = nullptr);

}  // namespace markshot::settings
