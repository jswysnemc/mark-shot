#pragma once

#include "settings/settings_config.h"

#include <QColor>
#include <QWidget>

class QCheckBox;
class QDoubleSpinBox;
class QLineEdit;
class QPushButton;
class QSpinBox;

namespace markshot::settings {

class SettingsPagePinned final : public QWidget {
public:
    /// @brief 创建置顶图片设置页。
    /// @param parent 父控件。
    explicit SettingsPagePinned(QWidget *parent = nullptr);

    /// @brief 将配置加载到页面控件。
    /// @param config 设置配置。
    void setConfig(const SettingsConfig &config);

    /// @brief 将页面控件值写回配置。
    /// @param config 需要更新的设置配置。
    void updateConfig(SettingsConfig *config) const;

private:
    /// @brief 刷新置顶图片边框颜色按钮。
    void updateBorderColorButton();

    QCheckBox *m_alwaysOnTop = nullptr;
    QCheckBox *m_textSelectionCopy = nullptr;
    QCheckBox *m_borderEnabled = nullptr;
    QCheckBox *m_shadowEnabled = nullptr;
    QPushButton *m_borderColor = nullptr;
    QDoubleSpinBox *m_borderWidth = nullptr;
    QDoubleSpinBox *m_inactiveBorderOpacity = nullptr;
    QCheckBox *m_ocrEnabled = nullptr;
    QCheckBox *m_autoOcr = nullptr;
    QLineEdit *m_ocrBackend = nullptr;
    QLineEdit *m_ocrCommand = nullptr;
    QSpinBox *m_ocrTimeoutMs = nullptr;
    QCheckBox *m_autoTranslate = nullptr;
    QLineEdit *m_targetLanguage = nullptr;
    QLineEdit *m_translationCommand = nullptr;
    QSpinBox *m_translationTimeoutMs = nullptr;
    QColor m_borderColorValue;
};

}  // namespace markshot::settings
