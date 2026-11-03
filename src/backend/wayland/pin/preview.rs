//! 【贴图】【拖动预览】保留原表面的隐式指针抓取，独立无输入 layer 显示可跨输出的图像

use super::{render::buffer, runtime::Pin};
use crate::{AppResult, features::pinned_image::placement::Placement};
use smithay_client_toolkit::{
    compositor::{FrameCallbackData, Region},
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
    },
    shm::slot::Buffer,
};
use wayland_client::{QueueHandle, protocol::wl_output};
use wayland_protocols::wp::{
    fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1,
    viewporter::client::wp_viewport::WpViewport,
};

pub(super) struct Preview {
    pub layer: LayerSurface,
    pub output: wl_output::WlOutput,
    pub placement: Placement,
    pub viewport: Option<WpViewport>,
    pub fractional: Option<WpFractionalScaleV1>,
    pub scale: f32,
    pub configured: bool,
    pub ready: bool,
    pub dirty: bool,
    pub frame_pending: bool,
    pub buffers: Vec<Buffer>,
}

impl Drop for Preview {
    /// 【贴图】【预览释放】先释放比例和视口，再销毁 layer；不保留拖动结束后的协议对象
    fn drop(&mut self) {
        if let Some(fractional) = self.fractional.take() {
            fractional.destroy();
        }
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
    }
}

impl Pin {
    /// 【贴图】【预览更新】按 output、placement 更新预览；qh 绑定新表面，原输入表面保持不动
    pub fn update_preview(
        &mut self,
        output: &wl_output::WlOutput,
        placement: Placement,
        qh: &QueueHandle<Self>,
    ) -> AppResult {
        if self
            .preview
            .as_ref()
            .is_none_or(|preview| &preview.output != output)
        {
            let Some(layers) = &self.layers else {
                return Ok(());
            };
            let layer = layers.create_layer_surface(
                qh,
                self.compositor.create_surface(qh),
                Layer::Top,
                Some("mark-shot-pin-preview"),
                Some(output),
            );
            layer.set_anchor(Anchor::TOP | Anchor::LEFT);
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            layer.set_exclusive_zone(-1);
            let region = Region::new(&self.compositor)?;
            layer
                .wl_surface()
                .set_input_region(Some(region.wl_region()));
            let viewport = self
                .viewporter
                .as_ref()
                .map(|manager| manager.get_viewport(layer.wl_surface(), qh, ()));
            let fractional = viewport.as_ref().and_then(|_| {
                self.fractional_manager
                    .as_ref()
                    .map(|manager| manager.get_fractional_scale(layer.wl_surface(), qh, ()))
            });
            let scale = self
                .outputs
                .info(output)
                .map_or(1.0, |info| info.scale_factor.max(1) as f32);
            self.preview = Some(Preview {
                layer,
                output: output.clone(),
                placement,
                viewport,
                fractional,
                scale,
                configured: false,
                ready: false,
                dirty: true,
                frame_pending: false,
                buffers: Vec::new(),
            });
        }
        if let Some(preview) = &mut self.preview {
            preview.placement = placement;
            preview.dirty = true;
            preview
                .layer
                .set_size(placement.visible.width, placement.visible.height);
            preview
                .layer
                .set_margin(placement.margin.1, 0, 0, placement.margin.0);
            preview.layer.wl_surface().commit();
        }
        Ok(())
    }

    /// 【贴图】【预览提交】向 qh 对应预览提交可见图像；等待配置或帧回调时不分配，不启动定时器
    pub fn render_preview(&mut self, qh: &QueueHandle<Self>) -> AppResult {
        let Some(mut preview) = self.preview.take() else {
            return Ok(());
        };
        let result = (|| {
            if self.drag.is_none() || !preview.configured || !preview.dirty || preview.frame_pending
            {
                return Ok(());
            }
            let scale = if preview.viewport.is_some() {
                preview.scale
            } else {
                preview.scale.ceil()
            };
            let size = (
                (preview.placement.visible.width as f32 * scale).ceil() as u32,
                (preview.placement.visible.height as f32 * scale).ceil() as u32,
            );
            let Some(index) = buffer(&mut self.pool, &mut preview.buffers, size)? else {
                return Ok(());
            };
            let canvas = self.draw_image(size, scale, preview.placement)?;
            let buffer = &preview.buffers[index];
            crate::utils::copy_swizzled(
                buffer
                    .canvas(&mut self.pool)
                    .ok_or("preview buffer is active")?,
                canvas.data(),
            );
            if let Some(viewport) = &preview.viewport {
                viewport.set_destination(
                    preview.placement.visible.width as i32,
                    preview.placement.visible.height as i32,
                );
            } else {
                preview.layer.wl_surface().set_buffer_scale(scale as i32);
            }
            buffer.attach_to(preview.layer.wl_surface())?;
            preview
                .layer
                .wl_surface()
                .damage_buffer(0, 0, size.0 as i32, size.1 as i32);
            preview
                .layer
                .wl_surface()
                .frame(qh, FrameCallbackData(preview.layer.wl_surface().clone()));
            preview.layer.wl_surface().commit();
            if !preview.ready {
                self.dirty = true;
            }
            preview.ready = true;
            preview.dirty = false;
            preview.frame_pending = true;
            Ok(())
        })();
        self.preview = Some(preview);
        result
    }
}
