//! 【序号】【标签排版】拟合有界标签并直接旋转字形，避免分配整段尾部大小的文字缓冲

use cosmic_text::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style, SwashContent, Weight,
};
use tiny_skia::{
    Color, FillRule, FilterQuality, Mask, PathBuilder, Pixmap, PixmapPaint, Rect, Transform,
};

use super::{Painter, text::shape_single_line};
use crate::features::number::window_geometry::Geometry;
use crate::features::number::{NumberMark, NumberStyle};

pub(crate) struct NumberLabel {
    buffer: Buffer,
    position: (f32, f32),
}

impl NumberLabel {
    /// 【序号】【实际标签边界】按mark返回旋转后的排版边界，包含已降到八点仍超出圆框的长标签，缺少有效排版时返回空值
    pub(crate) fn bounds(&self, mark: NumberMark) -> Option<Rect> {
        let width = self
            .buffer
            .layout_runs()
            .map(|run| run.line_w)
            .fold(0.0, f32::max);
        let height: f32 = self.buffer.layout_runs().map(|run| run.line_height).sum();
        let rect = Rect::from_xywh(
            self.position.0,
            self.position.1,
            width.max(1.0),
            height.max(1.0),
        )?;
        let corners = [
            (rect.left(), rect.top()),
            (rect.right(), rect.top()),
            (rect.right(), rect.bottom()),
            (rect.left(), rect.bottom()),
        ]
        .map(|point| mark.to_visual(point));
        Rect::from_ltrb(
            corners
                .iter()
                .map(|point| point.0)
                .fold(f32::INFINITY, f32::min),
            corners
                .iter()
                .map(|point| point.1)
                .fold(f32::INFINITY, f32::min),
            corners
                .iter()
                .map(|point| point.0)
                .fold(f32::NEG_INFINITY, f32::max),
            corners
                .iter()
                .map(|point| point.1)
                .fold(f32::NEG_INFINITY, f32::max),
        )
    }
}

/// 【序号】【标签拟合】按geometry在painter字体中排版白色标签，返回逐点调整的排版，缺少字体时返回空值
pub(crate) fn prepare(painter: &mut Painter<'_>, geometry: Geometry) -> Option<NumberLabel> {
    let window = painter.window_units();
    let mark = geometry.mark;
    let fonts = painter.fonts.as_mut()?;
    let radius = geometry.radius;
    let label = mark.style.label(mark.number);
    let mut points = (window.window(radius) * 0.92).clamp(12.0, 54.0).round();
    let rect = Rect::from_xywh(
        mark.bubble.0 - radius * 0.79,
        mark.bubble.1 - radius * 0.8,
        radius * 1.58,
        radius * 1.6,
    )?;
    let cjk = mark.style.is_cjk() && !label.is_ascii();
    let (mut buffer, mut width, mut height) = shape(
        fonts.system,
        &label,
        window.font_pixels(window.canvas(points)),
        cjk,
    );
    // 1. 【序号】【窗口字号】原版逐点减小字号直到文字宽度可容纳，最低八点，不连续缩放既有字形
    while points > 8.0 && width > rect.width() {
        points -= 1.0;
        (buffer, width, height) = shape(
            fonts.system,
            &label,
            window.font_pixels(window.canvas(points)),
            cjk,
        );
    }
    Some(NumberLabel {
        buffer,
        position: (
            rect.left() + (rect.width() - width) / 2.0,
            rect.top() + (rect.height() - height) / 2.0,
        ),
    })
}

/// 【序号】【界面预览】把 style 的短预览按 color 绘入 painter 的 rect；中文强制使用内置子集，不依赖系统字体，无返回值
pub(crate) fn draw_preview(
    painter: &mut Painter<'_>,
    rect: Rect,
    style: NumberStyle,
    color: Color,
) {
    let Some(fonts) = painter.fonts.as_mut() else {
        return;
    };
    let label = fit_label(
        fonts.system,
        style.preview(),
        crate::theme::font::label(),
        rect,
        style.is_cjk(),
    );
    let center = (
        rect.left() + rect.width() / 2.0,
        rect.top() + rect.height() / 2.0,
    );
    if let Some(mark) = NumberMark::new(center, 1, style) {
        draw(painter, &label, mark, color);
    }
}

/// 【序号】【排版拟合】用 fonts 按 text、size 在 rect 内排版，cjk 决定独立字形族；返回居中的拟合标签
fn fit_label(fonts: &mut FontSystem, text: &str, size: f32, rect: Rect, cjk: bool) -> NumberLabel {
    let size = size.max(0.1);
    let (mut buffer, mut width, mut height) = shape(fonts, text, size, cjk);
    let fit = (rect.width() / width.max(0.01))
        .min(rect.height() / height.max(0.01))
        .min(1.0);
    if fit < 1.0 {
        (buffer, width, height) = shape(fonts, text, size * fit, cjk);
    }
    NumberLabel {
        buffer,
        position: (
            rect.left() + (rect.width() - width) / 2.0,
            rect.top() + (rect.height() - height) / 2.0,
        ),
    }
}

