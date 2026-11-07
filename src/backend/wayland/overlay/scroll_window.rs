//! 【滚动截图】【普通控制窗口】只提交面板像素，原输出和捕获区域不跟随合成器窗口位置变化

use super::WaylandOverlay;
use crate::backend::wayland::utils::scroll_stamp::{Bounds, Stamp};
use crate::backend::wayland::utils::surface::{Role, ScrollFrame};
use tiny_skia::Rect;

pub(super) struct ScrollWindow {
    pub(super) rect: Rect,
    paused: bool,
    pub(super) hidden: bool,
    last_bounds: Option<Bounds>,
}

impl WaylandOverlay {
    /// 【滚动截图】【窗口局部帧】按monitor_idx和frame缓存面板像素，返回是否已处理，不复制整屏画布
    pub(super) fn stage_local_scroll_window(
        &mut self,
        monitor_idx: usize,
        frame: &crate::types::scroll_panel::ScrollPanelFrame<'_>,
    ) -> crate::AppResult<bool> {
        if self.scrolling != Some(monitor_idx) {
            return Ok(false);
        }
        let Some(panel) = &self.scroll_window else {
            return Ok(false);
        };
        if panel.hidden && !panel.paused {
            return Ok(true);
        }
        if frame.pixels.len() != frame.size.0 as usize * frame.size.1 as usize * 4 {
            return Err("invalid local scroll panel".into());
        }
        let runtime = &mut self.runtime;
        let surface = runtime
            .state
            .surfaces
            .get_mut(&monitor_idx)
            .ok_or("the scroll control window is gone")?;
        let mut pixels = surface
            .scroll_frame
            .take()
            .map_or_else(Vec::new, |frame| frame.rgba);
        pixels.clear();
        pixels.extend_from_slice(frame.pixels);
        surface.scroll_frame = Some(ScrollFrame {
            stamp: Stamp::new(&pixels, frame.size, surface.scale),
            rgba: pixels,
            size: frame.size,
        });
        surface.canvas_rect = Rect::from_xywh(
            frame.bounds.x() as f32,
            frame.bounds.y() as f32,
            frame.bounds.width() as f32,
            frame.bounds.height() as f32,
        );
        if let Role::ScrollPopup(popup) = &mut surface.role {
            popup.resize((frame.bounds.width(), frame.bounds.height()));
        }
        runtime
            .state
            .paint_scroll_frame(monitor_idx, &runtime.event_queue.handle())?;
        Ok(true)
    }
    /// 【滚动截图】【窗口布局】用 rect 和 paused 同步普通控制窗口；支持浮层时不创建此状态，无返回值
    pub(super) fn configure_scroll_window(&mut self, rect: Rect, paused: bool) {
        if self.runtime.state.layer_shell.is_some() {
            return;
        }
        match &mut self.scroll_window {
            Some(panel) => {
                panel.rect = rect;
                panel.paused = paused;
            }
            None => {
                self.scroll_window = Some(ScrollWindow {
                    rect,
                    paused,
                    hidden: false,
                    last_bounds: None,
                });
            }
        }
    }

