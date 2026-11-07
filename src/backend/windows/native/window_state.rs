//! 【Windows表面】【原生回调】回调只绘制已提交位图并收集事件，不直接调用业务状态

use super::{Bitmap, Bounds, Event, EventQueue, events::modifiers};
use std::{
    cell::{Cell, RefCell},
    thread::ThreadId,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    Graphics::Gdi::{BeginPaint, BitBlt, EndPaint, GdiFlush, PAINTSTRUCT, SRCCOPY, ScreenToClient},
    UI::{
        Controls::WM_MOUSELEAVE,
        Input::KeyboardAndMouse::{
            ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
        },
        WindowsAndMessaging::*,
    },
};

pub(super) struct Context {
    pub id: u64,
    pub queue: EventQueue,
    pub bitmap: RefCell<Option<Bitmap>>,
    pub minimum: (u32, u32),
    pub layered: bool,
    pub activate: bool,
    pub cursor: Cell<HCURSOR>,
    pub owner: ThreadId,
    pub inside: Cell<bool>,
    pub text_target: Cell<u64>,
    pub ime_context: Cell<windows_sys::Win32::UI::Input::Ime::HIMC>,
}

/// 【Windows表面】【指针解码】按原生lparam返回有符号客户区物理坐标，不把负坐标转换为大正值
fn point(lparam: LPARAM) -> (f64, f64) {
    (
        (lparam as u16 as i16) as f64,
        ((lparam >> 16) as u16 as i16) as f64,
    )
}

/// 【Windows表面】【消息回调】从hwnd绑定的context收集message、wparam和lparam，返回系统消息处理结果；context始终属于创建线程
pub(super) unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = unsafe { &*(lparam as *const CREATESTRUCTW) };
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
    }
    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const Context;
    if pointer.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    // 1. 【Windows表面】【固定所有权】Box保持地址稳定，DestroyWindow返回前context仍存活，窗口消息只在创建线程分派
    let context = unsafe { &*pointer };
    debug_assert_eq!(context.owner, std::thread::current().id());
    let id = context.id;
    let event = match message {
        WM_CLOSE => Some(Event::Close { id }),
        WM_SETFOCUS | WM_KILLFOCUS => Some(Event::Focus {
            id,
            focused: message == WM_SETFOCUS,
        }),
        WM_SIZE => {
            let mut rect = unsafe { std::mem::zeroed() };
            unsafe {
                GetClientRect(hwnd, &mut rect);
            }
            Some(Event::Resize {
                id,
                size: (
                    (rect.right - rect.left).max(1) as u32,
                    (rect.bottom - rect.top).max(1) as u32,
                ),
            })
        }
        WM_DPICHANGED => {
            let rect = unsafe { &*(lparam as *const windows_sys::Win32::Foundation::RECT) };
            Some(Event::Dpi {
                id,
                dpi: wparam as u16 as u32,
                bounds: Bounds {
                    left: rect.left,
                    top: rect.top,
                    width: (rect.right - rect.left).max(1) as u32,
                    height: (rect.bottom - rect.top).max(1) as u32,
                },
            })
        }
        WM_MOUSEMOVE => {
            if !context.inside.replace(true) {
                let mut tracking = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe {
                    TrackMouseEvent(&mut tracking);
                }
            }
            Some(Event::Pointer {
                id,
                position: point(lparam),
            })
        }
        WM_MOUSELEAVE => {
            context.inside.set(false);
            Some(Event::Leave { id })
        }
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_MBUTTONDOWN
        | WM_MBUTTONUP => {
            let pressed = matches!(message, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN);
            unsafe {
                if pressed {
                    SetCapture(hwnd);
                } else {
                    ReleaseCapture();
                }
            }
            Some(Event::Button {
                id,
                position: point(lparam),
                pressed,
                time: unsafe { GetMessageTime() } as u32,
                button: if matches!(message, WM_LBUTTONDOWN | WM_LBUTTONUP) {
                    0x110
                } else if matches!(message, WM_RBUTTONDOWN | WM_RBUTTONUP) {
                    0x111
                } else {
                    0x112
                },
            })
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            let position = point(lparam);
            let mut local = POINT {
                x: position.0 as i32,
                y: position.1 as i32,
            };
            unsafe {
                ScreenToClient(hwnd, &mut local);
            }
            Some(Event::Wheel {
                id,
                position: (local.x as f64, local.y as f64),
                amount: (wparam >> 16) as u16 as i16,
                horizontal: message == WM_MOUSEHWHEEL,
                modifiers: modifiers(),
            })
        }
        WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP => Some(Event::Key {
            id,
            code: wparam as u32,
            pressed: matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN),
            repeated: lparam & (1 << 30) != 0,
            modifiers: modifiers(),
        }),
        WM_IME_STARTCOMPOSITION | WM_IME_ENDCOMPOSITION | WM_IME_COMPOSITION => {
            let target = context.text_target.get();
            if target == 0 {
                return 0;
            }
            let composition = if message == WM_IME_COMPOSITION {
                super::ime::read(hwnd, lparam).ok()
            } else {
                None
            };
            Some(Event::Composition {
                id,
                target,
                preedit: composition
                    .as_ref()
                    .map_or_else(String::new, |composition| composition.preedit.clone()),
                cursor: composition
                    .as_ref()
                    .map_or(0, |composition| composition.cursor),
                commit: composition.map_or_else(String::new, |composition| composition.commit),
            })
        }
        WM_CHAR => Some(Event::Character {
            id,
            character: wparam as u16,
        }),
        WM_ERASEBKGND => return 1,
        WM_MOUSEACTIVATE if !context.activate => return MA_NOACTIVATE as isize,
        WM_SETCURSOR => {
            unsafe {
                SetCursor(context.cursor.get());
            }
            return 1;
        }
        WM_GETMINMAXINFO => {
            let info = unsafe { &mut *(lparam as *mut MINMAXINFO) };
            info.ptMinTrackSize.x = context.minimum.0 as i32;
            info.ptMinTrackSize.y = context.minimum.1 as i32;
            return 0;
        }
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = unsafe { std::mem::zeroed() };
            let dc = unsafe { BeginPaint(hwnd, &mut paint) };
            if !context.layered
                && let Some(bitmap) = context.bitmap.borrow().as_ref()
            {
                let size = bitmap.size();
                unsafe {
                    BitBlt(
                        dc,
                        0,
                        0,
                        size.0 as i32,
                        size.1 as i32,
                        bitmap.dc(),
                        0,
                        0,
                        SRCCOPY,
                    );
                    GdiFlush();
                }
            }
            unsafe {
                EndPaint(hwnd, &paint);
            }
            return 0;
        }
        WM_NCDESTROY => {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            None
        }
        _ => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    };
    if let Some(event) = event {
        context.queue.push(event);
    }
    if matches!(
        message,
        WM_SYSKEYDOWN | WM_SYSKEYUP | WM_NCCREATE | WM_NCDESTROY
    ) {
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    } else {
        0
    }
}
