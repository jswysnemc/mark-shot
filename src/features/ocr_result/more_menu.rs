//! 【文字识别】【更多菜单】真实快捷键子菜单和单次悬停期限，不执行禁用的说明动作

mod input;
#[cfg(test)]
mod tests;

use super::{source_menu, window::ResultWindow};
use crate::{
    backend::window::WindowAction,
    types::{
        popup::PopupFrame,
        popup_tree::{Placement, PopupNode},
    },
    ui::context_menu::{ContextMenu, Row},
};
use std::time::{Duration, Instant};

pub(super) const SHORTCUTS: [Row; 3] = [
    Row {
        label: "Translate",
        shortcut: "Ctrl+Enter",
        enabled: false,
    },
    Row {
        label: "Copy all text",
        shortcut: "Ctrl+Shift+C",
        enabled: false,
    },
    Row {
        label: "Undo edit",
        shortcut: "Ctrl+Z",
        enabled: false,
    },
];

#[derive(Default)]
pub(super) struct MoreMenu {
    owner: Option<u64>,
    pub child: Option<ContextMenu>,
    child_position: Option<(i32, i32)>,
    pending: Option<(bool, Instant)>,
}

impl MoreMenu {
    /// 【文字识别】【菜单清理】取消子菜单和单次悬停期限，无参数、无返回值；不修改正文或剪贴板
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// 【文字识别】【菜单期限】按 root 返回当前父会话的最近悬停期限，不启动线程，旧会话没有期限
    pub fn deadline(&self, root: Option<u64>) -> Option<Instant> {
        self.pending
            .filter(|_| self.owner == root && root.is_some())
            .map(|(_, deadline)| deadline)
    }

    /// 【文字识别】【父子节点】按 menu 返回存活子菜单的父关系及双向重叠定位，未打开或会话不符时返回空值
    pub fn child_node(&self, menu: &ContextMenu) -> Option<PopupNode> {
        let child = self
            .child
            .as_ref()
            .filter(|_| self.owner == Some(menu.id))?;
        Some(PopupNode {
            id: child.id,
            parent: Some(menu.id),
            placement: Placement::Submenu {
                parent_width: menu.layout.size.0,
                row_top: menu.layout.rows[0].top(),
                item_inset: child.layout.rows[0].top(),
            },
        })
    }

    /// 【文字识别】【子菜单取消】仅清除 id 对应当前子菜单，返回是否发生变化；不能关闭新会话或父菜单
    pub fn close_child(&mut self, id: u64) -> bool {
        if self.child.as_ref().is_none_or(|child| child.id != id) {
            return false;
        }
        self.child = None;
        self.child_position = None;
        self.pending = None;
        true
    }
}

/// 【文字识别】【更多打开】按 state 在真实按钮下方打开原版父菜单，返回窗口动作，子菜单初始不显示
pub(super) fn open(state: &mut ResultWindow) -> WindowAction {
    let bounds = state.layout.more;
    let action = source_menu::open_rows(
        state,
        (f64::from(bounds.left()), f64::from(bounds.bottom() + 4.0)),
        source_menu::Kind::More,
        vec![Row {
            label: "Keyboard shortcuts",
            shortcut: "",
            enabled: true,
        }],
    );
    let menu = state.source_menu.as_mut().unwrap();
    menu.control_anchor = Some(bounds);
    menu.submenu(0, &mut state.fonts);
    state.more_menu.owner = Some(menu.id);
    action
}

/// 【文字识别】【子菜单展开】为 state 当前父会话打开真实禁用说明菜单，无返回值；重复展开不创建新编号
fn show_child(state: &mut ResultWindow) {
    let Some(root) = state
        .source_menu
        .as_mut()
        .filter(|root| state.more_menu.owner == Some(root.id))
    else {
        return;
    };
    state.more_menu.pending = None;
    if root.active != Some(0) {
        root.active = Some(0);
        root.dirty = true;
    }
    if state.more_menu.child.is_some() {
        return;
    }
    let mut child = ContextMenu::new(
        SHORTCUTS
            .iter()
            .map(|row| Row {
                label: state.language.text(row.label),
                ..*row
            })
            .collect(),
        &mut state.fonts,
        (0.0, 0.0),
        state.layout.size,
        0,
    );
    child.id = crate::features::text::input_method::focus::identity();
    state.more_menu.child = Some(child);
}

/// 【文字识别】【悬停到期】在 now 消费 state 当前单次期限，返回是否需要重绘；父会话已经关闭时不会展开
pub(super) fn wake(state: &mut ResultWindow, now: Instant) -> bool {
    let Some((open, deadline)) = state
        .more_menu
        .pending
        .filter(|(_, deadline)| *deadline <= now)
    else {
        return false;
    };
    state.more_menu.pending = None;
    if state.source_menu.as_ref().map(|menu| menu.id) != state.more_menu.owner {
        return false;
    }
    if open {
        show_child(state);
    } else if let Some(id) = state.more_menu.child.as_ref().map(|child| child.id) {
        state.more_menu.close_child(id);
        if let Some(root) = &mut state.source_menu {
            root.active = None;
            root.dirty = true;
        }
    }
    let _ = deadline;
    true
}

/// 【文字识别】【子菜单像素】按 state、id、scale 返回变化后的独立子菜单帧，不能把两层菜单画进同一表面
pub(super) fn frame(state: &mut ResultWindow, id: u64, scale: f32) -> Option<PopupFrame<'_>> {
    let child = state
        .more_menu
        .child
        .as_mut()
        .filter(|child| child.id == id)?;
    source_menu::frame_menu(child, &mut state.fonts, &mut state.cache, scale)
}

/// 【文字识别】【菜单事件】按 state、id、event 分派当前父子菜单，返回窗口动作，不修改原文
pub(super) fn event(
    state: &mut ResultWindow,
    id: u64,
    event: crate::types::popup::PopupEvent,
) -> crate::backend::window::WindowPopupAction {
    input::event(state, id, event)
}
