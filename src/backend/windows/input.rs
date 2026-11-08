//! 【Windows输入】【共享转换】原生字符消息与快捷键只交付一次，UTF16代理对合并为完整字符

use super::native::{Event, Modifiers};
use crate::{
    backend::window::{KeyInput, ScrollInput, ScrollSource, WindowEvent},
    keys::{Key, Mods},
};

#[derive(Default)]
pub(super) struct Decoder {
    pending: Option<(Key, Mods)>,
    high: Option<u16>,
}

/// 【Windows输入】【修饰转换】按modifiers返回共享修饰键快照，不读取变化后的系统键盘状态
pub(super) fn mods(modifiers: Modifiers) -> Mods {
    Mods {
        ctrl: modifiers.ctrl,
        shift: modifiers.shift,
        alt: modifiers.alt,
        logo: modifiers.super_key,
    }
}

/// 【Windows输入】【按键转换】按虚拟code返回共享按键，未知媒体键返回空值，不伪造正文
fn key(code: u32) -> Option<Key> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    Some(match code as u16 {
        VK_ESCAPE => Key::Escape,
        VK_RETURN => Key::Enter,
        VK_SPACE => Key::Space,
        VK_TAB => Key::Tab,
        VK_BACK => Key::Backspace,
        VK_DELETE => Key::Delete,
        VK_INSERT => Key::Insert,
        VK_HOME => Key::Home,
        VK_END => Key::End,
        VK_PRIOR => Key::PageUp,
        VK_NEXT => Key::PageDown,
        VK_LEFT => Key::Left,
        VK_RIGHT => Key::Right,
        VK_UP => Key::Up,
        VK_DOWN => Key::Down,
        VK_OEM_4 => Key::Char('['),
        VK_OEM_6 => Key::Char(']'),
        VK_OEM_MINUS => Key::Char('-'),
        VK_OEM_PLUS => Key::Char('='),
        VK_OEM_1 => Key::Char(';'),
        VK_OEM_7 => Key::Char('\''),
        VK_OEM_COMMA => Key::Char(','),
        VK_OEM_PERIOD => Key::Char('.'),
        VK_OEM_2 => Key::Char('/'),
        VK_OEM_5 => Key::Char('\\'),
        VK_OEM_3 => Key::Char('`'),
        value if (VK_F1..=VK_F24).contains(&value) => Key::F((value - VK_F1 + 1) as u8),
        value if (0x30..=0x39).contains(&value) || (0x41..=0x5a).contains(&value) => {
            Key::Char((value as u8 as char).to_ascii_lowercase())
        }
        _ => return None,
    })
}

impl Decoder {
    /// 【Windows输入】【事件转换】按event与scale返回共享窗口事件；普通可打印按键等待字符消息，修饰快捷键立即交付
    pub(super) fn event(&mut self, event: Event, scale: f32) -> Option<WindowEvent> {
        let divide = |point: (f64, f64)| (point.0 / f64::from(scale), point.1 / f64::from(scale));
        Some(match event {
            Event::Resize { size, .. } => WindowEvent::Resize {
                size: (
                    (size.0 as f32 / scale).round().max(1.0) as u32,
                    (size.1 as f32 / scale).round().max(1.0) as u32,
                ),
            },
            Event::Focus { focused, .. } => {
                self.pending = None;
                self.high = None;
                WindowEvent::Focused(focused)
            }
            Event::Pointer { position, .. } => WindowEvent::PointerMove {
                position: divide(position),
            },
            Event::Button {
                position,
                button,
                pressed,
                ..
            } => WindowEvent::PointerButton {
                position: divide(position),
                button,
                pressed,
            },
            Event::Wheel {
                position,
                amount,
                horizontal,
                modifiers,
                ..
            } => {
                let angle = if horizontal {
                    (f64::from(amount), 0.0)
                } else {
                    (0.0, f64::from(amount))
                };
                WindowEvent::Wheel(ScrollInput {
                    position: divide(position),
                    delta: (-angle.0 / 12.0, -angle.1 / 12.0),
                    angle,
                    source: ScrollSource::Wheel,
                    mods: mods(modifiers),
                })
            }
            Event::Leave { .. } => WindowEvent::PointerLeave,
            Event::Key {
                code,
                pressed,
                modifiers,
                ..
            } => {
                let key = key(code)?;
                let mods = mods(modifiers);
                if pressed
                    && !mods.ctrl
                    && !mods.alt
                    && !mods.logo
                    && matches!(key, Key::Char(_) | Key::Space)
                {
                    self.pending = Some((key, mods));
                    return None;
                }
                let input = KeyInput {
                    key: Some(key),
                    text: String::new(),
                    mods,
                };
                if pressed {
                    WindowEvent::Key(input)
                } else {
                    WindowEvent::KeyRelease(input)
                }
            }
            Event::Theme { .. } => {
                crate::theme::refresh_system();
                WindowEvent::Wake
            }
            Event::Composition {
                preedit,
                cursor,
                commit,
                ..
            } => WindowEvent::TextInput(crate::types::text_input::Edit {
                preedit: crate::types::text_input::Preedit {
                    text: preedit,
                    cursor: Some((cursor, cursor)),
                },
                commit,
                replacement: None,
            }),
            Event::Character { character, .. } => {
                if (0xd800..=0xdbff).contains(&character) {
                    self.high = Some(character);
                    return None;
                }
                let character = if let Some(high) = self.high.take() {
                    char::decode_utf16([high, character])
                        .collect::<Result<String, _>>()
                        .ok()?
                } else {
                    char::from_u32(u32::from(character))?.to_string()
                };
                if character.chars().any(char::is_control) {
                    return None;
                }
                let pending = self.pending.take();
                WindowEvent::Key(KeyInput {
                    key: pending.map(|entry| entry.0),
                    mods: pending.map_or_else(Mods::default, |entry| entry.1),
                    text: character,
                })
            }
            _ => return None,
        })
    }
}
