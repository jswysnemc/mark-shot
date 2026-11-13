//! 【Windows窗口】【线程所有权】固定地址状态与HWND共同存活，窗口不能跨线程移动，界面控件使用共享渲染像素

use super::{Bitmap, Bounds, EventQueue, window_state::Context};
use std::{
    cell::{Cell, RefCell},
    io,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, SIZE},
    Graphics::Gdi::{AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, InvalidateRect, UpdateWindow},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

pub struct Options {
    pub title: String,
    pub bounds: Bounds,
    /// 以96DPI逻辑像素表示的最小客户区尺寸，原生回调按窗口当前DPI换算
    pub minimum: (u32, u32),
    pub topmost: bool,
    pub activate: bool,
    pub layered: bool,
    pub owner: isize,
}

pub struct Window {
    hwnd: HWND,
    context: Box<Context>,
    shown: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Cursor {
    Default,
    Crosshair,
    Text,
    Pointer,
    Move,
    HorizontalResize,
    VerticalResize,
    NwSeResize,
    NeSwResize,
    Wait,
}

const CLASS: &[u16] = &[
    77, 97, 114, 107, 83, 104, 111, 116, 46, 78, 97, 116, 105, 118, 101, 83, 117, 114, 102, 97, 99,
    101, 46, 86, 49, 0,
];

/// 【Windows窗口】【类型登记】无参数，在进程中登记唯一原生表面类型，返回系统结果，不创建窗口
fn register() -> io::Result<()> {
    static RESULT: OnceLock<Result<(), u32>> = OnceLock::new();
    match RESULT.get_or_init(|| {
        let mut class: WNDCLASSW = unsafe { std::mem::zeroed() };
        class.lpfnWndProc = Some(super::window_state::procedure);
        class.hInstance = unsafe { GetModuleHandleW(std::ptr::null()) };
        class.lpszClassName = CLASS.as_ptr();
        if unsafe { RegisterClassW(&class) } == 0 {
            Err(unsafe { windows_sys::Win32::Foundation::GetLastError() })
        } else {
            Ok(())
        }
    }) {
        Ok(()) => Ok(()),
        Err(error) => Err(io::Error::from_raw_os_error(*error as i32)),
    }
}

impl Window {
    /// 【Windows窗口】【隐藏创建】按options和queue创建自绘表面，返回绑定当前线程的窗口；首帧准备前保持隐藏
    pub fn new(options: Options, queue: EventQueue) -> io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        if options.bounds.width == 0
            || options.bounds.height == 0
            || options.bounds.width > i32::MAX as u32
            || options.bounds.height > i32::MAX as u32
        {
            return Err(io::Error::other("invalid Windows surface bounds"));
        }
        super::enable_dpi()?;
        register()?;
        let cursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) };
        let context = Box::new(Context {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            queue,
            bitmap: RefCell::new(None),
            minimum: options.minimum,
            layered: options.layered,
            activate: options.activate,
            image_window: Cell::new(false),
            cursor: Cell::new(cursor),
            owner: std::thread::current().id(),
            inside: Cell::new(false),
            text_target: Cell::new(0),
            ime_context: Cell::new(std::ptr::null_mut()),
        });
        let title: Vec<_> = options.title.encode_utf16().chain([0]).collect();
        let style = if options.topmost { WS_EX_TOPMOST } else { 0 }
            | if !options.activate {
                WS_EX_NOACTIVATE
            } else {
                0
            }
            | if options.layered { WS_EX_LAYERED } else { 0 };
        let bounds = options.bounds;
        let hwnd = unsafe {
            CreateWindowExW(
                style,
                CLASS.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                bounds.left,
                bounds.top,
                bounds.width as i32,
                bounds.height as i32,
                options.owner as HWND,
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                (&*context as *const Context).cast(),
            )
        };
        if hwnd.is_null() {
            Err(super::last_error())
        } else {
            context.ime_context.set(unsafe {
                windows_sys::Win32::UI::Input::Ime::ImmAssociateContext(hwnd, std::ptr::null_mut())
            });
            Ok(Self {
                hwnd,
                context,
                shown: false,
            })
        }
    }

    /// 【Windows窗口】【稳定身份】无参数，返回不可因系统复用HWND而重用的本次编号
    pub fn id(&self) -> u64 {
        self.context.id
    }

    /// 【Windows窗口】【句柄借用】无参数，返回原生句柄值，调用方不得关闭窗口或替换其用户数据
    pub fn handle(&self) -> isize {
        self.hwnd as isize
    }

    /// 【Windows图片窗口】【可调整边框】无参数，启用系统移动缩放与最小最大化，自绘客户区保留全部尺寸，返回系统错误
    pub fn enable_image_frame(&mut self) -> io::Result<()> {
        self.context.image_window.set(true);
        unsafe {
            let style = GetWindowLongPtrW(self.hwnd, GWL_STYLE) as u32;
            SetWindowLongPtrW(
                self.hwnd,
                GWL_STYLE,
                (style | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU) as isize,
            );
            if SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
            ) == 0
            {
                return Err(super::last_error());
            }
        }
        Ok(())
    }

    /// 【Windows窗口】【原生边界】无参数，返回当前物理屏幕位置和尺寸，不推测窗口管理器布局
    pub fn bounds(&self) -> io::Result<Bounds> {
        let mut rect = unsafe { std::mem::zeroed() };
        if unsafe { GetWindowRect(self.hwnd, &mut rect) } == 0 {
            return Err(super::last_error());
        }
        Ok(Bounds {
            left: rect.left,
            top: rect.top,
            width: (rect.right - rect.left).max(1) as u32,
            height: (rect.bottom - rect.top).max(1) as u32,
        })
    }

    /// 【Windows窗口】【边界调整】把bounds用于当前线程窗口，返回系统结果，不改变置顶和激活状态
    pub fn set_bounds(&mut self, bounds: Bounds) -> io::Result<()> {
        if bounds.width == 0
            || bounds.height == 0
            || bounds.width > i32::MAX as u32
            || bounds.height > i32::MAX as u32
        {
            return Err(io::Error::other("invalid Windows surface bounds"));
        }
        if unsafe {
            SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(),
                bounds.left,
                bounds.top,
                bounds.width as i32,
                bounds.height as i32,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } == 0
        {
            Err(super::last_error())
        } else {
            Ok(())
        }
    }

    /// 【Windows窗口】【像素提交】按size提交完整rgba，首帧随后显示；一个DIB反复复用，返回原生绘制结果
    pub fn set_rgba(&mut self, size: (u32, u32), rgba: &[u8]) -> io::Result<()> {
        self.update_bitmap(size, |bitmap| bitmap.write_rgba(rgba))?;
        self.present()
    }

    /// 【Windows窗口】【缓冲修改】按size借用唯一位图给update，返回更新结果；完成修改后才允许提交，不产生完整副本
    pub fn update_bitmap(
        &mut self,
        size: (u32, u32),
        update: impl FnOnce(&mut Bitmap) -> io::Result<()>,
    ) -> io::Result<()> {
        {
            let mut bitmap = self.context.bitmap.borrow_mut();
            if bitmap.as_ref().is_none_or(|bitmap| bitmap.size() != size) {
                *bitmap = Some(Bitmap::new(size)?);
            }
            update(bitmap.as_mut().unwrap())?;
        }
        Ok(())
    }

    /// 【Windows窗口】【缓冲呈现】无参数，提交已经准备的原生位图，首帧随后显示，返回系统绘制结果
    pub fn present(&mut self) -> io::Result<()> {
        let size = self
            .context
            .bitmap
            .borrow()
            .as_ref()
            .ok_or_else(|| io::Error::other("Windows surface has no prepared frame"))?
            .size();
        if self.context.layered {
            let bounds = self.bounds()?;
            let position = POINT {
                x: bounds.left,
                y: bounds.top,
            };
            let dimensions = SIZE {
                cx: size.0 as i32,
                cy: size.1 as i32,
            };
            let origin = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let bitmap = self.context.bitmap.borrow();
            if unsafe {
                UpdateLayeredWindow(
                    self.hwnd,
                    std::ptr::null_mut(),
                    &position,
                    &dimensions,
                    bitmap.as_ref().unwrap().dc(),
                    &origin,
                    0,
                    &blend,
                    ULW_ALPHA,
                )
            } == 0
            {
                return Err(super::last_error());
            }
        } else {
            unsafe {
                InvalidateRect(self.hwnd, std::ptr::null(), 0);
                UpdateWindow(self.hwnd);
            }
        }
        if !self.shown {
            unsafe {
                ShowWindow(
                    self.hwnd,
                    if self.context.activate {
                        SW_SHOW
                    } else {
                        SW_SHOWNOACTIVATE
                    },
                );
            }
            self.shown = true;
        }
        Ok(())
    }

    /// 【Windows窗口】【正文同步】按target与物理rect开启或停用输入法，返回系统结果，目标改变时先取消旧候选
    pub fn set_text_input(
        &mut self,
        target: u64,
        rect: Option<(i32, i32, i32, i32)>,
    ) -> io::Result<()> {
        let previous = self.context.text_target.replace(target);
        super::ime::sync(
            self.hwnd,
            self.context.ime_context.get(),
            previous,
            target,
            rect,
        )
    }

    /// 【Windows窗口】【暂时隐藏】无参数，隐藏当前表面，保留原生位图和窗口身份，无返回值
    pub fn hide(&mut self) {
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
        self.shown = false;
    }

    /// 【Windows窗口】【缓冲回收】无参数，释放隐藏表面的唯一DIB，下一次提交重新按所需尺寸分配
    pub fn discard_bitmap(&mut self) {
        self.context.bitmap.borrow_mut().take();
    }

    /// 【Windows窗口】【按需聚焦】无参数，在用户点击控件后请求前台及键盘焦点，首次显示仍保持不激活
    pub fn focus(&mut self) {
        unsafe {
            SetForegroundWindow(self.hwnd);
            windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(self.hwnd);
        }
    }

    /// 【Windows窗口】【模态输入】按enabled启用或隔离父窗口输入，无返回值，关闭子窗口后由运行时恢复
    pub fn set_enabled(&mut self, enabled: bool) {
        unsafe {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                self.hwnd,
                i32::from(enabled),
            );
        }
    }

    /// 【Windows窗口】【置顶设置】按enabled更新原生置顶，不移动或激活窗口，返回系统结果
    pub fn set_topmost(&mut self, enabled: bool) -> io::Result<()> {
        if unsafe {
            SetWindowPos(
                self.hwnd,
                if enabled {
                    HWND_TOPMOST
                } else {
                    HWND_NOTOPMOST
                },
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        } == 0
        {
            Err(super::last_error())
        } else {
            Ok(())
        }
    }

    /// 【Windows窗口】【光标选择】按cursor设置原生光标，只有指针处于本窗口时更新当前系统形状，返回加载结果
    pub fn set_cursor(&mut self, cursor: Cursor) -> io::Result<()> {
        let resource = match cursor {
            Cursor::Default => IDC_ARROW,
            Cursor::Crosshair => IDC_CROSS,
            Cursor::Text => IDC_IBEAM,
            Cursor::Pointer => IDC_HAND,
            Cursor::Move => IDC_SIZEALL,
            Cursor::HorizontalResize => IDC_SIZEWE,
            Cursor::VerticalResize => IDC_SIZENS,
            Cursor::NwSeResize => IDC_SIZENWSE,
            Cursor::NeSwResize => IDC_SIZENESW,
            Cursor::Wait => IDC_WAIT,
        };
        let handle = unsafe { LoadCursorW(std::ptr::null_mut(), resource) };
        if handle.is_null() {
            return Err(super::last_error());
        }
        self.context.cursor.set(handle);
        if self.context.inside.get() {
            unsafe {
                SetCursor(handle);
            }
        }
        Ok(())
    }
}

