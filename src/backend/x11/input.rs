//! 【X11输入】【键盘状态】使用服务器XKB映射解析布局、锁定修饰键及本地组合输入

use super::connection::Display;
use crate::{backend::window::KeyInput, keys::Mods, types::MouseButton};
use smithay_client_toolkit::seat::keyboard::Keysym;
use x11rb::protocol::{
    Event,
    xkb::{self as protocol, ConnectionExt as _},
    xproto::*,
};
use xkbcommon::xkb::{self, compose};

pub(super) struct Keyboard {
    context: xkb::Context,
    state: xkb::State,
    compose: Option<compose::State>,
}

impl Keyboard {
    /// 【X11输入】【键盘创建】按display取得服务器映射并订阅变化，返回键盘与组合状态
    pub fn new(display: &Display) -> crate::AppResult<Self> {
        let mut major = 0;
        let mut minor = 0;
        let mut base_event = 0;
        let mut base_error = 0;
        if !xkb::x11::setup_xkb_extension(
            display.connection.as_ref(),
            1,
            0,
            xkb::x11::SetupXkbExtensionFlags::NoFlags,
            &mut major,
            &mut minor,
            &mut base_event,
            &mut base_error,
        ) {
            return Err("X11 keyboard requires the XKB extension".into());
        }
        display
            .connection
            .xkb_select_events(
                protocol::ID::USE_CORE_KBD.into(),
                protocol::EventType::default(),
                protocol::EventType::NEW_KEYBOARD_NOTIFY | protocol::EventType::MAP_NOTIFY,
                protocol::MapPart::KEY_TYPES
                    | protocol::MapPart::KEY_SYMS
                    | protocol::MapPart::MODIFIER_MAP,
                protocol::MapPart::KEY_TYPES
                    | protocol::MapPart::KEY_SYMS
                    | protocol::MapPart::MODIFIER_MAP,
                &protocol::SelectEventsAux::default(),
            )?
            .check()?;
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let state = Self::load(&context, display)?;
        let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
            .iter()
            .find_map(|name| std::env::var_os(name).filter(|value| !value.is_empty()))
            .unwrap_or_else(|| "C.UTF-8".into());
        let compose = compose::Table::new_from_locale(&context, &locale, compose::COMPILE_NO_FLAGS)
            .ok()
            .map(|table| compose::State::new(&table, compose::STATE_NO_FLAGS));
        Ok(Self {
            context,
            state,
            compose,
        })
    }

    /// 【X11输入】【服务器映射】按context及display读取当前核心键盘，返回有效状态或明确错误
    fn load(context: &xkb::Context, display: &Display) -> crate::AppResult<xkb::State> {
        let device = xkb::x11::get_core_keyboard_device_id(display.connection.as_ref());
        if device < 0 {
            return Err("X11 core keyboard unavailable".into());
        }
        let keymap = xkb::x11::keymap_new_from_device(
            context,
            display.connection.as_ref(),
            device,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        );
        if keymap.get_raw_ptr().is_null() {
            return Err("X11 keyboard map unavailable".into());
        }
        let state = xkb::x11::state_new_from_device(&keymap, display.connection.as_ref(), device);
        if state.get_raw_ptr().is_null() {
            return Err("X11 keyboard state unavailable".into());
        }
        Ok(state)
    }

    /// 【X11输入】【布局变化】按display及event刷新变化后的映射，无关事件不发出协议请求
    pub fn update(&mut self, display: &Display, event: &Event) -> crate::AppResult {
        if matches!(
            event,
            Event::MappingNotify(_) | Event::XkbMapNotify(_) | Event::XkbNewKeyboardNotify(_)
        ) {
            self.state = Self::load(&self.context, display)?;
            self.reset();
        }
        Ok(())
    }

    /// 【X11输入】【组合取消】无参数，在焦点或输入目标变化后清除尚未确认的组合序列
    pub fn reset(&mut self) {
        if let Some(compose) = &mut self.compose {
            compose.reset();
        }
    }

    /// 【X11输入】【真实修饰键】无参数，按XKB映射返回当前有效修饰键，不混入布局切换键
    pub fn modifiers(&self) -> Mods {
        let active = |name| {
            self.state
                .mod_name_is_active(name, xkb::STATE_MODS_EFFECTIVE)
        };
        Mods {
            ctrl: active("Control"),
            shift: active("Shift"),
            alt: active("Mod1"),
            logo: active("Mod4"),
        }
    }

