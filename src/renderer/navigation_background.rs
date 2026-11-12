//! 【画布导航】【按需背景】视图变化直接绘入画布，稳定局部帧只缓存实际访问的设备像素块

use super::{FrozenImage, Painter, RenderRequest, background_restore};
use crate::editor::canvas_navigation::transform::ViewTransform;
use std::cell::RefCell;
use tiny_skia::{IntRect, Pixmap, Rect};

const SIDE: u32 = 256;

#[derive(Default)]
pub(crate) struct TileCache {
    size: (u32, u32),
    valid: Vec<bool>,
}

impl TileCache {
    /// 【画布导航】【缓存判断】无参数，返回是否已有有效尺寸的局部缓存，不读取像素
    pub(crate) fn ready(&self) -> bool {
        !self.valid.is_empty()
    }

    /// 【画布导航】【缓存失效】无参数，清除全部块的有效标记，返回值为空；像素所有权由编辑器图层管理
    pub(crate) fn invalidate(&mut self) {
        self.size = (0, 0);
        self.valid.clear();
    }

    /// 【画布导航】【缓存准备】按 pixels 和 size 创建稀疏栅格及有效标记，无返回值，未采样块保持零页
    fn prepare(&mut self, pixels: &mut Pixmap, size: (u32, u32)) {
        if self.size != size || (pixels.width(), pixels.height()) != size {
            *pixels = Pixmap::new(size.0, size.1).expect("cannot allocate navigation tile cache");
            crate::utils::transparent_pixmap::prepare_sparse(pixels);
            #[cfg(target_os = "linux")]
            crate::utils::transparent_pixmap::clear(pixels);
            self.size = size;
            self.valid = vec![false; (size.0.div_ceil(SIDE) * size.1.div_ceil(SIDE)) as usize];
        }
    }
}

/// 【画布导航】【背景区域】按 source、transform、scale、selection、size 和 area 恢复 canvas，返回值为空，原件与遮罩使用原有顺序
fn background(
    canvas: &mut Pixmap,
    source: FrozenImage<'_>,
    transform: tiny_skia::Transform,
    scale: f32,
    selection: Option<Rect>,
    size: (f32, f32),
    area: IntRect,
) {
    crate::theme::canvas::checkerboard_area(canvas, (scale, scale), (0, 0), area);
    source.draw_transformed_region(canvas, transform, Some(area));
    background_restore::shade_region(
        &mut Painter::new(canvas).with_scale(scale),
        selection.as_ref(),
        size,
        Some(area),
    );
}

/// 【画布导航】【损坏恢复】按 req、view、changed 和 cache 恢复背景，无返回值；整帧直接采样，稳定局部输入复用相交块
pub(super) fn restore(
    req: &mut RenderRequest<'_>,
    view: ViewTransform,
    changed: bool,
    cache: &RefCell<TileCache>,
) {
    let Some(area) = background_restore::damage(req.canvas, req.dirty_rect, req.scale) else {
        return;
    };
    let size = (req.canvas.width(), req.canvas.height());
    let full = IntRect::from_xywh(0, 0, size.0, size.1).unwrap();
    let origin = view.origin(req.offset);
    let scale = req.scale * view.scale as f32;
    let transform =
        tiny_skia::Transform::from_translate(-origin.0, -origin.1).post_scale(scale, scale);
    let mut cache = cache.borrow_mut();
    if changed {
        cache.invalidate();
        *req.dimmed = Pixmap::new(1, 1).unwrap();
    }
    // 1. 【画布导航】【整帧直接恢复】缩放或完整重绘不创建第二幅整屏背景
    if changed || area == full {
        background(
            req.canvas,
            req.frozen_image,
            transform,
            req.scale,
            req.selection.copied(),
            req.size,
            area,
        );
        return;
    }
    // 2. 【画布导航】【空闲首次恢复】首次局部帧直接采样实际损坏，不预填相交块之外像素，连续输入再启用块复用
    if !cache.ready() {
        background(
            req.canvas,
            req.frozen_image,
            transform,
            req.scale,
            req.selection.copied(),
            req.size,
            area,
        );
        cache.prepare(req.dimmed, size);
        return;
    }
    cache.prepare(req.dimmed, size);
    let columns = size.0.div_ceil(SIDE);
    // 3. 【画布导航】【稳定局部缓存】只采样尚未访问的设备像素块，重复绘图不重复执行原图插值
    for y in area.top() as u32 / SIDE..=(area.bottom() as u32 - 1) / SIDE {
        for x in area.left() as u32 / SIDE..=(area.right() as u32 - 1) / SIDE {
            let index = (y * columns + x) as usize;
            if !cache.valid[index] {
                let tile = IntRect::from_xywh(
                    (x * SIDE) as i32,
                    (y * SIDE) as i32,
                    SIDE.min(size.0 - x * SIDE),
                    SIDE.min(size.1 - y * SIDE),
                )
                .unwrap();
                background(
                    req.dimmed,
                    req.frozen_image,
                    transform,
                    req.scale,
                    req.selection.copied(),
                    req.size,
                    tile,
                );
                cache.valid[index] = true;
            }
        }
    }
    let stride = size.0 as usize * 4;
    for y in area.top()..area.bottom() {
        let first = y as usize * stride + area.left() as usize * 4;
        let last = first + area.width() as usize * 4;
        req.canvas.data_mut()[first..last].copy_from_slice(&req.dimmed.data()[first..last]);
    }
}
