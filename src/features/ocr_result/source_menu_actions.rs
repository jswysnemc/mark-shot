//! 【文字识别】【原文菜单动作】共享编辑器历史和纯文本剪贴板，不调用截图标注文字的业务命令

use super::window::ResultWindow;
use crate::{backend::wayland::window::WindowAction, ui::context_menu::Row};

const ITEMS: [(&str, &str); 9] = [
    ("Undo", "Ctrl+Z"),
    ("Redo", "Ctrl+Shift+Z"),
    ("", ""),
    ("Cut", "Ctrl+X"),
    ("Copy", "Ctrl+C"),
    ("Paste", "Ctrl+V"),
    ("Delete", "Del"),
    ("", ""),
    ("Select All", "Ctrl+A"),
];

/// 【文字识别】【菜单资格】根据 state 的正文、选择和历史以及 clipboard 是否有纯文本构造原版九行，返回菜单行
pub(super) fn rows(state: &ResultWindow, clipboard: bool) -> Vec<Row> {
    let selected = state.source.selected_text().is_some();
    let enabled = [
        state.source.can_undo(),
        state.source.can_redo(),
        false,
        selected,
        selected,
        clipboard,
        selected,
        false,
        !state.source.text().is_empty(),
    ];
    ITEMS
        .iter()
        .zip(enabled)
        .map(|(&(label, shortcut), enabled)| Row {
            label,
            shortcut,
            enabled,
        })
        .collect()
}

/// 【文字识别】【选区复制】将 text 写入持久剪贴板，state 仅在错误时显示反馈；返回是否成功，不触发全文复制反馈
fn copy(state: &mut ResultWindow, text: &str) -> bool {
    log::info!("【文字识别】【选区复制】开始复制，字节数={}", text.len());
    match crate::utils::copy_to_clipboard(text) {
        Ok(()) => true,
        Err(error) => {
            log::error!("【文字识别】【选区复制】{error}");
            state.status = Some(format!("Copy failed: {error}"));
            false
        }
    }
}

/// 【文字识别】【菜单执行】对 state 执行 index 对应已启用动作；原生表面已销毁，重新检查选择和历史，返回重绘动作
pub(super) fn activate(state: &mut ResultWindow, index: usize) -> WindowAction {
    let changed = match index {
        0 => state.source.undo(&mut state.fonts),
        1 => state.source.redo(&mut state.fonts),
        3 | 4 => {
            let Some(text) = state.source.selected_text() else {
                return WindowAction::None;
            };
            let copied = copy(state, &text);
            copied && index == 3 && state.source.delete_selection(&mut state.fonts)
        }
        5 => {
            let Some(text) = crate::utils::paste_from_clipboard().filter(|text| !text.is_empty())
            else {
                return WindowAction::None;
            };
            state.ensure_fonts(&text);
            state.source.insert(&mut state.fonts, &text)
        }
        6 => state.source.delete_selection(&mut state.fonts),
        8 => {
            state.source.select_all(&mut state.fonts);
            false
        }
        _ => return WindowAction::None,
    };
    if changed {
        state.status = None;
        state.copied = false;
    }
    WindowAction::Redraw
}
