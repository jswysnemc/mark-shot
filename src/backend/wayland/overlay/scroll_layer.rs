//! 【滚动截图】【局部子表面】透明父表面负责输入，面板与选区外侧四条边线各自持有独立局部像素

use super::state::OverlayState;
use crate::{
    backend::wayland::utils::shm::{ShmBuffer, create_shm_buffer},
    types::scroll_panel::{ScrollPanelFrame, edge_bounds, edge_pixels},
};
use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    shm::slot::SlotPool,
    subcompositor::SubcompositorState,
};
use tiny_skia::{Color, IntRect, Rect};
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
    /// 【滚动截图】【子表面创建】按 parent 和协议对象创建无输入子表面，返回角色或协议错误，不分配整屏像素
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

    /// 【滚动截图】【局部像素】按 pixels、size 和 bounds 提交局部图像，pool 提供共享存储，返回协议结果
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
        // 1. 由共享池标记槽位正在使用，服务端释放前不复用同一像素槽位
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
    edges: Vec<Sprite>,
    outline: Option<(Rect, f32, Color)>,
}

impl ScrollLayer {
    /// 【滚动截图】【浮层局部绘制】按 parent 及协议对象将 frame 面板与识别区域四边线框提交到 pool，返回协议结果
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
        // 1. 提交控制面板子表面像素
        if self.panel.is_none() {
            self.panel = Some(Sprite::new(
                parent,
                compositor,
                subcompositor,
                viewporter,
                qh,
            )?);
        }
        self.panel
            .as_mut()
            .unwrap()
            .paint(frame.pixels, frame.size, frame.bounds, pool)?;

        // 2. 在选区外侧五逻辑像素处更新四条无输入选区边线，捕获区域内部保持纯净透明
        let key = (frame.region, frame.scale, frame.border);
        if self.outline != Some(key) {
            while self.edges.len() < 4 {
                self.edges.push(Sprite::new(
                    parent,
                    compositor,
                    subcompositor,
                    viewporter,
                    qh,
                )?);
            }
            for (sprite, area) in self.edges.iter_mut().zip(edge_bounds(frame.region)) {
                let bounds = area.round_out().ok_or("invalid scroll edge bounds")?;
                let pixels = edge_pixels(bounds, frame.region, frame.scale, frame.border)
                    .ok_or("cannot allocate scroll edge")?;
                sprite.paint(
                    pixels.data(),
                    (pixels.width(), pixels.height()),
                    bounds,
                    pool,
                )?;
            }
            self.outline = Some(key);
        }
        Ok(())
    }
}
