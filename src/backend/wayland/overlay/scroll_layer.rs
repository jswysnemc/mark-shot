//! 【滚动截图】【局部子表面】透明父表面负责输入，仅由无边框面板子表面承载自身像素

use super::state::OverlayState;
use crate::{
    backend::wayland::utils::shm::{ShmBuffer, create_shm_buffer},
    types::scroll_panel::ScrollPanelFrame,
};
use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    shm::slot::SlotPool,
    subcompositor::SubcompositorState,
};
use tiny_skia::IntRect;
use wayland_client::{
    QueueHandle,
    protocol::{wl_subsurface::WlSubsurface, wl_surface::WlSurface},
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};

struct Sprite {
    surface: WlSurface,
    subsurface: WlSubsurface,
    viewport: Option<WpViewport>,
    buffer: Option<ShmBuffer>,
}

impl Sprite {
    /// 【滚动截图】【子表面创建】按parent和协议对象创建无输入子表面，返回角色或协议错误，不分配整屏像素
    fn new(
        parent: &WlSurface,
        compositor: &CompositorState,
        subcompositor: &SubcompositorState,
        viewporter: Option<&WpViewporter>,
        qh: &QueueHandle<OverlayState>,
    ) -> crate::AppResult<Self> {
        let (subsurface, surface) = subcompositor.create_subsurface(parent.clone(), qh);
        subsurface.place_above(parent);
        let region = Region::new(compositor)?;
        surface.set_input_region(Some(region.wl_region()));
        let viewport = viewporter.map(|manager| manager.get_viewport(&surface, qh, ()));
        Ok(Self {
            surface,
            subsurface,
            viewport,
            buffer: None,
        })
    }

    /// 【滚动截图】【局部像素】按pixels、size和bounds提交局部图像，pool提供共享存储，返回协议结果
    fn paint(
        &mut self,
        pixels: &[u8],
        size: (u32, u32),
        bounds: IntRect,
        pool: &mut SlotPool,
    ) -> crate::AppResult {
        if pixels.len() != size.0 as usize * size.1 as usize * 4 {
            return Err("invalid scroll sprite pixels".into());
        }
        let mut buffer = create_shm_buffer(pool, size.0, size.1)?;
        buffer.write_pixels(pool, pixels);
        self.subsurface.set_position(bounds.x(), bounds.y());
        if let Some(viewport) = &self.viewport {
            viewport.set_destination(bounds.width() as i32, bounds.height() as i32);
        }
        // 1. 【滚动截图】【缓冲使用期】由工具包标记正在使用，服务端释放前不能重用同一像素槽位
        buffer.buffer.attach_to(&self.surface)?;
        self.surface
            .damage_buffer(0, 0, size.0 as i32, size.1 as i32);
        self.surface.commit();
        self.buffer = Some(buffer);
        Ok(())
    }
}

impl Drop for Sprite {
    /// 【滚动截图】【子表面回收】无参数，释放视区、子角色与表面，像素缓冲由所有者随后释放
    fn drop(&mut self) {
        if let Some(viewport) = &self.viewport {
            viewport.destroy();
        }
        self.subsurface.destroy();
        self.surface.destroy();
    }
}

#[derive(Default)]
pub(crate) struct ScrollLayer {
    panel: Option<Sprite>,
}

impl ScrollLayer {
    /// 【滚动截图】【浮层局部绘制】按 parent 及协议对象将无边框 frame 面板提交到 pool，返回协议结果，捕获区保持纯净无描边
    pub(super) fn paint(
        &mut self,
        parent: &WlSurface,
        compositor: &CompositorState,
        subcompositor: &SubcompositorState,
        viewporter: Option<&WpViewporter>,
        pool: &mut SlotPool,
        qh: &QueueHandle<OverlayState>,
        frame: &ScrollPanelFrame<'_>,
    ) -> crate::AppResult {
        // 1. 按需创建承载极简控制面板的无输入子表面
        if self.panel.is_none() {
            self.panel = Some(Sprite::new(
                parent,
                compositor,
                subcompositor,
                viewporter,
                qh,
            )?);
        }
        // 2. 提交控制面板局部像素，不在选区外侧附加边框子表面
        self.panel
            .as_mut()
            .unwrap()
            .paint(frame.pixels, frame.size, frame.bounds, pool)?;
        Ok(())
    }
}
