//! 【文字菜单】【业务动作】提供原版菜单顺序和打开时可用性，动作关闭菜单后交给编辑器局部事务

use crate::{
    editor::{DamageZone, EditorState, dirty::mark_dirty},
    tools::text::session_input::{self, Command},
    ui::context_menu::{ContextMenu, Event, Row},
};

pub const ITEMS: &[(&str, &str, Option<Command>)] = &[
    ("Undo", "Ctrl+Z", Some(Command::Undo)),
    ("Redo", "Ctrl+Shift+Z", Some(Command::Redo)),
    ("", "", None),
    ("Cut", "Ctrl+X", Some(Command::Cut)),
    ("Copy", "Ctrl+C", Some(Command::Copy)),
    ("Paste", "Ctrl+V", Some(Command::Paste)),
    ("Delete", "Del", Some(Command::DeleteSelection)),
    ("", "", None),
    ("Select All", "Ctrl+A", Some(Command::SelectAll)),
];

/// 【文字菜单】【动作状态】根据 state 和 clipboard_text 返回打开时菜单行，剪贴板内容只用于可用性，不代替实际粘贴
pub fn rows(state: &EditorState, clipboard_text: bool) -> Vec<Row> {
    let selection = session_input::has_selection(state);
    let session = state.text_session.as_ref();
    let text = session.is_some_and(|session| matches!(&session.working.shape, crate::types::AnnotationShape::Text { content, .. } if !content.is_empty()));
    ITEMS
        .iter()
        .map(|&(label, shortcut, command)| {
            let enabled = match command {
                Some(Command::Undo | Command::Redo) => session.is_some_and(|session| {
                    session.history.available(command == Some(Command::Redo))
                }),
                Some(Command::Cut | Command::Copy | Command::DeleteSelection) => selection,
                Some(Command::Paste) => clipboard_text,
                Some(Command::SelectAll) => text,
                None => true,
            };
            Row {
                label,
                shortcut,
                enabled,
            }
        })
        .collect()
}

/// 【文字菜单】【重绘范围】将 state 当前菜单加入 dirty，不改变字符正文、历史或工具，无返回值
pub fn damage(state: &mut EditorState, dirty: &mut u32) {
    if let Some(menu) = &state.text_context_menu {
        state.damage_rects.push(DamageZone::Local {
            monitor_idx: menu.monitor_idx,
            rect: menu.rect(),
        });
        mark_dirty(dirty, menu.monitor_idx);
    }
}

/// 【文字菜单】【菜单关闭】释放 state 菜单并通过 dirty 恢复覆盖区域，不提交文字或改变选区，无返回值
pub fn close(state: &mut EditorState, dirty: &mut u32) {
    damage(state, dirty);
    state.text_context_menu = None;
}

/// 【文字菜单】【菜单打开】在 state 指针位置打开原版菜单，dirty 返回范围；保留光标与字符选区，不产生历史
pub fn open(state: &mut EditorState, dirty: &mut u32) {
    close(state, dirty);
    let Some(session) = state.text_session.as_mut() else {
        return;
    };
    session.dragging = false;
    state.input.left_canvas = false;
    state.input.clicks.clear();
    state.input.right_clicks.clear();
    let clipboard = crate::utils::paste_from_clipboard().is_some_and(|text| !text.is_empty());
    let rows = rows(state, clipboard);
    let pointer = state.input.pointer;
    let bounds = state.placements[pointer.monitor_idx].size;
    state.text_context_menu = Some(ContextMenu::new(
        rows,
        &mut state.text.font_system,
        (
            pointer.local.0.round() as f32,
            pointer.local.1.round() as f32,
        ),
        (bounds.0 as f32, bounds.1 as f32),
        pointer.monitor_idx,
    ));
    state.text_context_menu.as_mut().unwrap().id = state.next_menu_id;
    state.next_menu_id = state.next_menu_id.wrapping_add(1).max(1);
    damage(state, dirty);
}

/// 【文字菜单】【事件交付】处理 state 的 event，dirty 返回重绘范围；只有可用行执行动作，并且先关闭菜单再编辑
pub fn event(state: &mut EditorState, event: Event, dirty: &mut u32) {
    match event {
        Event::None => {}
        Event::Changed => damage(state, dirty),
        Event::Close => close(state, dirty),
        Event::Activate(index) => {
            let command = state
                .text_context_menu
                .as_ref()
                .and_then(|menu| menu.rows.get(index))
                .filter(|row| row.enabled)
                .and_then(|_| ITEMS.get(index))
                .and_then(|item| item.2);
            if let Some(command) = command {
                let native = state
                    .text_context_menu
                    .as_ref()
                    .is_some_and(|menu| menu.native);
                close(state, dirty);
                if native {
                    state.pending_text_menu_action = Some(command);
                } else {
                    session_input::command(state, command, dirty);
                }
            }
        }
    }
}
