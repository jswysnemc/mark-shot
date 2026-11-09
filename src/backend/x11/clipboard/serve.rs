//! 【X11剪贴板】【数据拥有者】确认所有权后通知父进程，持续服务至其他应用接管

use super::{CHUNK, LIMIT};
use crate::backend::x11::{connection::Display, surface::Surface};
use std::{
    collections::HashMap,
    io::{Read, Write},
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::{Event, xproto::*},
    wrapper::ConnectionExt as _,
};

struct Transfer {
    offset: usize,
    expires: Instant,
    target: Atom,
}

/// 【X11剪贴板】【独立服务】按text与primary读取有界标准输入并服务选择区，返回接管结束或协议错误
pub fn serve(text: bool, primary: bool) -> crate::AppResult {
    let display = Display::connect()?;
    let surface = Surface::new(display.clone(), (0, 0), (1, 1), true)?;
    let selection = if primary {
        AtomEnum::PRIMARY.into()
    } else {
        display.atom("CLIPBOARD")?
    };
    let target = display.atom(if text { "UTF8_STRING" } else { "image/png" })?;
    let targets = display.atom("TARGETS")?;
    let incr = display.atom("INCR")?;
    let utf8_mime = display.atom("text/plain;charset=utf-8")?;
    let plain = display.atom("text/plain")?;
    let mut bytes = Vec::new();
    std::io::stdin()
        .lock()
        .take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > LIMIT {
        return Err("clipboard exceeds 128 MiB".into());
    }
    display
        .connection
        .set_selection_owner(surface.window, selection, x11rb::CURRENT_TIME)?
        .check()?;
    if display
        .connection
        .get_selection_owner(selection)?
        .reply()?
        .owner
        != surface.window
    {
        return Err("X11 clipboard ownership was not granted".into());
    }
    writeln!(std::io::stdout())?;
    std::io::stdout().flush()?;
    let mut transfers = HashMap::<(Window, Atom), Transfer>::new();
    let wake = crate::backend::task_wake::Source::new()?;
    loop {
        while let Some(event) = display.connection.poll_for_event()? {
            match event {
                Event::SelectionClear(event) if event.selection == selection => return Ok(()),
                Event::SelectionRequest(event) => {
                    let property = if event.property == x11rb::NONE {
                        event.target
                    } else {
                        event.property
                    };
                    let result = if event.selection != selection {
                        x11rb::NONE
                    } else if event.target == targets {
                        let supported = if text {
                            vec![targets, target, utf8_mime, plain]
                        } else {
                            vec![targets, target]
                        };
                        display
                            .connection
                            .change_property32(
                                PropMode::REPLACE,
                                event.requestor,
                                property,
                                AtomEnum::ATOM,
                                &supported,
                            )?
                            .check()?;
                        property
                    } else if event.target == target
                        || text && [utf8_mime, plain].contains(&event.target)
                    {
                        if bytes.len() <= CHUNK {
                            display
                                .connection
                                .change_property8(
                                    PropMode::REPLACE,
                                    event.requestor,
                                    property,
                                    event.target,
                                    &bytes,
                                )?
                                .check()?;
                        } else {
                            display
                                .connection
                                .change_window_attributes(
                                    event.requestor,
                                    &ChangeWindowAttributesAux::new().event_mask(
                                        EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY,
                                    ),
                                )?
                                .check()?;
                            display
                                .connection
                                .change_property32(
                                    PropMode::REPLACE,
                                    event.requestor,
                                    property,
                                    incr,
                                    &[bytes.len() as u32],
                                )?
                                .check()?;
                            transfers.insert(
                                (event.requestor, property),
                                Transfer {
                                    offset: 0,
                                    expires: Instant::now() + Duration::from_secs(10),
                                    target: event.target,
                                },
                            );
                        }
                        property
                    } else {
                        x11rb::NONE
                    };
                    display
                        .connection
                        .send_event(
                            false,
                            event.requestor,
                            EventMask::NO_EVENT,
                            SelectionNotifyEvent {
                                response_type: SELECTION_NOTIFY_EVENT,
                                sequence: 0,
                                time: event.time,
                                requestor: event.requestor,
                                selection,
                                target: event.target,
                                property: result,
                            },
                        )?
                        .check()?;
                }
                Event::PropertyNotify(event) if event.state == Property::DELETE => {
                    let key = (event.window, event.atom);
                    if let Some(transfer) = transfers.get_mut(&key) {
                        let end = (transfer.offset + CHUNK).min(bytes.len());
                        let result = display
                            .connection
                            .change_property8(
                                PropMode::REPLACE,
                                event.window,
                                event.atom,
                                transfer.target,
                                &bytes[transfer.offset..end],
                            )?
                            .check();
                        if result.is_err() || transfer.offset == end {
                            transfers.remove(&key);
                        } else {
                            transfer.offset = end;
                            transfer.expires = Instant::now() + Duration::from_secs(10);
                        }
                    }
                }
                Event::DestroyNotify(event) => {
                    transfers.retain(|(window, _), _| *window != event.window)
                }
                _ => {}
            }
        }
        display.connection.flush()?;
        transfers.retain(|_, transfer| transfer.expires > Instant::now());
        let timeout = transfers
            .values()
            .map(|transfer| transfer.expires.saturating_duration_since(Instant::now()))
            .min();
        crate::backend::x11::wait::wait(&display, &wake, timeout)?;
    }
}
