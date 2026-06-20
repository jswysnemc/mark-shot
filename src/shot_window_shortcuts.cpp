#include "shot_window_module.h"

namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

/**
 * 获取指定动作的快捷键。
 * @param action 动作枚举。
 * @return 配置后的动作快捷键。
 */
QKeySequence ShotWindow::shortcutForAction(Action action) const
{
    return m_actionShortcuts.at(shortcuts::actionIndex(action));
}

/**
 * 获取指定工具的快捷键。
 * @param tool 工具枚举。
 * @return 配置后的工具快捷键。
 */
QKeySequence ShotWindow::shortcutForTool(Tool tool) const
{
    return m_toolShortcuts.at(shortcuts::toolIndex(tool));
}

/**
 * 获取动作快捷键的显示文本。
 * @param action 动作枚举。
 * @param fallback 未配置快捷键时的备用文本。
 * @return 用于界面显示的快捷键文本。
 */
QString ShotWindow::shortcutText(Action action, const QString &fallback) const
{
    const QKeySequence sequence = shortcutForAction(action);
    if (sequence.isEmpty()) {
        return fallback;
    }
    return sequence.toString(QKeySequence::NativeText);
}

/**
 * 获取工具快捷键的显示文本。
 * @param tool 工具枚举。
 * @return 用于界面显示的快捷键文本。
 */
QString ShotWindow::shortcutText(Tool tool) const
{
    const QKeySequence sequence = shortcutForTool(tool);
    return sequence.isEmpty() ? QString() : sequence.toString(QKeySequence::NativeText);
}

namespace {

/**
 * 判断按键事件是否匹配指定快捷键。
 * @param sequence 需要匹配的快捷键。
 * @param event 当前按键事件。
 * @return 匹配时返回 true，否则返回 false。
 */
bool shortcutMatchesEvent(const QKeySequence &sequence, const QKeyEvent *event)
{
    if (!event || sequence.isEmpty() || event->key() == Qt::Key_unknown) {
        return false;
    }
    return QKeySequence(event->keyCombination()).matches(sequence) == QKeySequence::ExactMatch;
}

}

/**
 * 判断按键事件是否匹配指定动作。
 * @param event 当前按键事件。
 * @param action 动作枚举。
 * @return 匹配时返回 true，否则返回 false。
 */
bool ShotWindow::eventMatchesShortcut(const QKeyEvent *event, Action action) const
{
    return shortcutMatchesEvent(shortcutForAction(action), event);
}

/**
 * 判断按键事件是否匹配指定工具。
 * @param event 当前按键事件。
 * @param tool 工具枚举。
 * @return 匹配时返回 true，否则返回 false。
 */
bool ShotWindow::eventMatchesShortcut(const QKeyEvent *event, Tool tool) const
{
    return shortcutMatchesEvent(shortcutForTool(tool), event);
}

/**
 * 判断按键事件是否匹配启动阶段工具。
 * @param event 当前按键事件。
 * @param tool 启动阶段工具枚举。
 * @return 匹配时返回 true，否则返回 false。
 */
bool ShotWindow::eventMatchesStartupShortcut(const QKeyEvent *event, StartupTool tool) const
{
    if (tool == StartupTool::ColorPicker) {
        return shortcutMatchesEvent(m_startupColorPickerShortcut, event);
    }
    if (tool == StartupTool::Ruler) {
        return shortcutMatchesEvent(m_startupRulerShortcut, event);
    }
    if (tool == StartupTool::CodeScanner) {
        return shortcutMatchesEvent(m_startupCodeScannerShortcut, event);
    }
    return false;
}

/**
 * 判断按键事件是否匹配启动阶段显示器快速截取快捷键。
 * @param event 当前按键事件。
 * @return 匹配时返回 true，否则返回 false。
 */
bool ShotWindow::eventMatchesDisplayCaptureShortcut(const QKeyEvent *event) const
{
    return shortcutMatchesEvent(m_startupDisplayCaptureShortcut, event);
}

/**
 * 处理已配置的动作快捷键。
 * @param event 当前按键事件。
 * @return 已处理时返回 true，否则返回 false。
 */
bool ShotWindow::handleConfiguredActionShortcut(QKeyEvent *event)
{
    if (eventMatchesShortcut(event, Action::Cancel)) {
        emit sessionCancelRequested();
        close();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Copy)) {
        commitTextEditor();
        copySelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Save)) {
        commitTextEditor();
        saveSelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Pin)) {
        pinSelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Undo)) {
        undoAnnotationEdit();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Redo) || event->matches(QKeySequence::Redo)) {
        redoAnnotation();
        return true;
    }
    if (eventMatchesShortcut(event, Action::ToggleCaptureScope)) {
        toggleCaptureScope();
        return true;
    }
    if (eventMatchesShortcut(event, Action::ToggleToolbarLayout)) {
        toggleToolbarLayout();
        return true;
    }
    if (eventMatchesShortcut(event, Action::OpenWith)) {
        toggleOpenWithPanel();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Extensions)) {
        toggleExtensionPanel();
        return true;
    }
    if (eventMatchesShortcut(event, Action::ScrollCapture)) {
        startScrollCapture();
        return true;
    }
    if (eventMatchesShortcut(event, Action::OcrCopy)) {
        ocrCopySelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Upload)) {
        uploadSelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Settings)) {
        openSettingsAfterClosingCapture();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Clear)) {
        clearAnnotations();
        return true;
    }
    return false;
}

/**
 * 处理已配置的工具快捷键。
 * @param event 当前按键事件。
 * @return 已处理时返回 true，否则返回 false。
 */
bool ShotWindow::handleConfiguredToolShortcut(QKeyEvent *event)
{
    const std::array<Tool, static_cast<int>(Tool::Laser) + 1> tools = {
        Tool::Move,
        Tool::Select,
        Tool::Pen,
        Tool::Line,
        Tool::Highlighter,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Arrow,
        Tool::Text,
        Tool::Number,
        Tool::Mosaic,
        Tool::Magnifier,
        Tool::Laser,
    };
    for (Tool tool : tools) {
        if (eventMatchesShortcut(event, tool)) {
            setTool(tool);
            return true;
        }
    }
    return false;
}
