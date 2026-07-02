#pragma once

#include "shot_window.h"

#include <QKeySequence>

#include <array>
#include <optional>

namespace markshot::shortcut {

using ActionShortcuts = std::array<QKeySequence, static_cast<int>(ShotWindow::Action::Cancel) + 1>;
using ToolShortcuts = std::array<QKeySequence, static_cast<int>(ShotWindow::Tool::Laser) + 1>;

struct ShortcutConfig {
    ActionShortcuts actions;
    ToolShortcuts tools;
    QKeySequence startupColorPicker;
    QKeySequence startupRuler;
    QKeySequence startupCodeScanner;
    QKeySequence startupDisplayCapture;
    QKeySequence startupGifRecorder;
    QKeySequence startupVideoRecorder;
};

int actionIndex(ShotWindow::Action action);
int toolIndex(ShotWindow::Tool tool);
std::optional<ShotWindow::Action> actionFromConfigName(QString name);
std::optional<ShotWindow::Tool> toolFromConfigName(QString name);
ShortcutConfig configuredShortcuts(const QString &configPath);

}  // namespace markshot::shortcut
