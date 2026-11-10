//! 【系统托盘】【Windows通知区域】隐藏窗口接收托盘菜单与Explorer重启通知，退出清理图标

use super::super::{Action, Reporter, native_thread};
use windows_sys::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Shell::*, WindowsAndMessaging::*},
};

const CALLBACK: u32 = WM_APP + 17;
struct State {
    reporter: Reporter,
    icon: HICON,
    restart: u32,
    menu: tokio::sync::watch::Receiver<super::menu::State>,
}

/// 【系统托盘】【宽字符】按text返回带终止符的UTF16文本，供系统同步调用
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// 【系统托盘】【图标登记】按window、state和operation更新通知区域，返回系统执行结果
fn notify(window: HWND, state: &State, operation: u32) -> bool {
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = window;
    data.uID = 1;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = CALLBACK;
    data.hIcon = state.icon;
    for (target, value) in data.szTip.iter_mut().zip(wide("Mark Shot")) {
        *target = value;
    }
    unsafe { Shell_NotifyIconW(operation, &data) != 0 }
}

/// 【系统托盘】【菜单交付】按window和state显示系统托盘菜单，选择后通过服务队列交付动作
fn menu(window: HWND, state: &State) {
    let snapshot = *state.menu.borrow();
    match super::windows_menu::show(window, snapshot) {
        Ok(Some(action)) => state.reporter.action(action),
        Ok(None) => {}
        Err(error) => log::error!("【系统托盘】【菜单展示】{error}"),
    }
}

/// 【系统托盘】【窗口回调】按系统参数分派通知和菜单，返回默认处理或已处理结果
unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    first: WPARAM,
    second: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_NCCREATE {
            let creation = &*(second as *const CREATESTRUCTW);
            SetWindowLongPtrW(window, GWLP_USERDATA, creation.lpCreateParams as isize);
        }
        let pointer = GetWindowLongPtrW(window, GWLP_USERDATA) as *const State;
        if let Some(state) = pointer.as_ref() {
            if message == CALLBACK {
                match second as u32 {
                    WM_LBUTTONUP => state.reporter.action(Action::Capture),
                    WM_RBUTTONUP => menu(window, state),
                    _ => {}
                }
                return 0;
            }
            if message == state.restart {
                state.reporter.status(if notify(window, state, NIM_ADD) {
                    super::super::status::Registration::Registered(String::new())
                } else {
                    super::super::status::Registration::Failed(
                        std::io::Error::last_os_error().to_string(),
                    )
                });
                return 0;
            }
        }
        DefWindowProcW(window, message, first, second)
    }
}

/// 【系统托盘】【原生服务】按reporter和cancelled启动独立消息窗口，返回注册错误并释放全部系统资源
pub(super) async fn run(
    reporter: &Reporter,
    cancelled: tokio::sync::watch::Receiver<bool>,
    menu: tokio::sync::watch::Receiver<super::menu::State>,
) -> Result<(), String> {
    let reporter = reporter.clone();
    native_thread::run(cancelled, move |ready| unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class = wide("MarkShotDesktopTray");
        let definition = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassW(&definition) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut pixels = super::icon()?.take();
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        let icon = CreateIcon(
            instance,
            32,
            32,
            1,
            32,
            [0u8; 128].as_ptr(),
            pixels.as_ptr(),
        );
        if icon.is_null() {
            UnregisterClassW(class.as_ptr(), instance);
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut state = State {
            reporter,
            menu,
            icon,
            restart: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
        };
        let window = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("Mark Shot").as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            (&mut state as *mut State).cast(),
        );
        let result = if window.is_null() || !notify(window, &state, NIM_ADD) {
            Err(std::io::Error::last_os_error().to_string())
        } else if native_thread::ready(ready) {
            state
                .reporter
                .status(super::super::status::Registration::Registered(String::new()));
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
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        } else {
            Ok(())
        };
        if !window.is_null() {
            notify(window, &state, NIM_DELETE);
            DestroyWindow(window);
        }
        UnregisterClassW(class.as_ptr(), instance);
        DestroyIcon(icon);
        result
    })
    .await
}
