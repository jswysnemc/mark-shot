//! 【输入法】【编辑组合】候选拥有独立显示身份，真实正文和撤销仍由原编辑器管理

use cosmic_text::Cursor;

use crate::{editor::EditorState, types::text_input::Edit};

mod apply;
mod preview;

pub const PREEDIT_ID: u64 = u64::MAX - 1;

pub struct Composition {
    pub annotation: crate::types::Annotation,
    pub preedit: crate::types::text_input::Preedit,
    pub position: Cursor,
}

/// 【输入法】【正文交付】将 event 应用于 state 当前会话，dirty 返回动态区域，返回是否存在可用编辑目标
pub fn apply(state: &mut EditorState, event: Edit, dirty: &mut u32) -> bool {
    let Some(plan) = apply::prepare(state, &event) else {
        return false;
    };
    preview::cancel(state, dirty);
    apply::confirm(state, &event, plan, dirty);
    if !event.preedit.text.is_empty() {
        preview::show(state, event.preedit, dirty);
    }
    true
}

/// 【输入法】【候选取消】取消 state 的候选预览，dirty 返回旧显示范围，返回是否存在候选；不回填已经删除的选区
pub fn cancel(state: &mut EditorState, dirty: &mut u32) -> bool {
    preview::cancel(state, dirty)
}

/// 【输入法】【样式刷新】state 文档外观改变时更新候选预览，dirty 返回新旧范围；未改变样式的闪烁帧不复制全文，无返回值
pub fn refresh_style(state: &mut EditorState, dirty: &mut u32) {
    preview::refresh_style(state, dirty);
}
