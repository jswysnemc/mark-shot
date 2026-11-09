//! 【X11剪贴板】【有界接收】读取普通属性或ICCCM增量传输并限制总大小与等待时间

use super::LIMIT;
use crate::backend::x11::{connection::Display, surface::Surface};
use std::time::{Duration, Instant};
use x11rb::{
    connection::Connection,
    protocol::{Event, xproto::*},
};

/// 【X11剪贴板】【读取来源】按mime、primary和limit请求选择区，返回完整字节或超时、大小限制错误
pub fn read(mime: &str, primary: bool, limit: usize) -> crate::AppResult<Vec<u8>> {
    let display = Display::connect()?;
    let surface = Surface::new(display.clone(), (0, 0), (1, 1), true)?;
    display
        .connection
        .change_window_attributes(
            surface.window,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )?
        .check()?;
    let selection = if primary {
        AtomEnum::PRIMARY.into()
    } else {
        display.atom("CLIPBOARD")?
    };
    let target = display.atom(mime)?;
    let property = display.atom("MARK_SHOT_SELECTION")?;
    let incr = display.atom("INCR")?;
    display.connection.convert_selection(
        surface.window,
        selection,
        target,
        property,
        x11rb::CURRENT_TIME,
    )?;
    display.connection.flush()?;
    let limit = limit.min(LIMIT);
    let deadline = Instant::now() + Duration::from_secs(3);
    let wake = crate::backend::task_wake::Source::new()?;
    let mut incremental = false;
    let mut bytes = Vec::new();
    loop {
        while let Some(event) = display.connection.poll_for_event()? {
            let ready = match event {
                Event::SelectionNotify(event) if event.selection == selection => {
                    if event.property == x11rb::NONE {
                        return Err("clipboard does not offer the requested format".into());
                    }
                    true
                }
                Event::PropertyNotify(event) => {
                    incremental
                        && event.window == surface.window
                        && event.atom == property
                        && event.state == Property::NEW_VALUE
                }
                _ => false,
            };
            if !ready {
                continue;
            }
            let reply = display
                .connection
                .get_property(
                    true,
                    surface.window,
                    property,
                    AtomEnum::ANY,
                    0,
                    (limit.saturating_sub(bytes.len()).div_ceil(4) + 1) as u32,
                )?
                .reply()?;
            if reply.type_ == incr {
                incremental = true;
                if reply
                    .value32()
                    .and_then(|mut values| values.next())
                    .is_some_and(|size| size as usize > limit)
                {
                    return Err("clipboard exceeds the byte limit".into());
                }
            } else {
                if reply.bytes_after > 0 || bytes.len() + reply.value.len() > limit {
                    return Err("clipboard exceeds the byte limit".into());
                }
                let finished = !incremental || reply.value.is_empty();
                bytes.extend_from_slice(&reply.value);
                if finished {
                    return Ok(bytes);
                }
            }
            display.connection.flush()?;
        }
        let timeout = deadline.saturating_duration_since(Instant::now());
        if timeout.is_zero() {
            return Err("X11 clipboard transfer timed out".into());
        }
        crate::backend::x11::wait::wait(&display, &wake, Some(timeout))?;
    }
}
