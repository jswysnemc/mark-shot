//! 【Windows输入法】【隐藏光标】系统光标只供输入法读取位置，视觉闪烁由应用绘制

use std::{cell::Cell, io};
use windows_sys::Win32::{
    Foundation::HWND,
    UI::{
        Input::KeyboardAndMouse::GetFocus,
        WindowsAndMessaging::{CreateCaret, DestroyCaret, SetCaretPos},
    },
};

#[derive(Default)]
pub(super) struct Caret(Cell<Option<(i32, i32)>>);

impl Caret {
    /// 【Windows输入法】【光标同步】按 hwnd 和 rect 更新当前焦点的隐藏系统光标，返回系统结果；相同尺寸不重新创建
    pub(super) fn sync(&self, hwnd: HWND, rect: Option<(i32, i32, i32, i32)>) -> io::Result<()> {
        let rect = rect.filter(|_| unsafe { GetFocus() } == hwnd);
        let Some((x, y, width, height)) = rect else {
            self.clear();
            return Ok(());
        };
        let size = (width.max(1), height.max(1));
        if self.0.get() != Some(size) {
            // 1. 【Windows输入法】【隐藏创建】CreateCaret 默认隐藏，应用不调用 ShowCaret，避免两套光标同时闪烁
            if unsafe { CreateCaret(hwnd, std::ptr::null_mut(), size.0, size.1) } == 0 {
                return Err(super::last_error());
            }
            self.0.set(Some(size));
        }
        if unsafe { SetCaretPos(x, y) } == 0 {
            return Err(super::last_error());
        }
        Ok(())
    }

    /// 【Windows输入法】【光标释放】无参数，失焦、停用或关闭时释放本窗口拥有的系统光标，无返回值
    pub(super) fn clear(&self) {
        if self.0.take().is_some() {
            unsafe {
                DestroyCaret();
            }
        }
    }
}
