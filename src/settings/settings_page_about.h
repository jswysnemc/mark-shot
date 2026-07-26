#pragma once

#include <QWidget>

namespace markshot::settings {

/// @brief 设置「关于」页，展示版本与运行环境信息。
class SettingsPageAbout final : public QWidget {
public:
    /// @brief 创建关于页。
    /// @param parent 父控件。
    explicit SettingsPageAbout(QWidget *parent = nullptr);
};

}  // namespace markshot::settings
