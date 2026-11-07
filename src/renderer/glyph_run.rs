//! 【文字绘制】【单次字形】缓冲布局和定点字段共用同一字形缓存及物理绘制路径

use super::Painter;
use cosmic_text::{LayoutRun, SwashContent};
use tiny_skia::{Color, Pixmap, PixmapPaint, PremultipliedColorU8, Transform};

impl Painter<'_> {
    /// 【文字绘制】【字形输出】按 run、pos 和 color 绘制一行字形，复用当前字体缓存与设备比例，无返回值
    pub(crate) fn text_layout_run(&mut self, run: &LayoutRun<'_>, pos: (f32, f32), color: Color) {
        let scale = self.scale();
        let canvas_size = (self.canvas.width(), self.canvas.height());
        let Some(fonts) = self.fonts.as_mut() else {
            return;
        };
        let offset = self.origin;
        for glyph in run.glyphs {
            let color = glyph.color_opt.map_or(color, |color| {
                Color::from_rgba8(color.r(), color.g(), color.b(), color.a())
            });
            let physical = glyph.physical((0.0, 0.0), scale);
            if let Some(image) = fonts.cache.get_image(fonts.system, physical.cache_key) {
                let width = image.placement.width;
                let height = image.placement.height;
                if width == 0 || height == 0 {
                    continue;
                }
                let draw_x = (pos.0 * scale + physical.x as f32 + image.placement.left as f32
                    - offset.0 * scale) as i32;
                let draw_y = ((pos.1 + run.line_y) * scale
                    - image.placement.top as f32
                    - offset.1 * scale) as i32;
                if image.content != SwashContent::Mask {
                    continue;
                }
                // 1. 【文字绘制】【字形裁切】固定小块只分配实际相交的字形部分，完全不可见的字形不创建像素图
                let left = i64::from(draw_x).max(0);
                let top = i64::from(draw_y).max(0);
                let right = (i64::from(draw_x) + i64::from(width)).min(i64::from(canvas_size.0));
                let bottom = (i64::from(draw_y) + i64::from(height)).min(i64::from(canvas_size.1));
                if left >= right || top >= bottom {
                    continue;
                }
                let Some(mut glyph_pixmap) =
                    Pixmap::new((right - left) as u32, (bottom - top) as u32)
                else {
                    continue;
                };
                let from_x = (left - i64::from(draw_x)) as usize;
                let from_y = (top - i64::from(draw_y)) as usize;
                let clipped_width = (right - left) as usize;
                // 2. 【文字绘制】【原始遮罩】沿用完整字形的遮罩和颜色运算，只改变整数像素复制范围
                for (row, masks) in glyph_pixmap
                    .pixels_mut()
                    .chunks_exact_mut(clipped_width)
                    .zip(
                        image
                            .data
                            .chunks_exact(width as usize)
                            .skip(from_y)
                            .take((bottom - top) as usize),
                    )
                {
                    for (pixel, mask_alpha) in
                        row.iter_mut().zip(&masks[from_x..from_x + clipped_width])
                    {
                        let alpha = (f32::from(*mask_alpha) / 255.0) * color.alpha();
                        *pixel = PremultipliedColorU8::from_rgba(
                            (color.red() * alpha * 255.0) as u8,
                            (color.green() * alpha * 255.0) as u8,
                            (color.blue() * alpha * 255.0) as u8,
                            (alpha * 255.0) as u8,
                        )
                        .unwrap_or(PremultipliedColorU8::TRANSPARENT);
                    }
                }
                self.canvas.draw_pixmap(
                    left as i32,
                    top as i32,
                    glyph_pixmap.as_ref(),
                    &PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
            }
        }
    }
}