impl Drop for Window {
    /// 【Windows窗口】【线程内关闭】无参数，先销毁HWND，再由字段释放固定地址状态和位图，无返回值
    fn drop(&mut self) {
        debug_assert_eq!(self.context.owner, std::thread::current().id());
        unsafe {
            windows_sys::Win32::UI::Input::Ime::ImmAssociateContext(
                self.hwnd,
                self.context.ime_context.get(),
            );
            DestroyWindow(self.hwnd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Event, capture, dispatch};
    use super::*;
    use std::time::{Duration, Instant};

    /// 【Windows窗口测试】【真实像素与消息】无参数，只在私有运行时显示自绘窗口，验证实际屏幕像素、事件身份与关闭释放，无返回值
    #[test]
    #[ignore = "requires the private Windows test runtime"]
    fn native_window_presents_exact_pixels_and_collects_owned_character_events() {
        assert_eq!(
            std::env::var("MARK_SHOT_PRIVATE_WINDOWS_TEST").as_deref(),
            Ok("1")
        );
        let bounds = Bounds {
            left: 200,
            top: 180,
            width: 80,
            height: 64,
        };
        let queue = EventQueue::default();
        let mut window = Window::new(
            Options {
                title: "Mark Shot native surface test".into(),
                bounds,
                minimum: (1, 1),
                topmost: true,
                activate: true,
                layered: false,
                owner: 0,
            },
            queue.clone(),
        )
        .unwrap();
        let mut pixels = Vec::new();
        for y in 0..bounds.height {
            for x in 0..bounds.width {
                pixels.extend_from_slice(&[(x * 3) as u8, (y * 3) as u8, (x + y) as u8, 255]);
            }
        }
        window
            .set_rgba((bounds.width, bounds.height), &pixels)
            .unwrap();
        let expected: Vec<_> = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[2], pixel[1], pixel[0], pixel[3]])
            .collect();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            dispatch(&[], Some(Duration::from_millis(5))).unwrap();
            if capture(window.bounds().unwrap(), false).unwrap() == expected {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "native window did not present its exact pixel buffer"
            );
        }
        while queue.pop().is_some() {}
        assert_ne!(
            unsafe { PostMessageW(window.hwnd, WM_CHAR, 'α' as usize, 0) },
            0
        );
        dispatch(&[], Some(Duration::from_secs(1))).unwrap();
        assert!(
            matches!(queue.pop(), Some(Event::Character { id, character: 945 }) if id == window.id())
        );
        window.hide();
        drop(window);
    }
}
