//! 【截图绘制】【区域状态】明确记录区域绘制模式，仅在尚未适用分块的场景分配完整缓存

use super::EditorState;
use crate::types::{Annotation, AnnotationShape};
use tiny_skia::Pixmap;

#[derive(Default)]
pub(crate) struct State {
    pub(crate) enabled: bool,
    pub(crate) cached_annotations: bool,
    pub(crate) opaque_background: bool,
    pub(crate) native_background: bool,
    pub(crate) native_scene: bool,
    pub(crate) native_hover: bool,
    pub(crate) baked_background: bool,
    pub(crate) navigation_caches:
        Vec<std::cell::RefCell<crate::renderer::navigation_background::TileCache>>,
}

#[cfg(test)]
#[path = "region_rendering_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "region_rendering_tool_tests.rs"]
mod tool_tests;

/// 【标注渲染】【缓存需求】按annotation返回是否需要保留旧完整栅格，已验证的固定路径图形可以使用区域合成
fn requires_cache(annotation: &Annotation) -> bool {
    !matches!(
        annotation.shape,
        AnnotationShape::Rectangle { .. }
            | AnnotationShape::Circle { .. }
            | AnnotationShape::Marker { .. }
            | AnnotationShape::Ruler { .. }
    )
}

impl EditorState {
    /// 【截图绘制】【区域启用】将当前编辑器切换为区域绘制，释放旧整屏遮罩和标注层，无参数、无返回值
    pub(crate) fn enable_region_rendering(&mut self) {
        self.region_rendering.enabled = true;
        self.region_rendering.cached_annotations = false;
        for layer in self.dimmed.iter_mut().chain(&mut self.annotations_layer) {
            *layer = Pixmap::new(1, 1).expect("cannot allocate layer placeholder");
        }
        self.annotations_dirty = true;
    }

    /// 【标注渲染】【完整缓存】确保持久标注使用原有输出尺寸栅格，无参数、无返回值，首次分配时重建现有正文
    pub(crate) fn ensure_annotation_cache(&mut self) {
        if !self.region_rendering.enabled || self.region_rendering.cached_annotations {
            return;
        }
        self.region_rendering.cached_annotations = true;
        for (index, placement) in self.placements.iter().enumerate() {
            let (width, height) = placement.px_size();
            self.annotations_layer[index] =
                Pixmap::new(width, height).expect("cannot allocate annotation cache");
            // 1. 【标注缓存】【稀疏分配】保留完整栅格坐标，仅限制大页扩张，细线与笔迹不占有未触及的大块物理页
            crate::utils::transparent_pixmap::prepare_sparse(&mut self.annotations_layer[index]);
            crate::renderer::rebuild_annotations_layer(
                &mut crate::renderer::Painter::new(&mut self.annotations_layer[index])
                    .with_scale(placement.scale)
                    .with_origin(placement.offset())
                    .with_frozen_image(crate::renderer::FrozenImage::from_parts(
                        self.file_canvas.as_ref(),
                        &self.captures,
                        &self.placements,
                    ))
                    .with_drawing_bounds(self.ruler.label_bounds)
                    .with_text(&mut self.text.font_system, &mut self.text.swash_cache),
                &self.annotations,
                &mut self.text.editors,
                &self.filters.images,
                self.text_session
                    .as_ref()
                    .and_then(|session| session.original_id),
                None,
            );
            crate::utils::transparent_pixmap::reclaim_blank(&mut self.annotations_layer[index]);
        }
    }

    /// 【标注渲染】【提交缓存需求】按annotation准备旧栅格；返回是否应执行旧缓存正文绘制
    pub(crate) fn cache_annotation_pixels(&mut self, annotation: &Annotation) -> bool {
        if self.region_rendering.enabled && requires_cache(annotation) {
            self.ensure_annotation_cache();
        }
        !self.region_rendering.enabled || self.region_rendering.cached_annotations
    }

    /// 【截图绘制】【缓存准备】按当前正文和视图分配必要缓存，恒等视图释放导航背景，无参数、无返回值
    pub(crate) fn prepare_region_layers(&mut self) {
        if !self.region_rendering.enabled {
            return;
        }
        // 1. 【截图场景】【乘法底图】荧光笔依赖既有遮罩，保留原合成顺序并提交完整不透明前景
        self.region_rendering.baked_background = self.region_rendering.native_scene
            && self
                .annotations
                .iter()
                .chain(self.pending.iter())
                .any(|annotation| matches!(annotation.shape, AnnotationShape::Highlighter { .. }));
        let cache = self
            .annotations
            .iter()
            .chain(self.pending.iter())
            .chain(
                self.text_session
                    .as_ref()
                    .map(|session| session.presented()),
            )
            .any(requires_cache);
        if cache {
            self.ensure_annotation_cache();
        } else if self.region_rendering.cached_annotations {
            self.region_rendering.cached_annotations = false;
            for layer in &mut self.annotations_layer {
                *layer = Pixmap::new(1, 1).expect("cannot allocate annotation placeholder");
            }
        }
        let identity = self.canvas_transform().is_identity();
        self.region_rendering
            .navigation_caches
            .resize_with(self.placements.len(), Default::default);
        for (index, (layer, placement)) in self.dimmed.iter_mut().zip(&self.placements).enumerate()
        {
            let mut cache = self.region_rendering.navigation_caches[index].borrow_mut();
            if identity {
                cache.invalidate();
            }
            let size = if identity || self.region_rendering.native_scene && !cache.ready() {
                (1, 1)
            } else {
                placement.px_size()
            };
            if (layer.width(), layer.height()) != size {
                *layer =
                    Pixmap::new(size.0, size.1).expect("cannot allocate navigation background");
            }
        }
    }
}
