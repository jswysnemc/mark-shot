//! 【属性面板】【停靠避让】属性栏避开主栏及动作栏，始终放在固定边缘的内侧

use crate::{config::capture::ToolbarPosition, editor::EditorState, ui::panel::UiPanel};

/// 【属性面板】【可用宽度】按 state 返回扣除侧边工具栏后的宽度，横向停靠和图片窗口沿用整屏宽度
pub(crate) fn available_width(state: &EditorState) -> f32 {
    let toolbar = &state.toolbar;
    let full = state.placements[toolbar.monitor_idx].size.0 as f32 - 16.0;
    let Some(main) = toolbar.rect().filter(|_| state.file_canvas.is_none()) else {
        return full;
    };
    let side = state
        .action_toolbar
        .rect()
        .filter(|_| state.action_toolbar.monitor_idx == toolbar.monitor_idx);
    match toolbar.dock_override.unwrap_or(toolbar.docking) {
        ToolbarPosition::Left => {
            full - side.map_or(main.right(), |side| main.right().max(side.right()))
        }
        ToolbarPosition::Right => {
            side.map_or(main.left(), |side| main.left().min(side.left())) - 16.0
        }
        _ => full,
    }
    .max(160.0)
}

/// 【属性面板】【固定位置】按 state 返回停靠模式下的位置，跟随选区及图片窗口返回空值
pub(super) fn position(state: &EditorState) -> Option<((f32, f32), usize)> {
    if state.file_canvas.is_some() {
        return None;
    }
    let toolbar = &state.toolbar;
    let mode = toolbar.dock_override.unwrap_or(toolbar.docking);
    if mode == ToolbarPosition::Selection {
        return None;
    }
    let main = toolbar.rect()?;
    let side = state
        .action_toolbar
        .rect()
        .filter(|_| state.action_toolbar.monitor_idx == toolbar.monitor_idx);
    let (width, height) = state.settings_panel.size;
    let bounds = state.placements[toolbar.monitor_idx].size;
    let (x, y) = match mode {
        ToolbarPosition::Top => (
            (bounds.0 as f32 - width) / 2.0,
            side.map_or(main.bottom(), |side| main.bottom().max(side.bottom())) + 8.0,
        ),
        ToolbarPosition::Bottom => (
            (bounds.0 as f32 - width) / 2.0,
            side.map_or(main.top(), |side| main.top().min(side.top())) - height - 8.0,
        ),
        ToolbarPosition::Right => (
            side.map_or(main.left(), |side| main.left().min(side.left())) - width - 8.0,
            (bounds.1 as f32 - height) / 2.0,
        ),
        ToolbarPosition::Left => (
            side.map_or(main.right(), |side| main.right().max(side.right())) + 8.0,
            (bounds.1 as f32 - height) / 2.0,
        ),
        ToolbarPosition::Selection => return None,
    };
    Some((
        (
            x.clamp(8.0, (bounds.0 as f32 - width - 8.0).max(8.0)),
            y.clamp(8.0, (bounds.1 as f32 - height - 8.0).max(8.0)),
        ),
        toolbar.monitor_idx,
    ))
}
