//! 【系统托盘】【Windows 菜单】递归构造子菜单并保持动作编号一致，系统关闭菜单后释放全部句柄

use super::{
    super::Action,
    menu::{self, Item},
};
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};

struct Menu(HMENU);

impl Drop for Menu {
    /// 【系统托盘】【菜单释放】无参数，销毁根菜单及系统拥有的全部子菜单
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.0);
        }
    }
}

/// 【系统托盘】【菜单编码】按 text 返回以零结尾的 UTF-16 文本，参数仅供同步系统调用使用
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// 【系统托盘】【递归构造】按 entries 和 actions 创建菜单，统一分配命令编号；系统错误时释放已创建资源
fn build(entries: Vec<Item>, actions: &mut Vec<Option<Action>>) -> std::io::Result<Menu> {
    let handle = unsafe { CreatePopupMenu() };
    if handle.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    let menu = Menu(handle);
    for entry in entries {
        let result = match entry {
            Item::Separator => unsafe { AppendMenuW(handle, MF_SEPARATOR, 0, std::ptr::null()) },
            Item::Action {
                label,
                action,
                enabled,
            } => {
                actions.push(enabled.then_some(action));
                unsafe {
                    AppendMenuW(
                        handle,
                        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
                        actions.len(),
                        wide(&label).as_ptr(),
                    )
                }
            }
            Item::Submenu { label, items } => {
                let child = build(items, actions)?;
                let result = unsafe {
                    AppendMenuW(
                        handle,
                        MF_POPUP | MF_STRING,
                        child.0 as usize,
                        wide(&label).as_ptr(),
                    )
                };
                if result != 0 {
                    // 1. 【系统托盘】【句柄交接】子菜单已由根菜单拥有，避免重复销毁
                    std::mem::forget(child);
                }
                result
            }
        };
        if result == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(menu)
}

/// 【系统托盘】【菜单选择】按 window 和 state 在光标处展示菜单，返回已启用动作；取消不派发任何操作
pub(super) fn show(window: HWND, state: menu::State) -> std::io::Result<Option<Action>> {
    let mut actions = Vec::new();
    let menu = build(menu::entries(state), &mut actions)?;
    let selected = unsafe {
        let mut point = std::mem::zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(window);
        let selected = TrackPopupMenuEx(
            menu.0,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            window,
            std::ptr::null(),
        );
        PostMessageW(window, WM_NULL, 0, 0);
        selected
    };
    Ok(actions
        .get((selected as usize).wrapping_sub(1))
        .copied()
        .flatten())
}
