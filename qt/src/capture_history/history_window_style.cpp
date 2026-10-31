#include "capture_history/history_window_style.h"

#include "app_config_store.h"
#include "capture_history/history_window.h"
#include "settings/settings_design_tokens.h"
#include "ui/icons.h"

#include <QPainter>
#include <QPushButton>
#include <QStyleOption>

namespace markshot::history {
namespace {

/// @brief 按主题配色绘制应用已有的操作图标
/// @param action 工具图标类型
/// @param color 当前按钮文字颜色
/// @return 包含普通和禁用状态的图标
QIcon historyIcon(ShotWindow::Action action, QColor color)
{
    return markshot::ui::makeToolIcon(action, color);
}

}

QString historyWindowStyleSheet(markshot::ui::UiThemeMode mode)
{
    const QPalette colors = markshot::settings::tokens::settingsPalette(mode);
    const QColor border = mode == markshot::ui::UiThemeMode::Light
        ? QColor(203, 213, 225) : markshot::settings::tokens::kCardBorder;
    const QString selected = mode == markshot::ui::UiThemeMode::Light
        ? QStringLiteral("#E0F2EF") : QStringLiteral("#173936");
    return markshot::settings::tokens::settingsStyleSheet(mode) + QStringLiteral(
        "QWidget#historyWindow, QDialog#historyConfirmDialog { background: %1; color: %2; }"
        "QListWidget#historyList { background: transparent; border: 0; outline: 0; }"
        "QListWidget#historyList::item { color: %2; padding: 4px; border-radius: 6px; }"
        "QListWidget#historyList::item:hover { background: %3; }"
        "QListWidget#historyList::item:selected { background: %4; color: %2; }"
        "QLabel#historyPreview { background: %5; border: 1px solid %6; border-radius: 6px; padding: 6px; }"
        "QWidget#historyWindow QPushButton { min-height: 28px; padding: 0 8px; border-radius: 6px; }"
        "QPushButton#historyCloseButton { min-height: 0; padding: 0; }"
        "QSplitter::handle { background: transparent; width: 6px; }"
        "QSplitter::handle:hover { background: %6; border-radius: 3px; }")
        .arg(colors.color(QPalette::Window).name(), colors.color(QPalette::Text).name(),
             colors.color(QPalette::AlternateBase).name(), selected,
             colors.color(QPalette::Base).name(), border.name());
}

void HistoryWindow::applyTheme()
{
    // 1. 【截图历史】【主题同步】与设置页使用同一配置、调色板和控件样式
    const auto mode = markshot::ui::effectiveUiThemeMode(
        markshot::ui::uiThemeModeFromConfigRoot(markshot::readAppConfigRoot()));
    const QPalette colors = markshot::settings::tokens::settingsPalette(mode);
    setPalette(colors);
    setStyleSheet(historyWindowStyleSheet(mode));
    // 2. 【截图历史】【图标配色】浅色与深色主题分别使用当前文字色
    for (QPushButton *button : findChildren<QPushButton *>()) {
        const QVariant action = button->property("historyAction");
        if (action.isValid()) {
            button->setIcon(historyIcon(static_cast<ShotWindow::Action>(action.toInt()),
                colors.color(button->property("role").toString() == QStringLiteral("primary")
                    ? QPalette::HighlightedText : QPalette::ButtonText)));
        }
    }
    updateActionLayout();
}

void HistoryWindow::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event);
    QStyleOption option;
    option.initFrom(this);
    QPainter painter(this);
    style()->drawPrimitive(QStyle::PE_Widget, &option, &painter, this);
}

}
