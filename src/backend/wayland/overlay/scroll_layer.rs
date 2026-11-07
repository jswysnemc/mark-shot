//! 【滚动截图】【局部子表面】透明父表面负责输入，面板和四条边线只保存自身像素

use super::state::OverlayState;
use crate::{
    backend::wayland::utils::shm::{ShmBuffer, create_shm_buffer},
    renderer::Painter,
    types::scroll_panel::ScrollPanelFrame,
};
use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    shm::slot::SlotPool,
    subcompositor::SubcompositorState,
};
use tiny_skia::{Color, IntRect, Pixmap, Rect};
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
    edges: Vec<Sprite>,
    outline: Option<(Rect, f32, Color)>,
}

/// 【滚动截图】【边线区域】按region返回位于原图之外的四个窄条边界，保留旧描边宽度与五逻辑像素间隔
fn edge_bounds(region: Rect) -> [Rect; 4] {
    let outer = region.outset(5.0, 5.0).unwrap();
    let left = outer.left() - 2.0;
    let top = outer.top() - 2.0;
    [
        Rect::from_xywh(left, top, outer.width() + 4.0, 4.0).unwrap(),
        Rect::from_xywh(left, outer.bottom() - 2.0, outer.width() + 4.0, 4.0).unwrap(),
        Rect::from_xywh(left, top + 4.0, 4.0, (outer.height() - 4.0).max(1.0)).unwrap(),
        Rect::from_xywh(
            outer.right() - 2.0,
            top + 4.0,
            4.0,
            (outer.height() - 4.0).max(1.0),
        )
        .unwrap(),
    ]
}

/// 【滚动截图】【边线像素】按bounds、region、scale和color绘制旧边框的窄缓冲，返回像素，不创建完整输出画布
fn edge_pixels(bounds: IntRect, region: Rect, scale: f32, color: Color) -> Option<Pixmap> {
    let size = crate::types::px_size((bounds.width(), bounds.height()), scale);
    let mut pixels = Pixmap::new(size.0, size.1)?;
    Painter::new(&mut pixels)
        .with_scale(scale)
        .with_origin((bounds.x() as f32, bounds.y() as f32))
        .stroke_path(
            &tiny_skia::PathBuilder::from_rect(region.outset(5.0, 5.0)?),
            color,
            &tiny_skia::Stroke {
                width: 2.0,
                ..Default::default()
            },
        );
    Some(pixels)
}

impl ScrollLayer {
    /// 【滚动截图】【浮层局部绘制】按parent及协议对象将frame面板和边线提交到pool，返回协议结果，父表面保留原输出输入坐标
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
            // 1. 【滚动截图】【独立边线】只在区域、比例或主题变化时更新边线，捕获范围内保持透明
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
