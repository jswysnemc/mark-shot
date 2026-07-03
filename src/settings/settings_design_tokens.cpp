#include "settings/settings_design_tokens.h"

#include <QStringLiteral>

namespace markshot::settings::tokens {

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

}  // namespace markshot::settings::tokens