/// 【序号】【字形排版】按 fonts、text、size 排版，cjk 样式固定使用内置字形族；返回缓冲、逻辑宽度和行高
fn shape(fonts: &mut FontSystem, text: &str, size: f32, cjk: bool) -> (Buffer, f32, f32) {
    if !cjk {
        return shape_single_line(fonts, text, size, Weight::BLACK, Style::Normal);
    }
    let mut buffer = Buffer::new(
        fonts,
        Metrics::new(size, size * crate::theme::font::line_height()),
    );
    buffer.set_size(None, None);
    buffer.set_text(
        text,
        &Attrs::new()
            .family(Family::Name(crate::fonts::NUMERALS_FAMILY))
            .weight(Weight::BLACK),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fonts, false);
    let width = buffer
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0, f32::max);
    let height = buffer.layout_runs().map(|run| run.line_height).sum();
    (buffer, width, height)
}

/// 【序号】【旋转标签】将 label 按 color 围绕 mark 圆心绘入 painter；字形分块最大四 MiB，无返回值
pub(crate) fn draw(painter: &mut Painter<'_>, label: &NumberLabel, mark: NumberMark, color: Color) {
    let scale = painter.scale();
    let origin = painter.origin();
    let center = (mark.bubble.0 * scale, mark.bubble.1 * scale);
    let (sin, cos) = mark.rotation.sin_cos();
    let Some(fonts) = painter.fonts.as_mut() else {
        return;
    };
    for run in label.buffer.layout_runs() {
        for glyph in run.glyphs {
            let physical = glyph.physical((0.0, 0.0), scale);
            let Some(image) = fonts.cache.get_image(fonts.system, physical.cache_key) else {
                continue;
            };
            if image.content != SwashContent::Mask
                || image.placement.width == 0
                || image.placement.height == 0
            {
                continue;
            }
            let base = (
                label.position.0 * scale + physical.x as f32 + image.placement.left as f32,
                (label.position.1 + run.line_y) * scale + physical.y as f32
                    - image.placement.top as f32,
            );
            // 1. 【序号】【旋转标签】字形块保留一像素邻域，双线性旋转不会在分块边缘产生接缝
            for row in (0..image.placement.height).step_by(1022) {
                for column in (0..image.placement.width).step_by(1022) {
                    let source_x = column.saturating_sub(1);
                    let source_y = row.saturating_sub(1);
                    let inner_width = (image.placement.width - column).min(1022);
                    let inner_height = (image.placement.height - row).min(1022);
                    let end_x = (column + inner_width + 1).min(image.placement.width);
                    let end_y = (row + inner_height + 1).min(image.placement.height);
                    let Some(mut pixels) = Pixmap::new(end_x - source_x, end_y - source_y) else {
                        continue;
                    };
                    for (index, pixel) in pixels
                        .data_mut()
                        .as_chunks_mut::<4>()
                        .0
                        .iter_mut()
                        .enumerate()
                    {
                        let x = source_x as usize + index % (end_x - source_x) as usize;
                        let y = source_y as usize + index / (end_x - source_x) as usize;
                        let alpha = f32::from(image.data[y * image.placement.width as usize + x])
                            / 255.0
                            * color.alpha();
                        *pixel = [
                            (color.red() * alpha * 255.0) as u8,
                            (color.green() * alpha * 255.0) as u8,
                            (color.blue() * alpha * 255.0) as u8,
                            (alpha * 255.0) as u8,
                        ];
                    }
                    let delta = (
                        base.0 + source_x as f32 - center.0,
                        base.1 + source_y as f32 - center.1,
                    );
                    let transform = Transform::from_row(
                        cos,
                        sin,
                        -sin,
                        cos,
                        center.0 - origin.0 * scale + cos * delta.0 - sin * delta.1,
                        center.1 - origin.1 * scale + sin * delta.0 + cos * delta.1,
                    );
                    let chunked = image.placement.width > 1022 || image.placement.height > 1022;
                    let mask = if chunked {
                        let left = if column == 0 {
                            -1.0
                        } else {
                            (column - source_x) as f32
                        };
                        let top = if row == 0 {
                            -1.0
                        } else {
                            (row - source_y) as f32
                        };
                        let right = if column + inner_width == image.placement.width {
                            pixels.width() as f32 + 1.0
                        } else {
                            (column + inner_width - source_x) as f32
                        };
                        let bottom = if row + inner_height == image.placement.height {
                            pixels.height() as f32 + 1.0
                        } else {
                            (row + inner_height - source_y) as f32
                        };
                        let mut mask = Mask::new(painter.canvas.width(), painter.canvas.height());
                        if let Some(mask) = mask.as_mut()
                            && let Some(rect) = Rect::from_ltrb(left, top, right, bottom)
                        {
                            let mut path = PathBuilder::new();
                            path.push_rect(rect);
                            if let Some(path) = path.finish() {
                                mask.fill_path(&path, FillRule::Winding, false, transform);
                            }
                        }
                        mask
                    } else {
                        None
                    };
                    if chunked && mask.is_none() {
                        continue;
                    }
                    // 2. 【序号】【旋转标签】内部裁切各自拥有唯一像素，邻域只参与采样，不重复合成透明度
                    painter.canvas.draw_pixmap(
                        0,
                        0,
                        pixels.as_ref(),
                        &PixmapPaint {
                            quality: FilterQuality::Bilinear,
                            ..PixmapPaint::default()
                        },
                        transform,
                        mask.as_ref(),
                    );
                }
            }
        }
    }
}
