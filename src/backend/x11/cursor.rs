//! 【X11光标】【共享资源】普通窗口与覆盖层使用同一标准指针映射，状态未变化时不重复创建

use super::connection::Display;
use crate::types::CursorIcon;
use x11rb::{connection::Connection, protocol::xproto::*};

/// 【X11光标】【资源交付】按display、windows和icon设置核心光标，返回协议结果并回收临时资源
pub(super) fn set(
    display: &Display,
    windows: impl IntoIterator<Item = Window>,
    icon: CursorIcon,
) -> crate::AppResult {
    let glyph = match icon {
        CursorIcon::Default => 68,
        CursorIcon::Crosshair => 34,
        CursorIcon::Pointer => 60,
        CursorIcon::Progress => 150,
        CursorIcon::Text => 152,
        CursorIcon::Grab | CursorIcon::Grabbing => 52,
        CursorIcon::NsResize => 116,
        CursorIcon::EwResize => 108,
        CursorIcon::NeswResize => 136,
        CursorIcon::NwseResize => 134,
    };
    let font = display.connection.generate_id()?;
    let cursor = display.connection.generate_id()?;
    display.connection.open_font(font, b"cursor")?.check()?;
    let result = (|| {
        display
            .connection
            .create_glyph_cursor(
                cursor,
                font,
                font,
                glyph,
                glyph + 1,
                0,
                0,
                0,
                65535,
                65535,
                65535,
            )?
            .check()?;
        let result = (|| {
            for window in windows {
                display
                    .connection
                    .change_window_attributes(
                        window,
                        &ChangeWindowAttributesAux::new().cursor(cursor),
                    )?
                    .check()?;
            }
            Ok(())
        })();
        display.connection.free_cursor(cursor)?;
        result
    })();
    display.connection.close_font(font)?;
    result
}
