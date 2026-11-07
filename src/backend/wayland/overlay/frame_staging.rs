//! 【截图覆盖层】【原生帧】最多两个前景缓冲，活动帧只读，透明整页及释放后的备用页按需归还

use super::frame_pixels::FramePixels;
use super::{WaylandOverlay, state::OverlayState};
use crate::{
    AppResult,
    backend::{
        native_backdrop,
        wayland::utils::{shm::create_shm_buffer, surface::SurfaceData},
    },
    image_source::ImageSource,
    types::{DamageRect, OverlayEvent},
};
use smithay_client_toolkit::compositor::FrameCallbackData;
use std::time::Instant;

/// 【截图覆盖层】【可写检查】按surface返回是否有已释放缓冲或尚可创建第二缓冲，不把帧回调视为释放
pub(crate) fn has_writable_buffer(surface: &SurfaceData) -> bool {
    surface
        .shm_buffer
        .as_ref()
        .is_none_or(|buffer| !buffer.active())
        || surface
            .spare_buffer
            .as_ref()
            .is_none_or(|buffer| !buffer.active())
}

/// 【截图覆盖层】【释放进度】按state处理服务端释放的备用页，并唤醒此前等待槽位的输出，无返回值
pub(super) fn released(state: &mut OverlayState) {
    for (&monitor, surface) in &mut state.surfaces {
        if let Some(spare) = &mut surface.spare_buffer {
            spare.reclaim(&mut state.pool)
        }
        if surface.waiting_release && surface.frame_sent.is_none() && has_writable_buffer(surface) {
            surface.waiting_release = false;
            state.events.push_back(OverlayEvent::FrameDone {
                monitor_idx: monitor,
            });
        }
    }
}

impl WaylandOverlay {
    /// 【截图覆盖层】【像素交付】按monitor_idx、pixels、background和damage写入已释放槽位，返回协议结果；局部帧继承最新完整内容
    pub(super) fn stage_pixels(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        background: Option<&ImageSource>,
        damage: Option<DamageRect>,
    ) -> AppResult {
        self.stage_with_pixels(
            monitor_idx,
            FramePixels::Borrowed(pixels),
            background,
            damage,
        )
    }

    /// 【截图覆盖层】【统一像素交付】按monitor_idx、pixels、background和damage写入可用槽位，返回协议结果；消费路径在背景合成前归还已复制CPU页
    pub(super) fn stage_with_pixels(
        &mut self,
        monitor_idx: usize,
        mut pixels: FramePixels<'_>,
        background: Option<&ImageSource>,
        damage: Option<DamageRect>,
    ) -> AppResult {
        let runtime = &mut self.runtime;
        let qh = runtime.event_queue.handle();
        let surface = runtime
            .state
            .surfaces
            .get_mut(&monitor_idx)
            .ok_or("surface not found")?;
        let size = surface.buffer_size();
        let length = size.0 as usize * size.1 as usize * 4;
        if pixels.len() != length {
            return Err("the native frame does not match the configured buffer".into());
        }
        let current = surface
            .shm_buffer
            .as_ref()
            .ok_or("SHM buffer is not configured yet")?;
        let pool = &mut runtime.state.pool;
        let full = damage.is_none_or(|area| area == (0, 0, size.0, size.1));
        // 1. 【截图覆盖层】【服务端隔离】当前缓冲活动时只修改空闲备用槽位，完整帧直接覆盖，局部帧先复制最新可读内容
        if current.active() {
            if surface
                .spare_buffer
                .as_ref()
                .is_some_and(|buffer| buffer.active())
            {
                return Err("both native frame buffers are still active".into());
            }
            let mut spare = match surface.spare_buffer.take() {
                Some(buffer) => buffer,
                None => create_shm_buffer(pool, size.0, size.1)?,
            };
            if !full {
                spare.copy_from(pool, current)?
            }
            let old = surface.shm_buffer.replace(spare).unwrap();
            surface.spare_buffer = Some(old);
        }
        let buffer = surface.shm_buffer.as_mut().unwrap();
        buffer.write(pool, |native| {
            pixels.write_to(native, size, damage)?;
            if let Some(background) = background {
                native_backdrop::compose_bgra(background, native, size, damage)?
            }
            Ok(())
        })?;
        // 2. 【截图覆盖层】【活动登记】通过槽位接口附着并保持所有权，只有wl_buffer.release允许后续修改或回收
        buffer.buffer.attach_to(&surface.surface)?;
        let (x, y, width, height) = damage.unwrap_or((0, 0, size.0, size.1));
        surface
            .surface
            .damage_buffer(x as i32, y as i32, width as i32, height as i32);
        surface
            .surface
            .frame(&qh, FrameCallbackData(surface.surface.clone()));
        surface.frame_sent = Some(Instant::now());
        surface.waiting_release = true;
        surface.surface.commit();
        Ok(())
    }
}
