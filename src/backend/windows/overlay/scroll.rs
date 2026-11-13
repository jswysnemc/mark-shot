//! 【Windows长截图】【控制窗口】管理局部面板、显示器坐标与捕获避让

use super::{Overlay, native};
use std::time::{Duration, Instant};
use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindow, IsWindowVisible, SetForegroundWindow};

#[derive(Default)]
pub(super) struct State {
    pub monitor: Option<usize>,
    pub rect: Option<tiny_skia::Rect>,
    pub hidden: bool,
    repaint_after: Option<Instant>,
}

impl Overlay {
    /// 【Windows长截图】【角色切换】按monitor_idx移除全屏冻结画面并创建局部面板，返回原生结果，保留输出身份
    pub(super) fn start_scroll(&mut self, monitor_idx: usize) -> crate::AppResult {
        let rect = self
            .scroll
            .rect
            .ok_or("Windows scroll panel is not configured")?;
        let bounds = rect
            .round_out()
            .ok_or("invalid Windows scroll panel bounds")?;
        let origin = self
            .surfaces
            .get(monitor_idx)
            .and_then(|entry| self.outputs.get(entry.0))
            .ok_or("Windows scroll output is unavailable")?
            .native
            .bounds;
        // 1. 【Windows长截图】【独立面板】首帧显示不激活窗口，底层页面继续接收滚轮和键盘
        let panel = native::Window::new(
            native::WindowOptions {
                title: "Mark Shot - Scroll".into(),
                bounds: native::Bounds {
                    left: origin
                        .left
                        .checked_add(bounds.x())
                        .ok_or("scroll panel position overflow")?,
                    top: origin
                        .top
                        .checked_add(bounds.y())
                        .ok_or("scroll panel position overflow")?,
                    width: bounds.width(),
                    height: bounds.height(),
                },
                minimum: (1, 1),
                topmost: true,
                activate: false,
                layered: true,
                owner: 0,
            },
            self.queue.clone(),
        )?;
        // 2. 【Windows长截图】【冻结回收】隐藏全部覆盖层并释放完整DIB，随后用面板替换目标窗口
        for (_, window) in &mut self.surfaces {
            window.hide();
            window.discard_bitmap();
        }
        self.surfaces[monitor_idx].1 = panel;
        self.staged.fill(false);
        self.scroll.monitor = Some(monitor_idx);
        self.scroll.hidden = false;
        self.scroll.repaint_after = Some(Instant::now() + Duration::from_millis(150));
        // 3. 【Windows长截图】【焦点恢复】仅恢复仍存在的原窗口，不根据当前覆盖层猜测底层应用
        let previous = self.previous_focus as _;
        if unsafe { IsWindow(previous) != 0 && IsWindowVisible(previous) != 0 } {
            unsafe {
                SetForegroundWindow(previous);
            }
        }
        Ok(())
    }

    /// 【Windows长截图】【面板呈现】按monitor_idx和frame定位并提交唯一局部DIB，返回系统结果，隐藏期间不提交像素
    pub(super) fn scroll_frame(
        &mut self,
        monitor_idx: usize,
        frame: &crate::types::scroll_panel::ScrollPanelFrame<'_>,
    ) -> crate::AppResult {
        if self.scroll.monitor != Some(monitor_idx) {
            return Err("Windows scroll panel has no active output".into());
        }
        if self.scroll.hidden {
            return Ok(());
        }
        let output = self.surfaces[monitor_idx].0;
        let origin = self.outputs[output].native.bounds;
        let window = &mut self.surfaces[monitor_idx].1;
        window.set_bounds(native::Bounds {
            left: origin
                .left
                .checked_add(frame.bounds.x())
                .ok_or("scroll panel position overflow")?,
            top: origin
                .top
                .checked_add(frame.bounds.y())
                .ok_or("scroll panel position overflow")?,
            width: frame.size.0,
            height: frame.size.1,
        })?;
        window.update_bitmap(frame.size, |bitmap| bitmap.write_rgba(frame.pixels))?;
        self.staged[monitor_idx] = true;
        Ok(())
    }

    /// 【Windows长截图】【捕获避让】按region检查真实窗口边界，重叠时隐藏并拒绝在途帧，返回是否可接收干净帧
    pub(super) fn allow_scroll(&mut self, region: tiny_skia::Rect) -> crate::AppResult<bool> {
        if let Some(deadline) = self.scroll.repaint_after {
            if Instant::now() >= deadline {
                self.scroll.repaint_after = None;
            }
            // 1. 【Windows长截图】【曝光过渡】期限结束后仍丢弃已经捕获的当前帧，下次请求才建立干净基准
            return Ok(false);
        }
        if self.scroll.hidden {
            return Ok(true);
        }
        let Some(index) = self.scroll.monitor else {
            return Ok(true);
        };
        let offset = self.scroll_input_offset(index)?;
        let bounds = self.surfaces[index].1.bounds()?;
        let panel = tiny_skia::Rect::from_xywh(
            offset.0 as f32,
            offset.1 as f32,
            bounds.width as f32,
            bounds.height as f32,
        )
        .ok_or("invalid Windows scroll panel geometry")?;
        if panel.intersect(&region).is_some() {
            self.surfaces[index].1.hide();
            self.staged[index] = false;
            self.scroll.hidden = true;
            self.scroll.repaint_after = Some(Instant::now() + Duration::from_millis(150));
            return Ok(false);
        }
        Ok(true)
    }

    /// 【Windows长截图】【坐标偏移】按index返回局部窗口相对原显示器的物理偏移，非长截图返回零偏移
    pub(super) fn scroll_input_offset(&self, index: usize) -> crate::AppResult<(f64, f64)> {
        if self.scroll.monitor != Some(index) {
            return Ok((0.0, 0.0));
        }
        let (output, window) = &self.surfaces[index];
        let bounds = window.bounds()?;
        let origin = self.outputs[*output].native.bounds;
        Ok((
            f64::from(bounds.left) - f64::from(origin.left),
            f64::from(bounds.top) - f64::from(origin.top),
        ))
    }
}

#[cfg(test)]
mod tests;
