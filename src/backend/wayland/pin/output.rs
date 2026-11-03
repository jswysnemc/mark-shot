//! 【贴图】【输出生命周期】首次定位只执行一次，之后输出更新不覆盖用户移动与缩放

use super::runtime::{Pin, Shell};
use crate::features::pinned_image::placement::Geometry;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use wayland_client::{Connection, QueueHandle, protocol::wl_output};

impl OutputHandler for Pin {
    /// 【贴图】【输出状态】返回当前连接的输出状态，不创建新连接
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    /// 【贴图】【新增输出】将 output 完整属性交给首次定位逻辑，conn、qh 用于协议更新
    fn new_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.update_output(conn, qh, output);
    }

    /// 【贴图】【输出更新】按 output 的逻辑属性初始化位置，之后只适配目标屏幕变化；qh 用于角色重建
    fn update_output(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        let Some(info) = self.outputs.info(&output) else {
            return;
        };
        let Some((width, height)) = info.logical_size else {
            return;
        };
        if width <= 0 || height <= 0 {
            return;
        }
        let origin = info.logical_position.unwrap_or_default();
        let screen = Geometry::new(origin, (width as u32, height as u32));
        if !self.initialized {
            if let Some((x, y)) = self.initial_position {
                if x < origin.0
                    || y < origin.1
                    || i64::from(x) >= screen.right()
                    || i64::from(y) >= screen.bottom()
                {
                    return;
                }
            } else if self.output.as_ref() != Some(&output) {
                return;
            }
            self.bounds = (width as u32, height as u32);
            self.image
                .set_zoom(1.0, (self.bounds.0 * 9 / 10, self.bounds.1 * 9 / 10));
            let size = self.image.size();
            self.position = self.initial_position.take().unwrap_or((
                origin
                    .0
                    .saturating_add((self.bounds.0.saturating_sub(size.0) / 2) as i32),
                origin
                    .1
                    .saturating_add((self.bounds.1.saturating_sub(size.1) / 2) as i32),
            ));
            // 1. 【贴图】【首次放置】原版首次要求整张图位于目标输出，之后允许拖出屏幕
            self.position.0 = i64::from(self.position.0)
                .clamp(i64::from(origin.0), screen.right() - i64::from(size.0))
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX))
                as i32;
            self.position.1 = i64::from(self.position.1)
                .clamp(i64::from(origin.1), screen.bottom() - i64::from(size.1))
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX))
                as i32;
            self.initialized = true;
            self.resize(false, qh);
        } else if self.output.as_ref() == Some(&output) {
            self.bounds = (width as u32, height as u32);
            self.resize(false, qh);
        }
    }

    /// 【贴图】【输出移除】目标输出移除时将图像交给剩余输出；所有输出消失才退出
    fn output_destroyed(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if self.output.as_ref() == Some(&output)
            || self
                .preview
                .as_ref()
                .is_some_and(|preview| preview.output == output)
        {
            self.preview = None;
            self.drag = None;
            if self.screens().is_empty() {
                self.exit = true;
            } else if matches!(self.shell, Shell::Layer(_)) {
                self.resize(false, qh);
            }
        }
    }
}
