//! 【Windows快捷键】【截图键回退】只处理已配置PrintScreen组合，回调只投递动作且在注销时移除钩子

use super::{Action, Reporter};
use crate::keys::Mods;
use std::cell::RefCell;
use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

struct Binding {
    modifiers: Mods,
    reporter: Reporter,
    pressed: bool,
}
thread_local! { static BINDING: RefCell<Option<Binding>> = const { RefCell::new(None) }; }

pub(super) struct Hook(HHOOK);

impl Hook {
    /// 【Windows快捷键】【安装回退】按modifiers与reporter安装截图键监听，返回负责完整注销的句柄或系统错误
    pub fn new(modifiers: Mods, reporter: Reporter) -> Result<Self, String> {
        let hook = unsafe {
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard),
                GetModuleHandleW(std::ptr::null()),
                0,
            )
        };
        if hook.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        BINDING.with(|binding| {
            *binding.borrow_mut() = Some(Binding {
                modifiers,
                reporter,
                pressed: false,
            })
        });
        Ok(Self(hook))
    }
}

impl Drop for Hook {
    /// 【Windows快捷键】【回退释放】无参数，先移除系统回调再释放当前线程配置
    fn drop(&mut self) {
        unsafe {
            UnhookWindowsHookEx(self.0);
        }
        BINDING.with(|binding| *binding.borrow_mut() = None);
    }
}

/// 【Windows快捷键】【实时修饰键】无参数，读取用户当前组合状态，返回标准修饰键集合
fn modifiers() -> Mods {
    let down = |key| unsafe { GetAsyncKeyState(key) < 0 };
    Mods {
        ctrl: down(VK_CONTROL.into()),
        alt: down(VK_MENU.into()),
        shift: down(VK_SHIFT.into()),
        logo: down(VK_LWIN.into()) || down(VK_RWIN.into()),
    }
}

/// 【Windows快捷键】【按键回调】按系统code、message和data处理配置截图键，返回吞掉已处理组合或继续系统钩子链
unsafe extern "system" fn keyboard(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && data != 0 {
        let event = unsafe { &*(data as *const KBDLLHOOKSTRUCT) };
        if event.vkCode == u32::from(VK_SNAPSHOT) {
            let handled = BINDING.with(|binding| {
                let mut slot = binding.borrow_mut();
                let Some(binding) = slot.as_mut() else {
                    return false;
                };
                match message as u32 {
                    WM_KEYDOWN | WM_SYSKEYDOWN
                        if binding.pressed || modifiers() == binding.modifiers =>
                    {
                        if !binding.pressed {
                            binding.reporter.action(Action::Capture);
                        }
                        binding.pressed = true;
                        true
                    }
                    WM_KEYUP | WM_SYSKEYUP if binding.pressed => {
                        binding.pressed = false;
                        true
                    }
                    _ => false,
                }
            });
            if handled {
                return 1;
            }
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, message, data) }
}
