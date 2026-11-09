//! 【全局快捷键】【X11注册】独立连接持有根窗口按键抓取，连接释放时自动注销

use super::super::{Action, Reporter};
use crate::{
    backend::x11::connection::Display,
    keys::{Chord, Key},
};
use std::os::fd::AsFd;
use x11rb::{
    connection::Connection,
    protocol::{Event, xproto::*},
};

/// 【全局快捷键】【符号映射】按key返回X11按键符号，无法跨布局表示的按键返回错误
fn keysym(key: Key) -> u32 {
    match key {
        Key::Char(character) => character as u32,
        Key::F(number) => 0xffbe + u32::from(number) - 1,
        Key::PrintScreen => 0xff61,
        Key::Escape => 0xff1b,
        Key::Enter => 0xff0d,
        Key::Space => 0x20,
        Key::Tab => 0xff09,
        Key::Backspace => 0xff08,
        Key::Delete => 0xffff,
        Key::Insert => 0xff63,
        Key::Home => 0xff50,
        Key::End => 0xff57,
        Key::PageUp => 0xff55,
        Key::PageDown => 0xff56,
        Key::Left => 0xff51,
        Key::Right => 0xff53,
        Key::Up => 0xff52,
        Key::Down => 0xff54,
    }
}

/// 【全局快捷键】【按键抓取】按display和chord注册所有锁定修饰组合，返回协议错误；冲突不抢占已有绑定
fn bind(display: &Display, chord: Chord) -> crate::AppResult {
    let setup = display.connection.setup();
    let count = setup.max_keycode - setup.min_keycode + 1;
    let mapping = display
        .connection
        .get_keyboard_mapping(setup.min_keycode, count)?
        .reply()?;
    let codes: Vec<_> = mapping
        .keysyms
        .chunks(usize::from(mapping.keysyms_per_keycode))
        .enumerate()
        .filter(|(_, symbols)| symbols.contains(&keysym(chord.key)))
        .map(|(index, _)| setup.min_keycode + index as u8)
        .collect();
    if codes.is_empty() {
        return Err("shortcut key is unavailable in this keyboard layout".into());
    }
    let mut modifiers = ModMask::default();
    for (enabled, mask) in [
        (chord.mods.ctrl, ModMask::CONTROL),
        (chord.mods.shift, ModMask::SHIFT),
        (chord.mods.alt, ModMask::M1),
        (chord.mods.logo, ModMask::M4),
    ] {
        if enabled {
            modifiers |= mask;
        }
    }
    let map = display.connection.get_modifier_mapping()?.reply()?;
    let mut ignored = u16::from(ModMask::LOCK);
    for (index, keycodes) in map
        .keycodes
        .chunks(usize::from(map.keycodes_per_modifier()))
        .enumerate()
    {
        if keycodes.iter().any(|code| {
            *code >= setup.min_keycode
                && mapping
                    .keysyms
                    .chunks(usize::from(mapping.keysyms_per_keycode))
                    .nth(usize::from(*code - setup.min_keycode))
                    .is_some_and(|symbols| symbols.contains(&0xff7f) || symbols.contains(&0xff14))
        }) {
            ignored |= 1 << index;
        }
    }
    for locks in 0u16..=255 {
        if locks & !ignored != 0 {
            continue;
        }
        for code in &codes {
            display
                .connection
                .grab_key(
                    false,
                    display.screen().root,
                    modifiers | ModMask::from(locks),
                    *code,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                )?
                .check()?;
        }
    }
    display.connection.flush()?;
    Ok(())
}

/// 【全局快捷键】【原生等待】按display、shortcut注册按键，通过reporter发送触发与状态，返回协议错误
pub(super) async fn run(
    display: Display,
    shortcut: &str,
    reporter: &Reporter,
) -> Result<(), String> {
    let chord = crate::keys::parse(shortcut)?;
    bind(&display, chord).map_err(|error| error.to_string())?;
    let descriptor = display
        .connection
        .as_fd()
        .try_clone_to_owned()
        .map_err(|error| error.to_string())?;
    // 1. 【全局快捷键】【描述符所有权】独立副本由AsyncFd持有，注册期间不会被外部关闭或替换
    let ready = unsafe { tokio::io::unix::AsyncFd::register(descriptor) }
        .map_err(|error| error.to_string())?;
    reporter.status(super::super::status::Registration::Registered(
        shortcut.into(),
    ));
    let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
    loop {
        while let Some(event) = display
            .connection
            .poll_for_event()
            .map_err(|error| error.to_string())?
        {
            match event {
                Event::KeyPress(_) if last.elapsed() >= std::time::Duration::from_millis(400) => {
                    last = std::time::Instant::now();
                    reporter.action(Action::Capture);
                }
                Event::MappingNotify(_) => {
                    display
                        .connection
                        .ungrab_key(0, display.screen().root, ModMask::ANY)
                        .map_err(|error| error.to_string())?;
                    bind(&display, chord).map_err(|error| error.to_string())?;
                }
                _ => {}
            }
        }
        ready
            .readable()
            .await
            .map_err(|error| error.to_string())?
            .clear_ready();
    }
}
