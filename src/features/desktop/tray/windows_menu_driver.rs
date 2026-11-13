//! 【系统托盘测试】【原生菜单驱动】在测试窗口接收菜单通知，只向本窗口投递按键并核验系统资源回收

use super::super::{CALLBACK, State, procedure, wide};
use crate::features::desktop::{Reporter, tray::menu};
use std::{
    cell::RefCell,
    sync::atomic::{AtomicU64, Ordering},
};
use windows_sys::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

const TIMER: usize = 1;

pub(super) struct Expected {
    pub title: &'static str,
    pub items: Vec<(&'static str, bool)>,
    pub selected: Option<usize>,
}

struct Session {
    expected: Expected,
    root: HMENU,
    recording: HMENU,
    submenu_opened: bool,
    error: Option<String>,
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

pub(super) struct Window {
    handle: HWND,
    instance: HINSTANCE,
    class: Vec<u16>,
    _state: Box<State>,
}

/// 【系统托盘测试】【菜单文字】按 handle 和 position 读取系统菜单文字，返回解码结果或系统错误
fn text(handle: HMENU, position: u32) -> Result<String, String> {
    let mut buffer = [0u16; 256];
    let count = unsafe {
        GetMenuStringW(
            handle,
            position,
            buffer.as_mut_ptr(),
            buffer.len() as i32,
            MF_BYPOSITION,
        )
    };
    if count <= 0 {
        return Err("cannot read native menu label".into());
    }
    String::from_utf16(&buffer[..count as usize]).map_err(|error| error.to_string())
}

/// 【系统托盘测试】【窗口按键】按 window 和 keys 向本测试窗口排队按下与释放消息，返回投递错误
fn keys(window: HWND, keys: &[VIRTUAL_KEY]) -> Result<(), String> {
    for &key in keys {
        for (message, flags) in [(WM_KEYDOWN, 1), (WM_KEYUP, 0xc0000001u32 as isize)] {
            if unsafe { PostMessageW(window, message, key as usize, flags) } == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
    }
    Ok(())
}

impl Session {
    /// 【系统托盘测试】【菜单检查】按 handle 读取录制标题、操作及禁用标记，返回结构或属性不符合预期的错误
    fn inspect(&mut self, handle: HMENU) -> Result<(), String> {
        self.root = handle;
        self.recording = unsafe { GetSubMenu(handle, 4) };
        if self.recording.is_null() {
            return Err("native recording submenu is missing".into());
        }
        let title = text(handle, 4)?;
        if title != self.expected.title {
            return Err(format!("unexpected native recording title: {title}"));
        }
        if unsafe { GetMenuItemCount(self.recording) } != self.expected.items.len() as i32 {
            return Err("unexpected native recording action count".into());
        }
        for (position, &(expected_label, expected_enabled)) in
            self.expected.items.iter().enumerate()
        {
            let label = text(self.recording, position as u32)?;
            let flags = unsafe { GetMenuState(self.recording, position as u32, MF_BYPOSITION) };
            if flags == u32::MAX {
                return Err("cannot read native recording action state".into());
            }
            let enabled = flags & (MF_GRAYED | MF_DISABLED) == 0;
            if label != expected_label || enabled != expected_enabled {
                return Err(format!(
                    "unexpected native action: {label}, enabled={enabled}"
                ));
            }
        }
        Ok(())
    }

    /// 【系统托盘测试】【菜单导航】按 window 和 handle 核验弹出菜单并排队选择，返回通知或按键投递错误
    fn popup(&mut self, window: HWND, handle: HMENU) -> Result<(), String> {
        if self.root.is_null() {
            // 1. 【系统托盘测试】【根菜单】从末项反向选择录制子菜单，避开延时子菜单的自动展开
            self.inspect(handle)?;
            if self.expected.selected.is_none() {
                return keys(window, &[VK_ESCAPE]);
            }
            keys(window, &[VK_END, VK_UP, VK_UP, VK_UP, VK_RIGHT])
        } else if handle == self.recording {
            // 2. 【系统托盘测试】【录制子菜单】启用项等待系统交付动作，禁用项尝试回车后取消
            self.submenu_opened = true;
            let selected = self.expected.selected.ok_or("unexpected submenu opening")?;
            let mut input = vec![VK_HOME];
            input.extend(std::iter::repeat_n(VK_DOWN, selected));
            input.push(VK_RETURN);
            if !self.expected.items[selected].1 {
                input.extend([VK_ESCAPE, VK_ESCAPE]);
            }
            keys(window, &input)
        } else {
            Err("unexpected native submenu opening".into())
        }
    }
}

/// 【系统托盘测试】【原生回调】按窗口消息驱动测试菜单，其余消息交给生产托盘回调，返回窗口处理结果
unsafe extern "system" fn callback(
    window: HWND,
    message: u32,
    first: WPARAM,
    second: LPARAM,
) -> LRESULT {
    if message == WM_INITMENUPOPUP || (message == WM_TIMER && first == TIMER) {
        SESSION.with(|cell| {
            let mut session = cell.borrow_mut();
            if let Some(session) = session.as_mut() {
                let result = if message == WM_TIMER {
                    Err("native menu did not complete before timeout".into())
                } else {
                    session.popup(window, first as HMENU)
                };
                if let Err(error) = result {
                    session.error = Some(error);
                    unsafe { EndMenu() };
                }
            }
        });
    }
    unsafe { procedure(window, message, first, second) }
}

impl Window {
    /// 【系统托盘测试】【窗口创建】按 reporter 和 menu 创建绑定生产状态的隐藏窗口，返回系统错误
    pub(super) fn new(
        reporter: Reporter,
        menu: tokio::sync::watch::Receiver<menu::State>,
    ) -> Result<Self, String> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        let class = wide(&format!(
            "MarkShotTrayMenuTest.{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let definition = WNDCLASSW {
            lpfnWndProc: Some(callback),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..unsafe { std::mem::zeroed() }
        };
        if unsafe { RegisterClassW(&definition) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut state = Box::new(State {
            reporter,
            menu,
            icon: std::ptr::null_mut(),
            restart: u32::MAX,
        });
        let handle = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                wide("Mark Shot Tray Menu Test").as_ptr(),
                0,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                (&mut *state as *mut State).cast(),
            )
        };
        if handle.is_null() {
            let error = std::io::Error::last_os_error().to_string();
            unsafe { UnregisterClassW(class.as_ptr(), instance) };
            return Err(error);
        }
        Ok(Self {
            handle,
            instance,
            class,
            _state: state,
        })
    }

    /// 【系统托盘测试】【实际弹出】按 expected 触发生产右键回调，返回菜单属性、导航或资源回收错误
    pub(super) fn open(&self, expected: Expected) -> Result<(), String> {
        if expected
            .selected
            .is_some_and(|position| position >= expected.items.len())
        {
            return Err("selected native menu action is out of range".into());
        }
        if unsafe { SetTimer(self.handle, TIMER, 5000, None) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        SESSION.with(|cell| {
            *cell.borrow_mut() = Some(Session {
                expected,
                root: std::ptr::null_mut(),
                recording: std::ptr::null_mut(),
                submenu_opened: false,
                error: None,
            });
        });
        // 1. 【系统托盘测试】【右键通知】执行实际回调，生产路径负责读取最新快照和交付动作
        unsafe { SendMessageW(self.handle, CALLBACK, 0, WM_RBUTTONUP as isize) };
        unsafe { KillTimer(self.handle, TIMER) };
        // 2. 【系统托盘测试】【消息清理】清除取消时剩余的本窗口按键，防止下一次菜单收到旧输入
        unsafe {
            let mut message = std::mem::zeroed();
            while PeekMessageW(
                &mut message,
                self.handle,
                WM_KEYFIRST,
                WM_KEYLAST,
                PM_REMOVE,
            ) != 0
            {}
        }
        let session = SESSION
            .with(|cell| cell.borrow_mut().take())
            .ok_or("missing menu session")?;
        if let Some(error) = session.error {
            return Err(error);
        }
        if session.root.is_null()
            || (session.expected.selected.is_some() && !session.submenu_opened)
        {
            return Err("native recording menu was not opened".into());
        }
        if unsafe { IsMenu(session.root) != 0 || IsMenu(session.recording) != 0 } {
            return Err("native menu handles were not released".into());
        }
        Ok(())
    }
}

impl Drop for Window {
    /// 【系统托盘测试】【窗口回收】无参数，先销毁窗口和类别，再释放供生产回调读取的状态
    fn drop(&mut self) {
        unsafe {
            KillTimer(self.handle, TIMER);
            DestroyWindow(self.handle);
            UnregisterClassW(self.class.as_ptr(), self.instance);
        }
    }
}
