//! 【截图窗口】【X11 显隐】成组解除窗口映射，恢复时保留窗口身份与内容

use super::*;

impl Node {
    /// 【截图窗口】【窗口组显隐】按 hidden 隐藏主窗口及子窗口，返回协议错误；恢复由下一次内容绘制重新映射
    pub(super) fn capture_visibility(&mut self, hidden: bool) -> crate::AppResult {
        self.popups.clear();
        self.popup_grab = None;
        if hidden {
            self.surface
                .display
                .connection
                .unmap_window(self.surface.window)?
                .check()?;
            self.surface.mapped = false;
        }
        self.dirty = true;
        if let Some((_, child)) = &mut self.dialog {
            child.capture_visibility(hidden)?;
        }
        self.surface.display.connection.flush()?;
        Ok(())
    }
}
