#pragma once

#include "ui/interface_theme_config.h"

#include <QColor>
#include <QPalette>
#include <QString>

namespace markshot::settings::tokens {

// 设置界面设计 token（深色玻璃 + teal 强调）。
// 与 overlay 主主题 theme::kAccent 保持一致，消除设置窗口与主应用视觉割裂。
// 颜色值同时供 QSS 字面量与 QPainter 代码引用，二者必须保持同值。

// 背景与表面层
inline const QColor kWindowBackground{15, 23, 42};    // #0F172A slate-900
inline const QColor kSidebarBackground{11, 18, 32};   // #0B1220 比内容区更深，形成层次
inline const QColor kContentBackground{15, 23, 42};   // #0F172A
inline const QColor kFooterBackground{11, 18, 32};    // #0B1220
inline const QColor kCardSurface{30, 41, 59};         // #1E293B slate-800
inline const QColor kCardBorder{51, 65, 85};          // #334155 slate-700

// 文本层级
inline const QColor kTextPrimary{241, 245, 249};      // #F1F5F9 slate-100
inline const QColor kTextSecondary{148, 163, 184};    // #94A3B8 slate-400
inline const QColor kTextMuted{100, 116, 139};        // #64748B slate-500

// teal 强调（与 theme::kAccent 一致）
inline const QColor kAccent{94, 234, 212};            // #5EEAD4
inline const QColor kAccentHover{45, 212, 191};       // #2DD4BF
inline const QColor kAccentInk{15, 23, 42};           // 主按钮 teal 底上的深色文字

// 输入控件
inline const QColor kInputBackground{15, 23, 42};     // #0F172A
inline const QColor kInputBorder{51, 65, 85};         // #334155

// 几何 token
inline constexpr int kSpacingPage = 20;       // 页面外边距
inline constexpr int kSpacingCardX = 18;      // 卡片水平内边距
inline constexpr int kSpacingCardY = 16;      // 卡片垂直内边距
inline constexpr int kRadiusCard = 14;        // 卡片圆角
inline constexpr int kRadiusControl = 8;      // 输入控件圆角
inline constexpr int kRadiusButton = 9;       // 按钮圆角
inline constexpr int kSidebarWidth = 220;     // 侧栏固定宽度
inline constexpr int kNavIconSize = 18;       // 导航图标边长

/// @brief 生成设置界面完整样式表（深色玻璃 + teal）。
/// @return Qt 样式表文本，按 objectName 作用域隔离。
QString settingsStyleSheet();

/**
 * 生成设置界面完整样式表。
 * @param mode 实际应用的明暗主题。
 * @return Qt 样式表文本，按 objectName 作用域隔离。
 */
QString settingsStyleSheet(markshot::ui::UiThemeMode mode);

/**
 * 生成设置界面使用的 Qt 调色板。
 * @param mode 实际应用的明暗主题。
 * @return 与设置样式表匹配的调色板。
 */
QPalette settingsPalette(markshot::ui::UiThemeMode mode);

}  // namespace markshot::settings::tokens
