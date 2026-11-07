//! 【冻结背景】【独立表面】冻结背景只上传一次，透明前景单独交付，保留原始采样与输入区域

use super::WaylandOverlay;
use crate::{
    AppResult,
    backend::wayland::utils::{
        shm::{ShmBuffer, create_shm_buffer},
        surface::Background,
    },
    image_source::ImageSource,
};
use smithay_client_toolkit::{compositor::Region, shm::slot::SlotPool};
use tiny_skia::IntRect;

impl WaylandOverlay {
    /// 【冻结背景】【分批上传】按monitor和source创建同原图网格背景，返回协议结果；读取预算沿用来源行访问，不重新分配完整CPU图像
    pub(super) fn upload_background_source(
        &mut self,
        monitor: usize,
        source: &ImageSource,
    ) -> AppResult {
        let (width, height) = (source.width(), source.height());
        self.install_background(monitor, width, height, |buffer, pool| {
            buffer.write(pool, |native| {
                let stride = width as usize * 4;
                source.visit_rows(
                    IntRect::from_xywh(0, 0, width, height).ok_or("invalid background size")?,
                    |row, rgba| {
                        let offset = row as usize * stride;
                        crate::utils::copy_swizzled(&mut native[offset..offset + stride], rgba);
                    },
                );
                Ok(())
            })
        })
    }

    /// 【冻结背景】【字节上传】按monitor、pixels和原生width/height创建背景，返回协议结果，兼容已有像素调用方
    pub(super) fn upload_background_pixels(
        &mut self,
        monitor: usize,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> AppResult {
        if pixels.len() != width as usize * height as usize * 4 {
            return Err("frozen background dimensions do not match pixels".into());
        }
        self.install_background(monitor, width, height, |buffer, pool| {
            buffer.write(pool, |native| {
                crate::utils::copy_swizzled(native, pixels);
                Ok(())
            })
        })
    }

    /// 【冻结背景】【表面安装】按monitor、原生width/height和upload写入唯一背景槽位，返回协议结果，背景缓冲提交后保持只读
    fn install_background(
        &mut self,
        monitor: usize,
        width: u32,
        height: u32,
        upload: impl FnOnce(&mut ShmBuffer, &mut SlotPool) -> AppResult,
    ) -> AppResult {
        if self.composited_background {
            return Ok(());
        }
        let qh = self.runtime.event_queue.handle();
        let state = &mut self.runtime.state;
        let subcompositor = state
            .subcompositor
            .as_ref()
            .ok_or("compositor has no wl_subcompositor")?;
        let parent = state
            .surfaces
            .get_mut(&monitor)
            .ok_or("surface not found")?;
        if state.viewporter.is_none() && (width, height) != (parent.width, parent.height) {
            return Err("can't scale the background without wp_viewporter".into());
        }
        let (subsurface, surface) = subcompositor.create_subsurface(parent.surface.clone(), &qh);
        subsurface.place_below(&parent.surface);
        let viewport = state.viewporter.as_ref().map(|manager| {
            let viewport = manager.get_viewport(&surface, &qh, ());
            viewport.set_destination(parent.width as i32, parent.height as i32);
            viewport
        });
        if let Ok(region) = Region::new(&state.compositor_state) {
            surface.set_input_region(Some(region.wl_region()));
        }
        let mut buffer = create_shm_buffer(&mut state.pool, width, height)?;
        upload(&mut buffer, &mut state.pool)?;
        // 1. 【冻结背景】【活动登记】背景同样通过槽位接口登记服务端占用，不在呈现期间修改或回收其内容
        buffer.buffer.attach_to(&surface)?;
        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.commit();
        parent.background = Some(Background {
            subsurface,
            surface,
            viewport,
            buffer,
        });
        Ok(())
    }
}
