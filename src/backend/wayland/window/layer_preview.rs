//! 【原生窗口】【浮层拖动预览】保持被抓取主表面静止，通过无输入预览浮层按帧跟随指针以消除坐标振荡

use super::{layer_geometry::Geometry, runtime::NativeWindow, surface::SurfaceId};
use crate::{AppResult, backend::wayland::utils::shm_pool::Buffer};
use smithay_client_toolkit::{
    compositor::{FrameCallbackData, Region},
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
    },
};
use tiny_skia::Pixmap;
use wayland_client::{
    QueueHandle,
    protocol::{wl_output::WlOutput, wl_shm, wl_surface::WlSurface},
};
use wayland_protocols::wp::{
    fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1,
    viewporter::client::wp_viewport::WpViewport,
};

pub(super) struct LayerPreview {
    pub layer: LayerSurface,
    pub output: WlOutput,
    pub geometry: Geometry,
    pub viewport: Option<WpViewport>,
    pub fractional: Option<WpFractionalScaleV1>,
    pub scale: f32,
    pub configured: bool,
    pub ready: bool,
    pub dirty: bool,
    pub position_dirty: bool,
    pub frame_pending: bool,
    pub buffers: Vec<Buffer>,
    canvas: Option<Pixmap>,
}

impl Drop for LayerPreview {
    /// 【原生窗口】【预览释放】销毁预览浮层关联的分数缩放与视区协议对象，无参数、无返回值
    fn drop(&mut self) {
        if let Some(fractional) = self.fractional.take() {
            fractional.destroy();
        }
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
    }
}

impl LayerPreview {
    /// 【原生窗口】【预览匹配】判断 surface 是否属于当前拖动预览浮层，返回布尔结果
    pub fn matches(&self, surface: &WlSurface) -> bool {
        self.layer.wl_surface() == surface
    }
}

impl NativeWindow {
    /// 【原生窗口】【预览输出】按 geometry 中心点查找目标输出及其逻辑原点，越界时回退到当前置顶输出
    pub(super) fn output_for_geometry(&self, geometry: Geometry) -> Option<(WlOutput, (i32, i32))> {
        let center = (
            geometry
                .position
                .0
                .saturating_add((geometry.size.0 / 2) as i32),
            geometry
                .position
                .1
                .saturating_add((geometry.size.1 / 2) as i32),
        );
        for output in self.outputs.outputs() {
            let Some(info) = self.outputs.info(&output) else {
                continue;
            };
            let Some(bounds) = info.logical_size else {
                continue;
            };
            let origin = info.logical_position.unwrap_or_default();
            if center.0 >= origin.0
                && center.0 < origin.0.saturating_add(bounds.0)
                && center.1 >= origin.1
                && center.1 < origin.1.saturating_add(bounds.1)
            {
                return Some((output, origin));
            }
        }
        self.stacking_output()
            .map(|placement| (placement.output, placement.origin))
    }

    /// 【原生窗口】【预览更新】按 geometry 与 qh 创建或定位无输入拖动预览浮层，返回协议执行结果
    pub(super) fn update_layer_preview(
        &mut self,
        geometry: Geometry,
        qh: &QueueHandle<Self>,
    ) -> AppResult {
        let Some((output, origin)) = self.output_for_geometry(geometry) else {
            return Ok(());
        };
        let mut target = geometry;
        target.output_origin = origin;
        target.initialized = true;
        // 1. 【原生窗口】【跨屏重建】首次拖动或跨显示器时建立空输入区域的预览浮层，不打断原表面指针抓取
        if self
            .layer_preview
            .as_ref()
            .is_none_or(|preview| preview.output != output)
        {
            let Some(layers) = &self.layers else {
                return Ok(());
            };
            let level = if self.content.above_pinned_windows() {
                Layer::Overlay
            } else {
                Layer::Top
            };
            let surface = self.compositor.create_surface(qh);
            let layer = layers.create_layer_surface(
                qh,
                surface,
                level,
                Some(format!("{}-drag", self.main.spec.app_id)),
                Some(&output),
            );
            layer.set_anchor(Anchor::TOP | Anchor::LEFT);
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            layer.set_exclusive_zone(-1);
            let region = Region::new(&self.compositor)?;
            layer
                .wl_surface()
                .set_input_region(Some(region.wl_region()));
            let viewport = self.viewporter.as_ref().map(|manager| {
                manager.get_viewport(layer.wl_surface(), qh, SurfaceId::LayerPreview)
            });
            let fractional = viewport.as_ref().and_then(|_| {
                self.fractional_manager.as_ref().map(|manager| {
                    manager.get_fractional_scale(layer.wl_surface(), qh, SurfaceId::LayerPreview)
                })
            });
            let scale = if fractional.is_some() {
                self.main.scale
            } else {
                self.outputs
                    .info(&output)
                    .map_or(self.main.scale, |info| info.scale_factor.max(1) as f32)
            };
            let (left, top) = target.margin();
            layer.set_size(target.size.0, target.size.1);
            layer.set_margin(top, 0, 0, left);
            layer.wl_surface().commit();
            self.layer_preview = Some(LayerPreview {
                layer,
                output,
                geometry: target,
                viewport,
                fractional,
                scale,
                configured: false,
                ready: false,
                dirty: true,
                position_dirty: false,
                frame_pending: false,
                buffers: Vec::new(),
                canvas: None,
            });
            return Ok(());
        }
        // 2. 【原生窗口】【位移合并】同一帧内的多次指针移动只记录最新边距，统一在帧提交阶段发送
        if let Some(preview) = &mut self.layer_preview {
            preview.geometry = target;
            preview.position_dirty = true;
        }
        Ok(())
    }

