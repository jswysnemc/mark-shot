//! 【文字识别】【原图菜单】保持实际 Qt 的编辑、贴图顺序，原生菜单销毁后才交接未经变换的原图

use super::{source_menu::Kind, window::ResultWindow};
use crate::{backend::window::WindowAction, ui::context_menu::Row};

#[cfg(test)]
mod tests;

/// 【文字识别】【原图菜单打开】在 position 为 state 打开独立原生菜单，返回窗口动作；不读取剪贴板，不平移原图
pub(super) fn open(state: &mut ResultWindow, position: (f64, f64)) -> WindowAction {
    super::source_menu::open_rows(
        state,
        position,
        Kind::Image,
        vec![
            Row {
                label: "Edit",
                shortcut: "",
                enabled: true,
            },
            Row {
                label: "Pin image",
                shortcut: "",
                enabled: true,
            },
        ],
    )
}

/// 【文字识别】【原图菜单交付】对 state 执行 index 对应编辑或贴图动作，返回重绘；原图快照与缩放、平移、正文编辑无关
pub(super) fn activate(state: &mut ResultWindow, index: usize) -> WindowAction {
    if index > 1 {
        return WindowAction::None;
    }
    log::info!(
        "【文字识别】【原图交接】开始交接，操作={index}，字节数={}",
        state.png.len()
    );
    let result = match index {
        0 => crate::features::pinned_image::actions::open(&state.png, false),
        1 => crate::backend::pin::spawn(&state.png, 1.0),
        _ => return WindowAction::None,
    };
    if let Err(error) = result {
        log::error!("【文字识别】【原图交接】操作失败: {error}");
        state.status = Some(format!("Image action failed: {error}"));
    }
    WindowAction::Redraw
}
