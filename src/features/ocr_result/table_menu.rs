//! 【文字识别】【表格菜单】表格复制拥有独立入口和原生菜单，不复用标题栏原文复制按钮

use super::{
    export::{self, Format},
    source_menu::Kind,
    window::ResultWindow,
};
use crate::{backend::window::WindowAction, ui::context_menu::Row};

const FORMATS: [Format; 4] = [Format::Tsv, Format::Markdown, Format::Csv, Format::Html];

/// 【文字识别】【表格菜单打开】确认 state 当前表格草稿后在复制按钮下方打开格式菜单，返回窗口动作，无其他输入
pub(super) fn open(state: &mut ResultWindow) -> WindowAction {
    let Some(table) = &mut state.table else {
        return WindowAction::None;
    };
    table.finish(true);
    let rect = state.layout.table_copy;
    let action = super::source_menu::open_rows(
        state,
        (f64::from(rect.left()), f64::from(rect.bottom() + 4.0)),
        Kind::Table,
        FORMATS
            .iter()
            .map(|format| Row {
                label: format.label(),
                shortcut: "",
                enabled: true,
            })
            .collect(),
    );
    state.source_menu.as_mut().unwrap().control_anchor = Some(rect);
    action
}

/// 【文字识别】【表格格式交付】按 index 导出 state 表格全文，原生菜单销毁后复制，返回窗口动作；不改变原文
pub(super) fn activate(state: &mut ResultWindow, index: usize) -> WindowAction {
    let Some(format) = FORMATS.get(index) else {
        return WindowAction::None;
    };
    let Some(table) = &state.table else {
        return WindowAction::None;
    };
    let text = export::text(&table.table, *format);
    state.copy_pane_text(&text, super::feedback::Pane::Table);
    WindowAction::Redraw
}
