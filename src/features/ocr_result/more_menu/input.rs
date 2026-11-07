use super::*;
use crate::{
    backend::window::WindowPopupAction,
    keys::Key,
    types::{MouseButton, popup::PopupEvent},
    ui::context_menu::Event,
};

/// 【文字识别】【菜单命中】把 id 本地 point 转为父菜单坐标，返回最深可见目标及其本地坐标，整树之外为空值
fn target(state: &ResultWindow, id: u64, point: (f64, f64)) -> Option<(u64, (f64, f64))> {
    let root = state.source_menu.as_ref()?;
    let child = state.more_menu.child.as_ref();
    let offset = state
        .more_menu
        .child_position
        .unwrap_or((root.layout.size.0 as i32 - 2, 0));
    let parent_point = if child.is_some_and(|child| child.id == id) {
        (point.0 + f64::from(offset.0), point.1 + f64::from(offset.1))
    } else {
        point
    };
    if let Some(child) = child {
        let local = (
            parent_point.0 - f64::from(offset.0),
            parent_point.1 - f64::from(offset.1),
        );
        if local.0 >= 0.0
            && local.1 >= 0.0
            && local.0 < f64::from(child.layout.size.0)
            && local.1 < f64::from(child.layout.size.1)
        {
            return Some((child.id, local));
        }
    }
    (parent_point.0 >= 0.0
        && parent_point.1 >= 0.0
        && parent_point.0 < f64::from(root.layout.size.0)
        && parent_point.1 < f64::from(root.layout.size.1))
    .then_some((root.id, parent_point))
}

/// 【文字识别】【悬停调度】按 state 的 id、point 更新父高亮及 180 毫秒单次切换；进入子菜单取消父切换，无返回值
fn hover(state: &mut ResultWindow, id: u64, point: (f64, f64)) {
    let Some((selected, point)) = target(state, id, point) else {
        leave_root(state);
        return;
    };
    if state
        .more_menu
        .child
        .as_ref()
        .is_some_and(|child| child.id == selected)
    {
        state.more_menu.pending = None;
        return;
    }
    let root = state.source_menu.as_mut().unwrap();
    let row = root.item_at(point) == Some(0);
    let open = state.more_menu.child.is_some();
    if row == open {
        state.more_menu.pending = None;
    } else if state
        .more_menu
        .pending
        .is_none_or(|(wanted, _)| wanted != row)
    {
        state.more_menu.pending = Some((row, Instant::now() + Duration::from_millis(180)));
    }
    let active = if !row && open {
        Some(0)
    } else {
        row.then_some(0)
    };
    if root.active != active {
        root.active = active;
        root.dirty = true;
    }
}

/// 【文字识别】【悬停离开】清除 state 未展开父菜单的期限和高亮，无返回值；已展开子层的跨表面移动保留父状态
fn leave_root(state: &mut ResultWindow) {
    if state.more_menu.child.is_some() {
        return;
    }
    state.more_menu.pending = None;
    if let Some(root) = &mut state.source_menu
        && root.active.take().is_some()
    {
        root.dirty = true;
    }
}

/// 【文字识别】【菜单按键】按 key 和 text 导航最深层，关闭子层保留父层，禁用说明不能执行，无返回值外的窗口动作
fn key(
    state: &mut ResultWindow,
    key: Option<crate::keys::Chord>,
    text: Option<String>,
) -> WindowPopupAction {
    if let Some(child) = state.more_menu.child.as_ref() {
        if key
            .as_ref()
            .is_some_and(|chord| matches!(chord.key, Key::Left | Key::Escape))
        {
            state.more_menu.close_child(child.id);
            return WindowPopupAction::Redraw;
        }
        return WindowPopupAction::None;
    }
    let root = state.source_menu.as_mut().unwrap();
    if let Some(chord) = key {
        match chord.key {
            Key::Escape => return WindowPopupAction::CloseAll,
            Key::Left => return WindowPopupAction::None,
            Key::Right | Key::Enter | Key::Space if root.active == Some(0) => {
                show_child(state);
                return WindowPopupAction::Redraw;
            }
            _ => {
                if root.key(chord.key, chord.mods.shift) == Event::Changed {
                    return WindowPopupAction::Redraw;
                }
                if chord.mods.ctrl || chord.mods.logo || chord.mods.alt {
                    return WindowPopupAction::None;
                }
            }
        }
    }
    if text
        .as_deref()
        .is_some_and(|text| text.eq_ignore_ascii_case("k"))
        && root.active != Some(0)
    {
        root.active = Some(0);
        root.dirty = true;
        return WindowPopupAction::Redraw;
    }
    WindowPopupAction::None
}

/// 【文字识别】【分层事件】按 state、id、event 分派当前菜单链，外部点击关闭整树，旧编号不能影响当前原文
pub(super) fn event(state: &mut ResultWindow, id: u64, event: PopupEvent) -> WindowPopupAction {
    let Some(root) = &state.source_menu else {
        return WindowPopupAction::None;
    };
    if state.more_menu.owner != Some(root.id)
        || (id != root.id
            && !state
                .more_menu
                .child
                .as_ref()
                .is_some_and(|child| child.id == id))
    {
        return WindowPopupAction::None;
    }
    match event {
        PopupEvent::Configured { position } => {
            if let Some(child) = state
                .more_menu
                .child
                .as_mut()
                .filter(|child| child.id == id)
            {
                state.more_menu.child_position = Some(position);
                child.position = (0.0, 0.0);
            } else {
                state.source_menu.as_mut().unwrap().position = (0.0, 0.0);
            }
            WindowPopupAction::None
        }
        PopupEvent::Pointer { x, y } => {
            hover(state, id, (x, y));
            WindowPopupAction::Redraw
        }
        PopupEvent::Button {
            x,
            y,
            button,
            pressed,
        } => {
            let Some((selected, point)) = target(state, id, (x, y)) else {
                return if pressed {
                    WindowPopupAction::CloseAll
                } else {
                    WindowPopupAction::None
                };
            };
            if pressed
                && matches!(button, MouseButton::Left)
                && state
                    .source_menu
                    .as_ref()
                    .is_some_and(|root| root.id == selected && root.item_at(point) == Some(0))
            {
                show_child(state);
                return WindowPopupAction::Redraw;
            }
            WindowPopupAction::None
        }
        PopupEvent::Key { chord, text } => key(state, chord, text),
        PopupEvent::Leave => {
            leave_root(state);
            WindowPopupAction::Redraw
        }
        PopupEvent::Closed => WindowPopupAction::Close,
        PopupEvent::Scroll { .. } => WindowPopupAction::None,
    }
}
