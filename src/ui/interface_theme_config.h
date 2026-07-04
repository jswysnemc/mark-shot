#pragma once

#include <QJsonObject>
#include <QPalette>
#include <QString>

namespace markshot::ui {

enum class UiThemeMode {
    System,
    Dark,
    Light,
};

/**
 * 从字符串解析界面主题模式。
 * @param raw 配置中的主题模式文本。
 * @return 解析后的界面主题模式。
 */
UiThemeMode uiThemeModeFromString(const QString &raw);

/**
 * 返回界面主题模式的配置名称。
 * @param mode 界面主题模式。
 * @return 可写入配置文件的名称。
 */
QString uiThemeModeName(UiThemeMode mode);

/**
 * 从应用配置根对象读取界面主题模式。
 * @param root 应用配置根对象。
 * @return 配置的主题模式，缺失或无效时返回 System。
 */
UiThemeMode uiThemeModeFromConfigRoot(const QJsonObject &root);

/**
 * 根据系统调色板解析实际界面主题。
 * @param mode 配置中的界面主题模式。
 * @param systemPalette 系统调色板。
 * @return 实际应用的明暗主题。
 */
UiThemeMode effectiveUiThemeMode(UiThemeMode mode, const QPalette &systemPalette);

/**
 * 根据当前 Qt 系统色彩方案解析实际界面主题。
 * @param mode 配置中的界面主题模式。
 * @return 实际应用的明暗主题。
 */
UiThemeMode effectiveUiThemeMode(UiThemeMode mode);

}  // namespace markshot::ui