    /// 【滚动截图】【面板提交】将 monitor_idx 的完整画布 pixels 裁为面板；返回是否已处理，其他角色由原路径提交
    pub(super) fn stage_scroll_window(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
    ) -> crate::AppResult<bool> {
        if self.scrolling != Some(monitor_idx) {
            return Ok(false);
        }
        let Some(panel) = &mut self.scroll_window else {
            return Ok(false);
        };
        if panel.hidden && !panel.paused {
            return Ok(true);
        }
        let target = self
            .targets
            .get(monitor_idx)
            .ok_or("the scroll window output is gone")?;
        let runtime = &mut self.runtime;
        let output = runtime
            .state
            .outputs
            .iter()
            .find(|output| output.wl_output == *target)
            .ok_or("the scroll window output is gone")?;
        let (width, height) = output
            .info
            .logical_size
            .ok_or("the scroll window output has no dimensions")?;
        let scale = if runtime.state.viewporter.is_some() {
            output.scale()
        } else {
            1.0
        };
        let source = crate::types::px_size((width as u32, height as u32), scale);
        let rect = panel.rect;
        let x = (rect.left() * scale).round() as u32;
        let y = (rect.top() * scale).round() as u32;
        let right = (rect.right() * scale).round() as u32;
        let bottom = (rect.bottom() * scale).round() as u32;
        if right <= x
            || bottom <= y
            || right > source.0
            || bottom > source.1
            || pixels.len() != source.0 as usize * source.1 as usize * 4
        {
            return Err("the scroll panel does not fit the original canvas".into());
        }
        // 1. 【滚动截图】【原画布裁切】复制真实面板像素，不能缩放原截图或按普通窗口位置改变捕获区域
        let surface = runtime
            .state
            .surfaces
            .get_mut(&monitor_idx)
            .ok_or("the scroll control window is gone")?;
        let mut frame = surface
            .scroll_frame
            .take()
            .map_or_else(Vec::new, |frame| frame.rgba);
        frame.clear();
        for row in y..bottom {
            let start = (row as usize * source.0 as usize + x as usize) * 4;
            let end = start + (right - x) as usize * 4;
            frame.extend_from_slice(&pixels[start..end]);
        }
        surface.scroll_frame = Some(ScrollFrame {
            stamp: Stamp::new(&frame, (right - x, bottom - y), surface.scale),
            rgba: frame,
            size: (right - x, bottom - y),
        });
        let size = (rect.width().ceil() as u32, rect.height().ceil() as u32);
        surface.canvas_rect = Some(rect);
        if let Role::ScrollPopup(popup) = &mut surface.role {
            popup.resize(size);
        }
        let qh = runtime.event_queue.handle();
        runtime.state.paint_scroll_frame(monitor_idx, &qh)?;
        Ok(true)
    }

    /// 【滚动截图】【窗口污染】以 frame 和 region 检查已呈现控件，重叠时释放窗口并拒绝该帧，返回是否接受
    pub(super) fn allow_window_scroll_frame(
        &mut self,
        frame: &crate::types::MonitorFrame,
        region: Rect,
    ) -> crate::AppResult<bool> {
        let Some(index) = self.scrolling else {
            return Ok(true);
        };
        let Some(window) = &mut self.scroll_window else {
            return Ok(true);
        };
        if window.hidden {
            return Ok(true);
        }
        let Some(surface) = self.runtime.state.surfaces.get(&index) else {
            return Ok(true);
        };
        if matches!(&surface.role, Role::ScrollPopup(popup) if popup.pending) {
            return Ok(false);
        }
        if surface.scroll_frame.is_none() {
            return Ok(false);
        }
        let logical = frame
            .info
            .size
            .ok_or("the scroll output has no logical size")?;
        if logical.0 <= 0 {
            return Err("the scroll output has invalid logical width".into());
        }
        let scale = frame.pw_width as f32 / logical.0 as f32;
        window.last_bounds = surface.scroll_stamps.iter().rev().find_map(|presented| {
            let displayed = crate::types::px_size(presented.logical, scale);
            presented
                .stamp
                .locate(frame, displayed, presented.source, window.last_bounds)
        });
        let selected = Rect::from_ltrb(
            region.left() * scale,
            region.top() * scale,
            region.right() * scale,
            region.bottom() * scale,
        )
        .ok_or("invalid scroll region")?;
        if let Some((x, y, width, height)) = window.last_bounds {
            let occupied = Rect::from_xywh(x as f32, y as f32, width as f32, height as f32)
                .ok_or("invalid scroll control bounds")?;
            if occupied.intersect(&selected).is_none() {
                return Ok(true);
            }
        }
        // 2. 【滚动截图】【未知位置】部分遮挡、旧视频帧或缩放变化无法确认位置时，也必须先取得无面板画面
        window.hidden = true;
        self.suspend_surfaces()?;
        log::info!(
            "【滚动截图】【窗口避让】控制位置={:?}，已隐藏并等待干净帧",
            self.scroll_window
                .as_ref()
                .and_then(|window| window.last_bounds)
        );
        Ok(false)
    }

    /// 【滚动截图】【恢复控制】paused 为真时恢复捕获期间隐藏的窗口，返回协议结果，不恢复冻结截图
    pub(super) fn resume_scroll_controls(&mut self, paused: bool) -> crate::AppResult {
        if paused
            && self
                .scroll_window
                .as_ref()
                .is_some_and(|window| window.hidden)
        {
            self.resume_surfaces()?;
            if let Some(window) = &mut self.scroll_window {
                window.hidden = false;
            }
        }
        Ok(())
    }
}
