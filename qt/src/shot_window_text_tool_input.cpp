#include "shot_window_module.h"

using namespace markshot::shot;

/**
 * 文字工具下按住鼠标时决定编辑已有文字、仅提交还是新建文字。
 * @param imagePoint 按下位置的图像坐标。
 * @param committedEditor 本次按下是否刚把编辑器中的文字提交掉。
 * @return 无返回值。
 */
void ShotWindow::handleTextToolPress(QPointF imagePoint, bool committedEditor)
{
    // 1. 编辑器仍然打开说明按在编辑器自身范围内，交给编辑器处理
    if (m_textEditor && m_textEditor->isVisible()) {
        return;
    }

    // 2. 按在已有文字上直接编辑该文字，不再叠一个新的输入框
    if (const std::optional<int> hitId = annotationAt(imagePoint)) {
        if (const Annotation *annotation = annotationById(*hitId); annotation && annotation->tool == Tool::Text) {
            setSelectedAnnotations({*hitId});
            beginEditingSelectedTextAnnotation();
            return;
        }
    }

    // 3. 这次点击只用于结束上一段输入，不紧接着打开新的空白输入框
    if (committedEditor) {
        updateCursor();
        return;
    }

    // 4. 选区外的文字不会进入成图，只在选区内开始新建
    if (!normalizedSelection().contains(imagePoint)) {
        return;
    }
    beginTextAnnotation(imagePoint);
}

/**
 * 判断文字工具在指定位置按下时是否会开始输入，供光标提示使用。
 * @param imagePoint 图像坐标。
 * @return 位于已有文字上或选区内时返回 true。
 */
bool ShotWindow::textToolAcceptsPoint(QPointF imagePoint) const
{
    if (const std::optional<int> hitId = annotationAt(imagePoint)) {
        const Annotation *annotation = annotationById(*hitId);
        if (annotation && annotation->tool == Tool::Text) {
            return true;
        }
    }
    return normalizedSelection().contains(imagePoint);
}

/**
 * 编辑文字时放行复制、保存、钉图等导出快捷键：先提交文字再执行动作。
 * 编辑器没有选中文字时 Ctrl+C 才复制截图，有选中文字时仍复制所选文字。
 * @param watched 事件来源。
 * @param event 当前事件。
 * @return 已处理时返回 true。
 */
bool ShotWindow::handleTextEditorActionShortcut(QObject *watched, QEvent *event)
{
    if (!m_textEditor || !m_textEditor->isVisible() || watched != m_textEditor || !event
        || (event->type() != QEvent::ShortcutOverride && event->type() != QEvent::KeyPress)) {
        return false;
    }

    // 1. 只接管带 Ctrl、Alt 或 Meta 的组合键，普通字符始终交给编辑器输入
    auto *keyEvent = static_cast<QKeyEvent *>(event);
    constexpr Qt::KeyboardModifiers kCommandModifiers = Qt::ControlModifier | Qt::AltModifier | Qt::MetaModifier;
    if (!(keyEvent->modifiers() & kCommandModifiers)) {
        return false;
    }
    std::optional<Action> matched;
    for (Action action : {Action::Copy, Action::Save, Action::Pin, Action::OcrCopy, Action::Translate, Action::Upload}) {
        if (eventMatchesShortcut(keyEvent, action)) {
            matched = action;
            break;
        }
    }
    if (!matched.has_value()
        || (*matched == Action::Copy && m_textEditor->textCursor().hasSelection())) {
        return false;
    }

    // 2. 先认领快捷键，避免编辑器把 Ctrl+C 当成复制空选区吞掉
    if (event->type() == QEvent::ShortcutOverride) {
        event->accept();
        return true;
    }
    if (keyEvent->isAutoRepeat()) {
        return true;
    }

    // 3. 各导出入口都会先提交文字，再按正常流程导出
    switch (*matched) {
    case Action::Copy:
        copySelection();
        break;
    case Action::Save:
        saveSelection();
        break;
    case Action::Pin:
        pinSelection();
        break;
    case Action::OcrCopy:
        ocrCopySelection();
        break;
    case Action::Translate:
        translateSelection();
        break;
    case Action::Upload:
        uploadSelection();
        break;
    default:
        break;
    }
    return true;
}
