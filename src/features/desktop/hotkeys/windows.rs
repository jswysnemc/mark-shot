//! 【全局快捷键】【Windows注册】RegisterHotKey冲突直接报告，退出前注销按键

use super::super::{Action, Reporter, native_thread};
use crate::keys::Key;
use windows_sys::Win32::UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*};

/// 【全局快捷键】【虚拟按键】按key返回系统虚拟键，字符使用当前键盘布局转换
fn virtual_key(key: Key) -> Result<u32, String> {
    let code = match key {
        Key::Char(character) => {
            let code = unsafe { VkKeyScanW(character as u16) };
            if code == -1 {
                return Err("key unavailable in current layout".into());
            }
            code as u16 & 0xff
        }
        Key::F(number) => VK_F1 + u16::from(number) - 1,
        Key::PrintScreen => VK_SNAPSHOT,
        Key::Escape => VK_ESCAPE,
        Key::Enter => VK_RETURN,
        Key::Space => VK_SPACE,
        Key::Tab => VK_TAB,
        Key::Backspace => VK_BACK,
        Key::Delete => VK_DELETE,
        Key::Insert => VK_INSERT,
        Key::Home => VK_HOME,
        Key::End => VK_END,
        Key::PageUp => VK_PRIOR,
        Key::PageDown => VK_NEXT,
        Key::Left => VK_LEFT,
        Key::Right => VK_RIGHT,
        Key::Up => VK_UP,
        Key::Down => VK_DOWN,
    };
    Ok(u32::from(code))
}

/// 【全局快捷键】【注册线程】按shortcut、reporter和cancelled运行系统按键消息循环，返回错误
pub(super) async fn run(
    shortcut: String,
    reporter: Reporter,
    cancelled: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    let chord = crate::keys::parse(&shortcut)?;
    let key = virtual_key(chord.key)?;
    let mut modifiers = MOD_NOREPEAT;
    for (enabled, mask) in [
        (chord.mods.ctrl, MOD_CONTROL),
        (chord.mods.alt, MOD_ALT),
        (chord.mods.shift, MOD_SHIFT),
        (chord.mods.logo, MOD_WIN),
    ] {
        if enabled {
            modifiers |= mask;
        }
    }
    native_thread::run(cancelled, move |ready| unsafe {
        if RegisterHotKey(std::ptr::null_mut(), 1, modifiers, key) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let result = if native_thread::ready(ready) {
            reporter.status(super::super::status::Registration::Registered(
                shortcut.clone(),
            ));
            let mut message = std::mem::zeroed();
            loop {
                let result = GetMessageW(&mut message, std::ptr::null_mut(), 0, 0);
                if result <= 0 {
                    break if result == 0 {
                        Ok(())
                    } else {
                        Err(std::io::Error::last_os_error().to_string())
                    };
                }
                if message.message == WM_HOTKEY && message.wParam == 1 {
                    reporter.action(Action::Capture);
                }
            }
        } else {
            Ok(())
        };
        UnregisterHotKey(std::ptr::null_mut(), 1);
        result
    })
    .await
}
