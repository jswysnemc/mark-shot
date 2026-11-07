//! 【滚动截图】【配置提交】只在控制角色尺寸配置完成后提交缓存像素，不在等待中覆盖可见缓冲

use super::OverlayState;
use crate::backend::wayland::utils::{
    shm::create_shm_buffer,
    surface::{PresentedPanel, Role},
};
use smithay_client_toolkit::compositor::FrameCallbackData;
use std::time::Instant;
use wayland_client::QueueHandle;

impl OverlayState {
    /// 【滚动截图】【页面失焦】按 popup 和 qh 重建被页面点击关闭的控制表面，返回是否属于长截图，失败记录配置错误
    pub(crate) fn reopen_scroll_popup(
        &mut self,
        popup: &smithay_client_toolkit::shell::xdg::popup::Popup,
        qh: &QueueHandle<Self>,
    ) -> bool {
        let Some((_, surface)) = self.surfaces.iter_mut().find(|(_, surface)| matches!(&surface.role, Role::ScrollPopup(current) if &current.popup == popup)) else { return false };
        let new_surface = self.compositor_state.create_surface(qh);
        let Role::ScrollPopup(current) = &mut surface.role else {
            return false;
        };
        if let Err(error) = current.reopen(new_surface.clone(), &self.xdg_shell, qh) {
            self.configure_error = Some(error.to_string());
            self.events
                .push_back(crate::types::OverlayEvent::EscapePressed);
            return true;
        }
        if let Some(viewport) = surface.viewport.take() {
            viewport.destroy();
        }
        surface.viewport = self
            .viewporter
            .as_ref()
            .map(|manager| manager.get_viewport(&new_surface, qh, ()));
        surface.surface = new_surface;
        surface.shm_buffer = None;
        surface.frame_sent = None;
        surface.mapped = false;
        self.pointer_surface_idx = None;
        self.pending_flush = true;
        log::info!("【滚动截图】【控件重建】页面获得输入焦点，已重建非独占控制表面");
        true
    }
    /// 【滚动截图】【配置后绘制】按 monitor_idx 和 qh 提交已配置控制表面的最新帧，返回分配错误，不安排空闲重绘
    pub(crate) fn paint_scroll_frame(
        &mut self,
        monitor_idx: usize,
        qh: &QueueHandle<Self>,
    ) -> crate::AppResult {
        let surface = self
            .surfaces
            .get_mut(&monitor_idx)
            .ok_or("the scroll control surface is gone")?;
        if matches!(&surface.role, Role::ScrollPopup(popup) if popup.pending) {
            return Ok(());
        }
        let Some(frame) = &surface.scroll_frame else {
            return Ok(());
        };
        let mut buffer = create_shm_buffer(&mut self.pool, frame.size.0, frame.size.1)?;
        buffer.write_pixels(&mut self.pool, &frame.rgba);
        if let Some(viewport) = &surface.viewport {
            viewport.set_destination(surface.width as i32, surface.height as i32);
        }
        // 1. 【滚动截图】【缓冲使用期】等待服务端释放后才允许槽位重用，避免面板颜色或旧内容混入下一帧
        buffer.buffer.attach_to(&surface.surface)?;
        surface
            .surface
            .damage_buffer(0, 0, frame.size.0 as i32, frame.size.1 as i32);
        surface
            .surface
            .frame(qh, FrameCallbackData(surface.surface.clone()));
        surface.frame_sent = Some(Instant::now());
        surface.mapped = true;
        surface.apply_input(&self.compositor_state);
        surface.shm_buffer = Some(buffer);
        surface.surface.commit();
        // 1. 【滚动截图】【呈现历史】只记实际提交的样本，保留按钮按下和释放期间仍可能在途的旧画面，不保留整帧副本
        if let Some(stamp) = &frame.stamp {
            let logical = (surface.width, surface.height);
            if surface
                .scroll_stamps
                .back()
                .is_none_or(|previous| previous.stamp != *stamp || previous.logical != logical)
            {
                surface.scroll_stamps.push_back(PresentedPanel {
                    stamp: stamp.clone(),
                    source: frame.size,
                    logical,
                });
                while surface.scroll_stamps.len() > 6 {
                    surface.scroll_stamps.pop_front();
                }
            }
        }
        Ok(())
    }
}
