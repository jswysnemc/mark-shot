//! 【框选翻译】【输入隔离】原生列表按身份接收事件，浮条左右键职责独立于画布

use super::{feedback, panels};
use crate::{
    editor::EditorState,
    keys::{Chord, Key},
    types::{MouseButton, popup::PopupEvent},
    ui::selection_translation::{draw::contains, language_popup::Event, result_bar::Action},
};

/// 【框选翻译】【弹层动作】将 event 应用到 state，dirty 返回关闭范围，选中语言留待角色销毁后启动
fn chosen(state: &mut EditorState, event: Event, dirty: &mut u32) {
    match event {
        Event::None => {}
        Event::Close => panels::close(state, dirty),
        Event::Choose(target) => {
            panels::close(state, dirty);
            state.selection_translation.requested = Some(target);
        }
    }
}

/// 【框选翻译】【原生事件】按 id 将 event 交给 state 当前语言列表，dirty 返回回退绘制范围，返回是否属于翻译弹层
pub(in crate::app) fn popup(
    state: &mut EditorState,
    id: u64,
    event: PopupEvent,
    dirty: &mut u32,
) -> bool {
    if let PopupEvent::Key {
        chord: Some(chord), ..
    } = &event
        && crate::theme::is_toggle(*chord)
        && state
            .selection_translation
            .popup
            .as_ref()
            .is_some_and(|p| p.id == id)
    {
        super::super::theme_switch::toggle(state, dirty);
        return true;
    }
    let Some(popup) = state
        .selection_translation
        .popup
        .as_mut()
        .filter(|p| p.id == id)
    else {
        return false;
    };
    let event = match event {
        PopupEvent::Configured { position } => {
            popup.rect = tiny_skia::Rect::from_xywh(
                position.0 as f32,
                position.1 as f32,
                popup.rect.width(),
                popup.rect.height(),
            )
            .unwrap();
            Event::None
        }
        PopupEvent::Pointer { x, y } => {
            popup.pointer((x, y));
            Event::None
        }
        PopupEvent::Button {
            x,
            y,
            button,
            pressed,
        } => popup.button((x, y), button, pressed),
        PopupEvent::Key { chord, .. } => {
            chord.map_or(Event::None, |c| popup.key(c.key, c.mods.shift))
        }
        PopupEvent::Closed => Event::Close,
        _ => Event::None,
    };
    chosen(state, event, dirty);
    true
}

/// 【框选翻译】【指针接入】按 state 最新指针更新浮条或列表，dirty 返回悬停区域，返回是否阻止画布手势
pub(in crate::app) fn pointer(state: &mut EditorState, dirty: &mut u32) -> bool {
    if let Some(popup) = &mut state.selection_translation.popup {
        if !popup.native && popup.monitor == state.input.pointer.monitor_idx {
            let point = (
                state.input.pointer.local.0 - f64::from(popup.rect.left()),
                state.input.pointer.local.1 - f64::from(popup.rect.top()),
            );
            if popup.pointer(point) {
                let (monitor, rect) = (popup.monitor, popup.rect);
                state.damage_local(monitor, rect);
                crate::editor::dirty::mark_dirty(dirty, state.input.pointer.monitor_idx);
            }
        }
        return true;
    }
    if state.selection_translation.busy() {
        return true;
    }
    let Some(bar) = &mut state.selection_translation.bar else {
        return false;
    };
    let hovered = if bar.monitor == state.input.pointer.monitor_idx {
        bar.hit(state.input.pointer.local)
    } else {
        None
    };
    let inside = bar.monitor == state.input.pointer.monitor_idx
        && contains(bar.rect, state.input.pointer.local);
    if bar.hovered != hovered {
        bar.hovered = hovered;
        let (monitor, rect) = (bar.monitor, bar.rect);
        state.damage_local(monitor, rect);
        crate::editor::dirty::mark_dirty(dirty, monitor);
    }
    inside || state.selection_translation.bar_pressed
}

/// 【框选翻译】【浮条动作】按 action 修改 state 显示、复制全文或退出，dirty 返回界面变化，不复制可见省略文本
fn action(state: &mut EditorState, action: Action, dirty: &mut u32) {
    match action {
        Action::Language => panels::open(state, true, dirty),
        Action::Original | Action::Translated => {
            if let Some(session) = &mut state.selection_translation.session {
                session.showing_translation = action == Action::Translated;
                super::apply(state, dirty);
            }
        }
        Action::Copy => {
            if let Some(translation) = state
                .selection_translation
                .session
                .as_ref()
                .and_then(|s| s.current())
            {
                let copied = crate::utils::copy_to_clipboard(&translation.text).is_ok();
                feedback(
                    state,
                    if copied {
                        "Translation copied"
                    } else {
                        "Copy failed"
                    },
                    dirty,
                );
            }
        }
        Action::Exit => super::exit(state, dirty),
    }
}

/// 【框选翻译】【按钮接入】按 button 和 pressed 分派 state 弹层与浮条，dirty 返回变化，返回是否已消费
pub(in crate::app) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if !pressed && state.selection_translation.swallow_release {
        if matches!(button, MouseButton::Left) {
            state.selection_translation.swallow_release = false;
        }
        return true;
    }
    if let Some(popup) = &mut state.selection_translation.popup {
        let point = state.input.pointer.local;
        let inside =
            popup.monitor == state.input.pointer.monitor_idx && contains(popup.rect, point);
        let event = if !inside && pressed {
            state.selection_translation.swallow_release = matches!(button, MouseButton::Left);
            Event::Close
        } else {
            popup.button(
                (
                    point.0 - f64::from(popup.rect.left()),
                    point.1 - f64::from(popup.rect.top()),
                ),
                button,
                pressed,
            )
        };
        chosen(state, event, dirty);
        return true;
    }
    if state.selection_translation.busy() {
        return true;
    }
    let Some(bar) = &mut state.selection_translation.bar else {
        return false;
    };
    let point = state.input.pointer.local;
    let inside = bar.monitor == state.input.pointer.monitor_idx && contains(bar.rect, point);
    if !inside && !state.selection_translation.bar_pressed {
        return false;
    }
    if matches!(button, MouseButton::Left) {
        state.selection_translation.bar_pressed = pressed;
    }
    let command = bar.button(point, button, pressed);
    if let Some(command) = command {
        action(state, command, dirty);
    }
    true
}

/// 【框选翻译】【键盘接入】按 chord 处理当前列表和任务取消，dirty 返回恢复范围，返回是否消费快捷键
pub(in crate::app) fn key(state: &mut EditorState, chord: Option<Chord>, dirty: &mut u32) -> bool {
    if let Some(popup) = &mut state.selection_translation.popup {
        let event = chord.map_or(Event::None, |c| popup.key(c.key, c.mods.shift));
        chosen(state, event, dirty);
        return true;
    }
    if state.selection_translation.busy() {
        if chord.is_some_and(|c| c.key == Key::Escape) {
            super::tasks::cancel(state, dirty);
            feedback(state, "Translation canceled", dirty);
        }
        return true;
    }
    if chord.is_some_and(|c| c.key == Key::Escape) && state.selection_translation.session.is_some()
    {
        super::exit(state, dirty);
        return true;
    }
    false
}