    /// 【X11输入】【按键转换】按event与pressed解析布局和组合文字，返回共享按键，释放不产生正文
    pub fn key(&mut self, event: &KeyPressEvent, pressed: bool) -> KeyInput {
        let mask = u32::from(u16::from(event.state));
        self.state
            .update_mask(mask & 0xff, 0, 0, 0, 0, (mask >> 13) & 3);
        let keycode = xkb::Keycode::new(u32::from(event.detail));
        let symbol = self.state.key_get_one_sym(keycode);
        let mods = self.modifiers();
        let chord = crate::backend::wayland::keysym::chord(
            Keysym::new(symbol.raw()),
            u32::from(event.detail).saturating_sub(8),
            mods,
        );
        let mut key = chord.map(|chord| chord.key);
        let mut text = String::new();
        if pressed && !mods.ctrl && !mods.alt && !mods.logo {
            text = self.state.key_get_utf8(keycode);
            if let Some(compose) = &mut self.compose {
                compose.feed(symbol);
                match compose.status() {
                    compose::Status::Composed => {
                        text = compose.utf8().unwrap_or_default();
                        compose.reset();
                    }
                    compose::Status::Composing => {
                        text.clear();
                        key = None;
                    }
                    compose::Status::Cancelled => {
                        text.clear();
                        key = None;
                        compose.reset();
                    }
                    compose::Status::Nothing => {}
                }
            }
            text.retain(|character| !character.is_control());
        }
        self.state.update_key(
            keycode,
            if pressed {
                xkb::KeyDirection::Down
            } else {
                xkb::KeyDirection::Up
            },
        );
        KeyInput { key, text, mods }
    }
}

/// 【X11输入】【修饰状态】按state返回四种共享修饰键，不把鼠标按钮混入按键状态
pub fn modifiers(state: KeyButMask) -> Mods {
    Mods {
        ctrl: state.contains(KeyButMask::CONTROL),
        shift: state.contains(KeyButMask::SHIFT),
        alt: state.contains(KeyButMask::MOD1),
        logo: state.contains(KeyButMask::MOD4),
    }
}

/// 【X11输入】【鼠标按钮】按detail返回共享按钮，滚轮和额外按钮交给各自处理路径
pub fn button(detail: u8) -> Option<MouseButton> {
    match detail {
        1 => Some(MouseButton::Left),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【X11输入】【测试键盘】按layout及variant构造独立映射，返回不访问桌面的键盘状态
    fn keyboard(layout: &str, variant: &str) -> Keyboard {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_names(
            &context,
            "evdev",
            "pc105",
            layout,
            variant,
            Some(String::new()),
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .unwrap();
        let table = compose::Table::new_from_locale(
            &context,
            std::ffi::OsStr::new("en_US.UTF-8"),
            compose::COMPILE_NO_FLAGS,
        )
        .unwrap();
        Keyboard {
            state: xkb::State::new(&keymap),
            context,
            compose: Some(compose::State::new(&table, compose::STATE_NO_FLAGS)),
        }
    }

    /// 【X11输入】【键码夹具】按detail和state返回独立按下事件，不发送到系统显示
    fn event(detail: u8, state: KeyButMask) -> KeyPressEvent {
        KeyPressEvent {
            response_type: KEY_PRESS_EVENT,
            detail,
            state,
            ..Default::default()
        }
    }

    /// 【X11输入】【组合回归】验证死键组合和焦点取消后无残留正文，无参数
    #[test]
    fn dead_keys_compose_and_focus_reset_cancels_pending_sequence() {
        let mut keyboard = keyboard("us", "intl");
        assert!(
            keyboard
                .key(&event(48, KeyButMask::default()), true)
                .text
                .is_empty()
        );
        assert_eq!(
            keyboard.key(&event(26, KeyButMask::default()), true).text,
            "é"
        );
        keyboard.key(&event(48, KeyButMask::default()), true);
        keyboard.reset();
        assert_eq!(
            keyboard.key(&event(26, KeyButMask::default()), true).text,
            "e"
        );
    }

    /// 【X11输入】【布局回归】验证俄语正文、物理快捷键、AltGr和大写锁定遵循XKB，无参数
    #[test]
    fn non_latin_altgr_and_caps_lock_keep_text_and_shortcuts_separate() {
        let mut russian = keyboard("ru", "");
        assert_eq!(
            russian.key(&event(38, KeyButMask::default()), true).text,
            "ф"
        );
        let shortcut = russian.key(&event(54, KeyButMask::CONTROL), true);
        assert!(shortcut.text.is_empty());
        assert_eq!(shortcut.key, Some(crate::keys::Key::Char('c')));
        let mut german = keyboard("de", "");
        assert_eq!(german.key(&event(24, KeyButMask::MOD5), true).text, "@");
        assert_eq!(german.key(&event(38, KeyButMask::LOCK), true).text, "A");
        assert_eq!(
            german
                .key(&event(38, KeyButMask::LOCK | KeyButMask::SHIFT), true)
                .text,
            "a"
        );
    }
}
