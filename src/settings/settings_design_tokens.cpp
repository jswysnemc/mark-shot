#include "settings/settings_design_tokens.h"

#include <QStringLiteral>

namespace markshot::settings::tokens {
namespace {

/**
 * 生成浅色设置界面完整样式表。
 * @return Qt 样式表文本。
 */
QString lightSettingsStyleSheet()
{
    return QStringLiteral(
        // 窗口与分区背景
        "QDialog#settingsDialog { background: #F8FAFC; color: #0F172A; }"
        "QFrame#settingsSidebar { background: #E2E8F0; border: 0; }"
        "QFrame#settingsFooter { background: #E2E8F0; border-top: 1px solid #CBD5E1; }"

        // 侧栏标题区
        "QLabel#settingsHeroTitle { color: #0F172A; font-size: 20px; font-weight: 800; }"
        "QLabel#settingsHeroText { color: #64748B; font-size: 12px; }"

        // 状态标签
        "QLabel#settingsStatus { color: #64748B; }"

        // 侧栏导航列表
        "QListWidget#settingsNavigation {"
        " background: transparent;"
        " border: 0;"
        " padding: 6px;"
        " outline: 0;"
        "}"
        "QListWidget#settingsNavigation::item {"
        " color: #475569;"
        " border-radius: 10px;"
        " padding: 9px 12px;"
        " margin: 2px 0;"
        "}"
        "QListWidget#settingsNavigation::item:hover {"
        " background: #FFFFFF;"
        " color: #0F172A;"
        "}"
        "QListWidget#settingsNavigation::item:selected {"
        " background: rgba(13, 148, 136, 0.14);"
        " color: #0F766E;"
        "}"
        "QListWidget#settingsNavigation::item:separator {"
        " background: transparent;"
        " border: 0;"
        " min-height: 1px;"
        " max-height: 1px;"
        " margin: 6px 10px;"
        "}"

        // 卡片
        "QFrame#settingsCard {"
        " background: #FFFFFF;"
        " border: 1px solid #CBD5E1;"
        " border-radius: 14px;"
        "}"
        "QLabel#settingsCardTitle { color: #0F172A; font-size: 15px; font-weight: 800; }"
        "QLabel#settingsCardDescription { color: #64748B; font-size: 12px; }"
        "QScrollArea#pluginDiagnosticsArea { background: transparent; border: 0; }"
        "QWidget#pluginDiagnosticsViewport { background: transparent; }"
        "QFrame#pluginDiagnosticItem {"
        " background: #F8FAFC;"
        " border: 1px solid #E2E8F0;"
        " border-radius: 12px;"
        "}"
        "QLabel#pluginDiagnosticProvider { color: #0F172A; font-size: 13px; font-weight: 800; }"
        "QLabel#pluginDiagnosticMeta { color: #64748B; font-size: 12px; }"
        "QLabel#pluginDiagnosticFieldTitle { color: #64748B; font-size: 11px; font-weight: 800; }"
        "QLabel#pluginDiagnosticFieldValue { color: #334155; font-size: 12px; }"
        "QLabel#pluginDiagnosticEmpty { color: #64748B; font-size: 12px; }"
        "QLabel#pluginDiagnosticStatus {"
        " border: 1px solid #CBD5E1;"
        " border-radius: 8px;"
        " padding: 2px 8px;"
        " background: #E2E8F0;"
        " color: #334155;"
        " font-size: 11px;"
        " font-weight: 800;"
        "}"
        "QLabel#pluginDiagnosticStatus[tone=\"success\"] {"
        " background: #CCFBF1;"
        " border-color: #99F6E4;"
        " color: #0F766E;"
        "}"
        "QLabel#pluginDiagnosticStatus[tone=\"error\"] {"
        " background: #FEE2E2;"
        " border-color: #FECACA;"
        " color: #B91C1C;"
        "}"

        // 输入控件统一浅色底
        "QLineEdit, QSpinBox, QDoubleSpinBox, QComboBox, QKeySequenceEdit, QPlainTextEdit {"
        " min-height: 30px;"
        " border: 1px solid #CBD5E1;"
        " border-radius: 8px;"
        " padding: 2px 8px;"
        " background: #FFFFFF;"
        " color: #0F172A;"
        " selection-background-color: rgba(13, 148, 136, 0.22);"
        "}"
        "QLineEdit:focus, QSpinBox:focus, QDoubleSpinBox:focus, QComboBox:focus,"
        " QKeySequenceEdit:focus, QPlainTextEdit:focus {"
        " border-color: #0D9488;"
        "}"
        "QComboBox QAbstractItemView {"
        " background: #FFFFFF;"
        " border: 1px solid #CBD5E1;"
        " border-radius: 8px;"
        " selection-background-color: rgba(13, 148, 136, 0.14);"
        " selection-color: #0F766E;"
        " color: #0F172A;"
        " outline: 0;"
        "}"
        "QSpinBox, QDoubleSpinBox { max-height: 30px; }"
        "QComboBox::drop-down {"
        " subcontrol-origin: padding;"
        " subcontrol-position: center right;"
        " width: 22px;"
        " border: 0;"
        " background: transparent;"
        "}"
        "QComboBox::down-arrow {"
        " image: url(:/icons/chevron-down.svg);"
        " width: 12px;"
        " height: 12px;"
        "}"
        "QSpinBox::up-button, QDoubleSpinBox::up-button {"
        " subcontrol-origin: content;"
        " subcontrol-position: top right;"
        " width: 20px;"
        " border: 0;"
        " background: transparent;"
        "}"
        "QSpinBox::down-button, QDoubleSpinBox::down-button {"
        " subcontrol-origin: content;"
        " subcontrol-position: bottom right;"
        " width: 20px;"
        " border: 0;"
        " background: transparent;"
        "}"
        "QSpinBox::up-arrow, QDoubleSpinBox::up-arrow {"
        " image: url(:/icons/chevron-up.svg);"
        " width: 11px;"
        " height: 11px;"
        "}"
        "QSpinBox::down-arrow, QDoubleSpinBox::down-arrow {"
        " image: url(:/icons/chevron-down.svg);"
        " width: 11px;"
        " height: 11px;"
        "}"

        "QCheckBox { color: #334155; spacing: 8px; }"
        "QCheckBox::indicator { width: 18px; height: 18px; }"

        // 按钮
        "QPushButton {"
        " min-height: 32px;"
        " border-radius: 9px;"
        " border: 1px solid #CBD5E1;"
        " padding: 4px 16px;"
        " background: #FFFFFF;"
        " color: #0F172A;"
        " font-weight: 700;"
        "}"
        "QPushButton:hover { border-color: #0D9488; color: #0F766E; }"
        "QPushButton[role=\"primary\"] {"
        " background: #0D9488;"
        " border-color: #0D9488;"
        " color: #FFFFFF;"
        "}"
        "QPushButton[role=\"primary\"]:hover { background: #0F766E; border-color: #0F766E; color: #FFFFFF; }"

        // 滚动条：浅色窄轨
        "QScrollBar:vertical { background: transparent; width: 10px; margin: 2px; }"
        "QScrollBar::handle:vertical {"
        " background: #CBD5E1;"
        " border-radius: 4px;"
        " min-height: 28px;"
        "}"
        "QScrollBar::handle:vertical:hover { background: #94A3B8; }"
        "QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical { height: 0; }"
        "QScrollBar::add-page:vertical, QScrollBar::sub-page:vertical { background: transparent; }");
}

}  // namespace

QString settingsStyleSheet()
{
    // 颜色字面量与 tokens.h 中的 QColor 保持同值。
    return QStringLiteral(
        // 窗口与分区背景
        "QDialog#settingsDialog { background: #0F172A; color: #F1F5F9; }"
        "QFrame#settingsSidebar { background: #0B1220; border: 0; }"
        "QFrame#settingsFooter { background: #0B1220; border-top: 1px solid #1E293B; }"

        // 侧栏标题区
        "QLabel#settingsHeroTitle { color: #F1F5F9; font-size: 20px; font-weight: 800; }"
        "QLabel#settingsHeroText { color: #64748B; font-size: 12px; }"

        // 状态标签
        "QLabel#settingsStatus { color: #64748B; }"

        // 侧栏导航列表
        "QListWidget#settingsNavigation {"
        " background: transparent;"
        " border: 0;"
        " padding: 6px;"
        " outline: 0;"
        "}"
        "QListWidget#settingsNavigation::item {"
        " color: #94A3B8;"
        " border-radius: 10px;"
        " padding: 9px 12px;"
        " margin: 2px 0;"
        "}"
        "QListWidget#settingsNavigation::item:hover {"
        " background: #1E293B;"
        " color: #F1F5F9;"
        "}"
        "QListWidget#settingsNavigation::item:selected {"
        " background: rgba(94, 234, 212, 0.14);"
        " color: #5EEAD4;"
        "}"
        // 分组分隔条：不可选的 separator 项
        "QListWidget#settingsNavigation::item:separator {"
        " background: transparent;"
        " border: 0;"
        " min-height: 1px;"
        " max-height: 1px;"
        " margin: 6px 10px;"
        "}"

        // 卡片
        "QFrame#settingsCard {"
        " background: #1E293B;"
        " border: 1px solid #334155;"
        " border-radius: 14px;"
        "}"
        "QLabel#settingsCardTitle { color: #F1F5F9; font-size: 15px; font-weight: 800; }"
        "QLabel#settingsCardDescription { color: #64748B; font-size: 12px; }"
        "QScrollArea#pluginDiagnosticsArea { background: transparent; border: 0; }"
        "QWidget#pluginDiagnosticsViewport { background: transparent; }"
        "QFrame#pluginDiagnosticItem {"
        " background: #0F172A;"
        " border: 1px solid #334155;"
        " border-radius: 12px;"
        "}"
        "QLabel#pluginDiagnosticProvider { color: #F1F5F9; font-size: 13px; font-weight: 800; }"
        "QLabel#pluginDiagnosticMeta { color: #94A3B8; font-size: 12px; }"
        "QLabel#pluginDiagnosticFieldTitle { color: #64748B; font-size: 11px; font-weight: 800; }"
        "QLabel#pluginDiagnosticFieldValue { color: #CBD5E1; font-size: 12px; }"
        "QLabel#pluginDiagnosticEmpty { color: #94A3B8; font-size: 12px; }"
        "QLabel#pluginDiagnosticStatus {"
        " border: 1px solid #334155;"
        " border-radius: 8px;"
        " padding: 2px 8px;"
        " background: #1E293B;"
        " color: #CBD5E1;"
        " font-size: 11px;"
        " font-weight: 800;"
        "}"
        "QLabel#pluginDiagnosticStatus[tone=\"success\"] {"
        " background: rgba(94, 234, 212, 0.14);"
        " border-color: rgba(94, 234, 212, 0.32);"
        " color: #5EEAD4;"
        "}"
        "QLabel#pluginDiagnosticStatus[tone=\"error\"] {"
        " background: rgba(248, 113, 113, 0.14);"
        " border-color: rgba(248, 113, 113, 0.32);"
        " color: #FCA5A5;"
        "}"

        // 输入控件统一深色底
        "QLineEdit, QSpinBox, QDoubleSpinBox, QComboBox, QKeySequenceEdit, QPlainTextEdit {"
        " min-height: 30px;"
        " border: 1px solid #334155;"
        " border-radius: 8px;"
        " padding: 2px 8px;"
        " background: #0F172A;"
        " color: #F1F5F9;"
        " selection-background-color: rgba(94, 234, 212, 0.3);"
        "}"
        "QLineEdit:focus, QSpinBox:focus, QDoubleSpinBox:focus, QComboBox:focus,"
        " QKeySequenceEdit:focus, QPlainTextEdit:focus {"
        " border-color: #5EEAD4;"
        "}"
        // 下拉弹出视图沿用深色
        "QComboBox QAbstractItemView {"
        " background: #0F172A;"
        " border: 1px solid #334155;"
        " border-radius: 8px;"
        " selection-background-color: rgba(94, 234, 212, 0.18);"
        " selection-color: #5EEAD4;"
        " color: #F1F5F9;"
        " outline: 0;"
        "}"
        // 数字输入框限高，与下拉框/文本框保持一致高度（否则上下调节按钮会撑高）
        "QSpinBox, QDoubleSpinBox { max-height: 30px; }"
        // 下拉箭头区：扁平透明、置于 padding 内，避免方形按钮盖住外框圆角
        "QComboBox::drop-down {"
        " subcontrol-origin: padding;"
        " subcontrol-position: center right;"
        " width: 22px;"
        " border: 0;"
        " background: transparent;"
        "}"
        // 显式提供下拉箭头图标（否则某些 widget style 在子控件被 QSS 接管后不画箭头）
        "QComboBox::down-arrow {"
        " image: url(:/icons/chevron-down.svg);"
        " width: 12px;"
        " height: 12px;"
        "}"
        // 数字微调按钮：扁平透明，各占内容区上下半高、在中线相接（不再固定高度而贴边）
        "QSpinBox::up-button, QDoubleSpinBox::up-button {"
        " subcontrol-origin: content;"
        " subcontrol-position: top right;"
        " width: 20px;"
        " border: 0;"
        " background: transparent;"
        "}"
        "QSpinBox::down-button, QDoubleSpinBox::down-button {"
        " subcontrol-origin: content;"
        " subcontrol-position: bottom right;"
        " width: 20px;"
        " border: 0;"
        " background: transparent;"
        "}"
        // 显式提供上下调节箭头图标
        "QSpinBox::up-arrow, QDoubleSpinBox::up-arrow {"
        " image: url(:/icons/chevron-up.svg);"
        " width: 11px;"
        " height: 11px;"
        "}"
        "QSpinBox::down-arrow, QDoubleSpinBox::down-arrow {"
        " image: url(:/icons/chevron-down.svg);"
        " width: 11px;"
        " height: 11px;"
        "}"

        "QCheckBox { color: #CBD5E1; spacing: 8px; }"
        "QCheckBox::indicator { width: 18px; height: 18px; }"

        // 按钮
        "QPushButton {"
        " min-height: 32px;"
        " border-radius: 9px;"
        " border: 1px solid #334155;"
        " padding: 4px 16px;"
        " background: #1E293B;"
        " color: #F1F5F9;"
        " font-weight: 700;"
        "}"
        "QPushButton:hover { border-color: #5EEAD4; color: #5EEAD4; }"
        "QPushButton[role=\"primary\"] {"
        " background: #5EEAD4;"
        " border-color: #5EEAD4;"
        " color: #0F172A;"
        "}"
        "QPushButton[role=\"primary\"]:hover { background: #2DD4BF; border-color: #2DD4BF; color: #0F172A; }"

        // 滚动条：深色窄轨，配合可滚动设置页
        "QScrollBar:vertical { background: transparent; width: 10px; margin: 2px; }"
        "QScrollBar::handle:vertical {"
        " background: #334155;"
        " border-radius: 4px;"
        " min-height: 28px;"
        "}"
        "QScrollBar::handle:vertical:hover { background: #475569; }"
        "QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical { height: 0; }"
        "QScrollBar::add-page:vertical, QScrollBar::sub-page:vertical { background: transparent; }");
}

QString settingsStyleSheet(markshot::ui::UiThemeMode mode)
{
    if (mode == markshot::ui::UiThemeMode::Light) {
        return lightSettingsStyleSheet();
    }
    return settingsStyleSheet();
}

QPalette settingsPalette(markshot::ui::UiThemeMode mode)
{
    QPalette pal;
    if (mode == markshot::ui::UiThemeMode::Light) {
        pal.setColor(QPalette::Window, QColor(248, 250, 252));
        pal.setColor(QPalette::WindowText, QColor(15, 23, 42));
        pal.setColor(QPalette::Base, QColor(255, 255, 255));
        pal.setColor(QPalette::AlternateBase, QColor(226, 232, 240));
        pal.setColor(QPalette::Text, QColor(15, 23, 42));
        pal.setColor(QPalette::Button, QColor(255, 255, 255));
        pal.setColor(QPalette::ButtonText, QColor(15, 23, 42));
        pal.setColor(QPalette::ToolTipBase, QColor(255, 255, 255));
        pal.setColor(QPalette::ToolTipText, QColor(15, 23, 42));
        pal.setColor(QPalette::BrightText, QColor(13, 148, 136));
        pal.setColor(QPalette::Link, QColor(13, 148, 136));
        pal.setColor(QPalette::Highlight, QColor(13, 148, 136));
        pal.setColor(QPalette::HighlightedText, QColor(255, 255, 255));
        return pal;
    }

    pal.setColor(QPalette::Window, kWindowBackground);
    pal.setColor(QPalette::WindowText, kTextPrimary);
    pal.setColor(QPalette::Base, kInputBackground);
    pal.setColor(QPalette::AlternateBase, kCardSurface);
    pal.setColor(QPalette::Text, kTextPrimary);
    pal.setColor(QPalette::Button, kCardSurface);
    pal.setColor(QPalette::ButtonText, kTextPrimary);
    pal.setColor(QPalette::ToolTipBase, kCardSurface);
    pal.setColor(QPalette::ToolTipText, kTextPrimary);
    pal.setColor(QPalette::BrightText, kAccent);
    pal.setColor(QPalette::Link, kAccent);
    pal.setColor(QPalette::Highlight, kAccent);
    pal.setColor(QPalette::HighlightedText, kAccentInk);
    return pal;
}

}  // namespace markshot::settings::tokens
