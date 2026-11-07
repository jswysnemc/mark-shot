//! 【贴图】【原图绘制】只在内容变化时重绘，复制与保存不包含显示边框和文字选择层

use super::runtime::{Pin, Shell};
use crate::{AppResult, features::pinned_image::placement::Placement, renderer::Painter};
use smithay_client_toolkit::{
    compositor::FrameCallbackData,
    shell::xdg::XdgSurface,
    shm::slot::{Buffer, SlotPool},
};
use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Transform};
use wayland_client::QueueHandle;

/// 【贴图】【缓冲复用】为 size 从 buffers、pool 取得空闲缓冲索引；最多保留三个同尺寸缓冲
pub(super) fn buffer(
    pool: &mut SlotPool,
    buffers: &mut Vec<Buffer>,
    size: (u32, u32),
) -> AppResult<Option<usize>> {
    crate::backend::wayland::utils::buffers::available(pool, buffers, size)
}

impl Pin {
    /// 【贴图】【可见图像】按 pixels、scale、placement 返回裁切显示缓冲；原图、边框和文字层共用完整逻辑原点
    pub fn draw_image(
        &mut self,
        pixels: (u32, u32),
        scale: f32,
        placement: Placement,
    ) -> AppResult<Pixmap> {
        let sx = pixels.0 as f32 / placement.visible.width.max(1) as f32;
        let sy = pixels.1 as f32 / placement.visible.height.max(1) as f32;
        let mut canvas = Pixmap::new(pixels.0, pixels.1).ok_or("cannot allocate pinned image")?;
        canvas.draw_pixmap(
            0,
            0,
            self.image.image.as_ref(),
            &PixmapPaint {
                quality: FilterQuality::Bilinear,
                ..Default::default()
            },
            Transform::from_scale(
                placement.logical.width as f32 * sx / self.image.image.width() as f32,
                placement.logical.height as f32 * sy / self.image.image.height() as f32,
            )
            .post_translate(
                placement.offset.0 as f32 * sx,
                placement.offset.1 as f32 * sy,
            ),
            None,
        );
        if self.translation.active {
            crate::features::image_translation::paint::viewport_xy(
                &mut canvas,
                &self.translation.blocks,
                (
                    placement.offset.0 as f32 * sx,
                    placement.offset.1 as f32 * sy,
                ),
                (
                    placement.logical.width as f32 * sx / self.image.image.width() as f32,
                    placement.logical.height as f32 * sy / self.image.image.height() as f32,
                ),
                &mut self.font_system,
                &mut self.swash,
            );
        }
        if self.text_selection {
            let mut painter = Painter::new(&mut canvas)
                .with_scale(scale)
                .with_origin((-placement.offset.0 as f32, -placement.offset.1 as f32));
            let ratio_x = placement.logical.width as f32 / self.image.image.width() as f32;
            let ratio_y = placement.logical.height as f32 / self.image.image.height() as f32;
            for index in 0..self.text.lines.len() {
                if let Some(selection) = self.text.line_selection(index)
                    && let Some(rect) = tiny_skia::Rect::from_ltrb(
                        selection.x.0 * ratio_x,
                        selection.y.0 * ratio_y,
                        selection.x.1 * ratio_x,
                        selection.y.1 * ratio_y,
                    )
                {
                    painter.fill_rounded(rect, 0.0, crate::theme::Rgba(72, 132, 245, 96));
                }
            }
        }
        let cfg = &crate::config::get().pinned;
        if cfg.border_enabled && cfg.border_width > 0.0 {
            let mut painter = Painter::new(&mut canvas)
                .with_scale(scale)
                .with_origin((-placement.offset.0 as f32, -placement.offset.1 as f32));
            let inset = cfg.border_width / 2.0;
            if let Some(rect) = tiny_skia::Rect::from_ltrb(
                inset,
                inset,
                placement.logical.width as f32 - inset,
                placement.logical.height as f32 - inset,
            ) {
                let mut path = tiny_skia::PathBuilder::new();
                path.push_rect(rect);
                if let Some(path) = path.finish() {
                    painter.stroke_path(
                        &path,
                        cfg.border_color,
                        &tiny_skia::Stroke {
                            width: cfg.border_width,
                            ..Default::default()
                        },
                    );
                }
            }
        }
        Ok(canvas)
    }

    /// 【贴图】【帧提交】按当前原图和逻辑大小提交一帧；qh 用于帧回调，缓冲忙碌时等待释放
    pub fn render(&mut self, qh: &QueueHandle<Self>) -> AppResult {
        let pixels = (
            (self.placement.visible.width as f32 * self.render_scale).ceil() as u32,
            (self.placement.visible.height as f32 * self.render_scale).ceil() as u32,
        );
        let Some(index) = buffer(&mut self.pool, &mut self.buffers, pixels)? else {
            return Ok(());
        };
        let canvas =
            if self.drag.is_some() && self.preview.as_ref().is_some_and(|preview| preview.ready) {
                Pixmap::new(pixels.0, pixels.1).ok_or("cannot allocate pinned input buffer")?
            } else {
                self.draw_image(pixels, self.render_scale, self.placement)?
            };
        let buffer = &self.buffers[index];
        crate::utils::copy_swizzled(
            buffer
                .canvas(&mut self.pool)
                .ok_or("pin buffer is active")?,
            canvas.data(),
        );
        if let Some(viewport) = &self.viewport {
            viewport.set_destination(
                self.placement.visible.width as i32,
                self.placement.visible.height as i32,
            );
        } else {
            self.shell
                .surface()
                .set_buffer_scale(self.render_scale as i32);
        }
        // 1. 【贴图】【原子提交】客户端几何、视口与新缓冲一起生效，避免原生缩放收到旧尺寸反馈
        if let Shell::Window(window) = &self.shell {
            window
                .xdg_surface()
                .set_window_geometry(0, 0, self.size.0 as i32, self.size.1 as i32);
        }
        buffer.attach_to(self.shell.surface())?;
        self.shell
            .surface()
            .damage_buffer(0, 0, pixels.0 as i32, pixels.1 as i32);
        self.shell
            .surface()
            .frame(qh, FrameCallbackData(self.shell.surface().clone()));
        self.shell.surface().commit();
        self.frame_pending = true;
        self.dirty = false;
        // 2. 【贴图】【画面交还】主表面完成提交后才删除预览，避免释放鼠标时出现空白
        if self.drag.is_none() {
            self.preview = None;
        }
        Ok(())
    }
}