    /// 【原生窗口】【预览帧提交】按 qh 向预览浮层提交像素或合并后的边距更新，受帧回调节流，返回错误结果
    pub(super) fn render_layer_preview(&mut self, qh: &QueueHandle<Self>) -> AppResult {
        let Some(mut preview) = self.layer_preview.take() else {
            return Ok(());
        };
        let active_drag = self
            .layer_gesture
            .is_some_and(|gesture| !gesture.resizing());
        let result = (|| -> AppResult {
            if !active_drag || !preview.configured || preview.frame_pending {
                return Ok(());
            }
            if !preview.dirty && !preview.position_dirty {
                return Ok(());
            }
            let (left, top) = preview.geometry.margin();
            preview
                .layer
                .set_size(preview.geometry.size.0, preview.geometry.size.1);
            preview.layer.set_margin(top, 0, 0, left);
            // 1. 【原生窗口】【纯位移提交】内容未变化时直接复用已附加缓冲并仅更新浮层边距
            if preview.ready && !preview.dirty {
                preview.layer.wl_surface().damage_buffer(0, 0, 1, 1);
                preview
                    .layer
                    .wl_surface()
                    .frame(qh, FrameCallbackData(preview.layer.wl_surface().clone()));
                preview.layer.wl_surface().commit();
                preview.position_dirty = false;
                preview.frame_pending = true;
                return Ok(());
            }
            // 2. 【原生窗口】【像素重绘】首次就绪、缩放变更或面板计时器刷新时绘制最新窗口画面
            let scale = if preview.viewport.is_some() {
                preview.scale
            } else {
                preview.scale.ceil().max(1.0)
            };
            let width = (preview.geometry.size.0 as f32 * scale).ceil() as i32;
            let height = (preview.geometry.size.1 as f32 * scale).ceil() as i32;
            if width <= 0 || height <= 0 {
                return Ok(());
            }
            let fresh = preview.canvas.as_ref().is_none_or(|canvas| {
                canvas.width() != width as u32 || canvas.height() != height as u32
            });
            if fresh {
                preview.canvas = Pixmap::new(width as u32, height as u32);
                preview
                    .buffers
                    .retain(|buffer| buffer.height() == height && buffer.stride() == width * 4);
            }
            let canvas = preview
                .canvas
                .as_mut()
                .ok_or("cannot allocate a layer preview canvas")?;
            self.content.draw(canvas, scale);
            if self.content.rounded_corners() {
                crate::ui::window_frame::clip_rounded_surface(canvas, scale);
            }
            let available = preview
                .buffers
                .iter()
                .position(|buffer| buffer.canvas(&mut self.pool).is_some());
            let index = if let Some(index) = available {
                index
            } else {
                if preview.buffers.len() >= 3 {
                    return Ok(());
                }
                let (buffer, _) =
                    self.pool
                        .create_buffer(width, height, width * 4, wl_shm::Format::Argb8888)?;
                preview.buffers.push(buffer);
                preview.buffers.len() - 1
            };
            let buffer = &preview.buffers[index];
            let bytes = buffer
                .canvas(&mut self.pool)
                .ok_or("layer preview buffer is still active")?;
            crate::utils::copy_swizzled(bytes, canvas.data());
            if let Some(viewport) = &preview.viewport {
                viewport.set_destination(
                    preview.geometry.size.0 as i32,
                    preview.geometry.size.1 as i32,
                );
            } else {
                preview.layer.wl_surface().set_buffer_scale(scale as i32);
            }
            buffer.attach_to(preview.layer.wl_surface())?;
            preview
                .layer
                .wl_surface()
                .damage_buffer(0, 0, width, height);
            preview
                .layer
                .wl_surface()
                .frame(qh, FrameCallbackData(preview.layer.wl_surface().clone()));
            preview.layer.wl_surface().commit();
            if !preview.ready {
                self.main.hidden_for_drag = true;
                self.main.dirty = true;
            }
            preview.ready = true;
            preview.dirty = false;
            preview.position_dirty = false;
            preview.frame_pending = true;
            Ok(())
        })();
        self.layer_preview = Some(preview);
        result
    }
}
