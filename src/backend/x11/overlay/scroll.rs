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
        let position = (
            origin.0 + (frame.bounds.x() as f32 * frame.scale).round() as i32,
            origin.1 + (frame.bounds.y() as f32 * frame.scale).round() as i32,
        );
        let size = (u16::try_from(frame.size.0)?, u16::try_from(frame.size.1)?);
        // 1. 【X11滚动截图】【窗口交接】全屏覆盖层退出后创建独立局部窗口，保留输出索引和原窗口输入焦点
        if surface.pixels.is_empty() {
            *surface = Surface::new(self.display.clone(), position, frame.size, true)?;
        }
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
        // 2. 在选区外侧同步维护四条无输入选区描边窄条，不阻挡底层页面点击与滚轮
        self.sync_scroll_edges(origin, frame)?;
        Ok(())
    }

    /// 【X11滚动截图】【选区框同步】按 origin 与 frame 更新捕获区域外侧四条穿透式选区描边窗口，返回协议结果
    fn sync_scroll_edges(
        &mut self,
        origin: (i32, i32),
        frame: &crate::types::scroll_panel::ScrollPanelFrame<'_>,
    ) -> crate::AppResult {
        use x11rb::protocol::shape::{ConnectionExt as _, SK, SO};
        let key = (frame.region, frame.scale, frame.border);
        if self.scroll_outline == Some(key) {
            return Ok(());
        }
        for old in self.scroll_edges.drain(..) {
            let _ = self.display.connection.destroy_window(old.window);
        }
        let outer = frame.region.outset(5.0, 5.0).unwrap_or(frame.region);
        let thickness = (2.0 * frame.scale).round().max(1.0) as u32;
        let left = origin.0 + (outer.left() * frame.scale).round() as i32 - thickness as i32 / 2;
        let top = origin.1 + (outer.top() * frame.scale).round() as i32 - thickness as i32 / 2;
        let width = ((outer.width() * frame.scale).round() as u32 + thickness).max(1);
        let height = ((outer.height() * frame.scale).round() as u32 + thickness).max(1);
        let color = frame.border.to_color_u8();
        let pixel = [color.red(), color.green(), color.blue(), 255];
        let strips = [
            ((left, top), (width, thickness)),
            (
                (left, top + height as i32 - thickness as i32),
                (width, thickness),
            ),
            (
                (left, top + thickness as i32),
                (thickness, height.saturating_sub(thickness * 2).max(1)),
            ),
            (
                (
                    left + width as i32 - thickness as i32,
                    top + thickness as i32,
                ),
                (thickness, height.saturating_sub(thickness * 2).max(1)),
            ),
        ];
        for (pos, strip_size) in strips {
            let mut edge = Surface::new(self.display.clone(), pos, strip_size, true)?;
            let _ = self.display.connection.shape_rectangles(
                SO::SET,
                SK::INPUT,
                ClipOrdering::UNSORTED,
                edge.window,
                0,
                0,
                &[],
            );
            let fill = pixel.repeat((strip_size.0 * strip_size.1) as usize);
            edge.present(&fill)?;
            self.scroll_edges.push(edge);
        }
        self.scroll_outline = Some(key);
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
            f64::from(i32::from(surface.position.0) - origin.0) / f64::from(self.display.ui_scale),
            f64::from(i32::from(surface.position.1) - origin.1) / f64::from(self.display.ui_scale),
        )
    }
}
