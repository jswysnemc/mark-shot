//! 【X11滚动截图】【局部控制】只映射控制面板，选区保持原应用输入与捕获像素

use super::*;
use x11rb::{connection::Connection, protocol::xproto::*};

impl Overlay {
    /// 【X11滚动截图】【角色切换】按monitor_idx隐藏截图并恢复原焦点，返回协议结果，保留表面索引身份
    pub(super) fn start_scroll(&mut self, monitor_idx: usize) -> crate::AppResult {
        self.surfaces
            .get(monitor_idx)
            .ok_or("X11 scroll output missing")?;
        for (_, surface) in &mut self.surfaces {
            self.display
                .connection
                .unmap_window(surface.window)?
                .check()?;
            surface.mapped = false;
            surface.pixels.clear();
        }
        if let Some(window) = self.previous_focus {
            self.display
                .connection
                .set_input_focus(InputFocus::POINTER_ROOT, window, x11rb::CURRENT_TIME)?
                .check()?;
        }
        self.scrolling = Some(monitor_idx);
        self.repaint_after =
            Some(std::time::Instant::now() + std::time::Duration::from_millis(150));
        self.display.connection.flush()?;
        Ok(())
    }

    /// 【X11滚动截图】【局部呈现】按monitor_idx及frame定位控制窗口，重叠捕获区时保持隐藏，返回协议结果
    pub(super) fn scroll_frame(
        &mut self,
        monitor_idx: usize,
        frame: &crate::types::scroll_panel::ScrollPanelFrame<'_>,
    ) -> crate::AppResult {
        if self.scrolling != Some(monitor_idx) {
            return Err("X11 scroll panel has no active output".into());
        }
        let (output, surface) = self
            .surfaces
            .get_mut(monitor_idx)
            .ok_or("X11 scroll output missing")?;
        if self.panel_hidden {
            return Ok(());
        }
        let origin = self.outputs[*output].info.location;
        let position = (origin.0 + frame.bounds.x(), origin.1 + frame.bounds.y());
        let size = (u16::try_from(frame.size.0)?, u16::try_from(frame.size.1)?);
        self.display
            .connection
            .configure_window(
                surface.window,
                &ConfigureWindowAux::new()
                    .x(position.0)
                    .y(position.1)
                    .width(frame.size.0)
                    .height(frame.size.1),
            )?
            .check()?;
        surface.position = (position.0.try_into()?, position.1.try_into()?);
        surface.size = size;
        surface.present(frame.pixels)?;
        Ok(())
    }

    /// 【X11滚动截图】【污染检查】按region核对面板实际布局，重叠则隐藏并拒绝当前帧，返回后续帧是否可用
    pub(super) fn allow_scroll(&mut self, region: tiny_skia::Rect) -> crate::AppResult<bool> {
        if let Some(deadline) = self.repaint_after {
            if std::time::Instant::now() >= deadline {
                self.repaint_after = None;
            }
            // 1. 【X11滚动截图】【曝光过渡】未合成桌面需由原应用响应Expose；期限后仍丢弃当前帧，下一次取帧才作拼接基准
            return Ok(false);
        }
        if self.panel_hidden {
            return Ok(true);
        }
        let Some(index) = self.scrolling else {
            return Ok(true);
        };
        if self
            .panel_rect
            .is_some_and(|panel| panel.intersect(&region).is_some())
        {
            let surface = &mut self.surfaces[index].1;
            self.display
                .connection
                .unmap_window(surface.window)?
                .check()?;
            surface.mapped = false;
            self.panel_hidden = true;
            self.repaint_after =
                Some(std::time::Instant::now() + std::time::Duration::from_millis(150));
            self.display.connection.flush()?;
            return Ok(false);
        }
        Ok(true)
    }

    /// 【X11滚动截图】【输入坐标】按index返回局部控制窗口相对原输出的偏移，用于共享面板命中测试
    pub(super) fn input_offset(&self, index: usize) -> (f64, f64) {
        if self.scrolling != Some(index) {
            return (0.0, 0.0);
        }
        let (output, surface) = &self.surfaces[index];
        let origin = self.outputs[*output].info.location;
        (
            f64::from(i32::from(surface.position.0) - origin.0),
            f64::from(i32::from(surface.position.1) - origin.1),
        )
    }
}
