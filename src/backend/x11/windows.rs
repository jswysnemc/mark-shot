//! 【X11窗口识别】【可见快照】按EWMH堆叠顺序读取映射窗口的桌面矩形

use super::connection::Display;
use crate::backend::window_detection::Window;
use x11rb::protocol::xproto::*;

/// 【X11窗口识别】【单次查询】按display返回当前桌面可见客户区，忽略已销毁或隐藏窗口，返回协议错误
pub fn snapshot(display: &Display) -> crate::AppResult<Vec<Window>> {
    let root = display.screen().root;
    let mut clients = display.property(root, "_NET_CLIENT_LIST_STACKING")?;
    if clients.is_empty() {
        clients = display.connection.query_tree(root)?.reply()?.children;
    }
    let desktop = display
        .property(root, "_NET_CURRENT_DESKTOP")?
        .first()
        .copied();
    let mut windows = Vec::new();
    for (order, client) in clients.into_iter().enumerate() {
        let result = (|| -> crate::AppResult<Option<Window>> {
            let attributes = display.connection.get_window_attributes(client)?.reply()?;
            if attributes.map_state != MapState::VIEWABLE || attributes.override_redirect {
                return Ok(None);
            }
            if let (Some(current), Some(window)) = (
                desktop,
                display
                    .property(client, "_NET_WM_DESKTOP")?
                    .first()
                    .copied(),
            ) && current != window
                && window != u32::MAX
            {
                return Ok(None);
            }
            let geometry = display.connection.get_geometry(client)?.reply()?;
            let position = display
                .connection
                .translate_coordinates(client, root, 0, 0)?
                .reply()?;
            Ok(tiny_skia::Rect::from_xywh(
                position.dst_x.into(),
                position.dst_y.into(),
                geometry.width.into(),
                geometry.height.into(),
            )
            .map(|rect| Window {
                rect,
                z_order: Some(order as i32),
            }))
        })();
        if let Ok(Some(window)) = result {
            windows.push(window);
        }
    }
    Ok(windows)
}
