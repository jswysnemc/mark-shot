//! 【标注选框】【按钮缓存】稳定原始像素相位，局部重绘只合成相交范围，不重复栅格化按钮边缘

use crate::renderer::{Painter, scale_rect};
use tiny_skia::{IntRect, Pixmap, PixmapPaint, Rect, Transform};

pub(crate) struct Cache {
    rect: Rect,
    scale: f32,
    hovered: bool,
    bounds: IntRect,
    pixels: Pixmap,
}

impl Cache {
    /// 【标注选框】【缓存构造】按rect、scale和hovered返回含五像素逻辑留白的小块像素，容纳扩大边框，不分配整屏缓冲
    pub fn new(rect: Rect, scale: f32, hovered: bool) -> Option<Self> {
        let bounds = scale_rect(&rect.outset(5.0, 5.0)?, scale)?.round_out()?;
        let mut pixels = Pixmap::new(bounds.width(), bounds.height())?;
        super::draw::delete_with_hover(
            &mut Painter::new(&mut pixels)
                .with_scale(scale)
                .with_origin((bounds.x() as f32 / scale, bounds.y() as f32 / scale)),
            rect,
            hovered,
        );
        Some(Self {
            rect,
            scale,
            hovered,
            bounds,
            pixels,
        })
    }

    /// 【标注选框】【缓存身份】按rect、scale与hovered返回缓存是否仍适用于当前按钮，无分配
    pub fn matches(&self, rect: Rect, scale: f32, hovered: bool) -> bool {
        self.rect == rect && self.scale == scale && self.hovered == hovered
    }

    /// 【标注选框】【像素合成】把缓存绘入 canvas 的 dirty 物理交集，无返回值，未恢复的像素不参与重复混合
    pub fn composite(&self, canvas: &mut Pixmap, dirty: Option<Rect>) {
        let Some(dirty) = dirty else {
            canvas.draw_pixmap(
                self.bounds.x(),
                self.bounds.y(),
                self.pixels.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
            return;
        };
        let Some(dirty) = scale_rect(&dirty, self.scale)
            .and_then(|rect| rect.round_out())
            .and_then(|rect| rect.intersect(&self.bounds))
        else {
            return;
        };
        let Some(relative) = IntRect::from_xywh(
            dirty.x() - self.bounds.x(),
            dirty.y() - self.bounds.y(),
            dirty.width(),
            dirty.height(),
        ) else {
            return;
        };
        let Some(part) = self.pixels.clone_rect(relative) else {
            return;
        };
        canvas.draw_pixmap(
            dirty.x(),
            dirty.y(),
            part.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【按钮缓存测试】【局部一致】分数比例与非整数位置下，逐块恢复的按钮必须与整块合成逐字节一致
    #[test]
    fn fractional_button_cache_is_identical_when_composited_in_dirty_slices() {
        for (scale, hovered) in [1.0, 1.5, 1280.0 / 853.0, 2.0]
            .into_iter()
            .flat_map(|scale| [false, true].map(|hovered| (scale, hovered)))
        {
            let cache = Cache::new(
                Rect::from_xywh(88.740_85, 64.007324, 20.0, 20.0).unwrap(),
                scale,
                hovered,
            )
            .unwrap();
            let mut whole = Pixmap::new((160.0 * scale) as u32, (140.0 * scale) as u32).unwrap();
            whole.fill(tiny_skia::Color::from_rgba8(30, 34, 38, 255));
            let background = whole.clone();
            let mut partial = whole.clone();
            cache.composite(&mut whole, None);
            for x in (0..partial.width()).step_by(7) {
                for y in (0..partial.height()).step_by(9) {
                    let rect = Rect::from_xywh(
                        x as f32 / scale,
                        y as f32 / scale,
                        7.0 / scale,
                        9.0 / scale,
                    )
                    .unwrap();
                    // 1. 【按钮缓存测试】【背景恢复】每次局部合成先恢复实际像素交集，分数取整造成的交叠不能重复混合旧图像
                    let restored = scale_rect(&rect, scale).unwrap().round_out().unwrap();
                    for row in restored.y().max(0)..restored.bottom().min(partial.height() as i32) {
                        let left = restored.x().max(0) as usize;
                        let right = restored.right().min(partial.width() as i32) as usize;
                        let stride = partial.width() as usize * 4;
                        let start = row as usize * stride + left * 4;
                        let end = row as usize * stride + right * 4;
                        partial.data_mut()[start..end]
                            .copy_from_slice(&background.data()[start..end]);
                    }
                    cache.composite(&mut partial, Some(rect));
                }
            }
            assert_eq!(whole.data(), partial.data(), "scale={scale}");
        }
    }
}
