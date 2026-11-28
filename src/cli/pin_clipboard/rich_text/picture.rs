//! 【贴图】【图片绘制】测量、排布并绘制单张图片或图片行，网络图片绘制为占位框

use super::super::html::{Align, BlockKind, ImageBlock};
use super::{PADDING, Palette, is_light};
use crate::renderer::Painter;
use crate::renderer::text::TextStyle;
use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Rect, Transform};

const GAP: f32 = 8.0;
const ROW_GAP: f32 = 8.0;
const RADIUS: f32 = 4.0;
const LABEL_SIZE: f32 = 11.0;
const LABEL_SIZE_LARGE: f32 = 12.0;
const LABEL_PADDING: f32 = 8.0;

/// 【贴图】【图片布局】记录单张图片或图片行每个项目的绝对坐标与整体高度
pub(super) struct Layout {
    items: Vec<Placed>,
    height: f32,
}

/// 【贴图】【图片项目】记录一个图片项目的位置、尺寸与在图片列表中的索引
struct Placed {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    index: usize,
}

impl Layout {
    /// 【贴图】【布局高度】无参数，返回图片块占用的总高度
    pub(super) fn height(&self) -> f32 {
        self.height
    }
}

/// 【贴图】【图片块判断】按 kind 返回是否为单张图片或图片行
pub(super) fn is_picture(kind: &BlockKind) -> bool {
    matches!(kind, BlockKind::Image(_) | BlockKind::ImageRow(_))
}

/// 【贴图】【图片测量】按 kind 返回图片块对正文固有宽度的贡献，图片行累计间距
pub(super) fn measure(kind: &BlockKind) -> f32 {
    match kind {
        BlockKind::Image(image) => item_width(image),
        BlockKind::ImageRow(images) => {
            let widths: f32 = images.iter().map(item_width).sum();
            widths + GAP * images.len().saturating_sub(1) as f32
        }
        _ => 0.0,
    }
}

/// 【贴图】【图片布局】按 kind、对齐与可用宽度计算每张图片的位置，超出宽度时换行
pub(super) fn layout(kind: &BlockKind, align: Align, available: f32, top: f32) -> Layout {
    let available = available.max(1.0);
    match kind {
        BlockKind::Image(image) => {
            let (width, height) = fit(item_width(image), item_height(image), available);
            let x = line_x(align, width, available);
            Layout {
                items: vec![Placed {
                    x,
                    y: top,
                    width,
                    height,
                    index: 0,
                }],
                height,
            }
        }
        BlockKind::ImageRow(images) => row(images, align, available, top),
        _ => Layout {
            items: Vec::new(),
            height: 0.0,
        },
    }
}

/// 【贴图】【图片行布局】按 8 像素间距横向排布图片，超出可用宽度时换行并对每行对齐
fn row(images: &[ImageBlock], align: Align, available: f32, top: f32) -> Layout {
    let mut dims: Vec<(usize, f32, f32)> = images
        .iter()
        .enumerate()
        .map(|(index, image)| {
            let (width, height) = fit(item_width(image), item_height(image), available);
            (index, width, height)
        })
        .collect();
    if dims.is_empty() {
        return Layout {
            items: Vec::new(),
            height: 0.0,
        };
    }
    let gaps = GAP * dims.len().saturating_sub(1) as f32;
    let widths: f32 = dims.iter().map(|(_, width, _)| *width).sum();
    if dims.len() <= 12 && widths + gaps > available && widths + gaps <= available * 1.35 {
        let scale = ((available - gaps) / widths).max(0.1);
        for (_, width, height) in &mut dims {
            *width = (*width * scale).max(1.0);
            *height = (*height * scale).max(1.0);
        }
    }
    let mut items = Vec::new();
    let mut y = top;
    let mut start = 0;
    let mut width = 0.0;
    for index in 0..dims.len() {
        let item = dims[index].1;
        let candidate = if index == start {
            item
        } else {
            width + GAP + item
        };
        if index > start && candidate > available {
            place_line(&dims[start..index], align, available, y, &mut items);
            y += line_height(&dims[start..index]) + ROW_GAP;
            start = index;
            width = item;
        } else {
            width = candidate;
        }
    }
    place_line(&dims[start..], align, available, y, &mut items);
    y += line_height(&dims[start..]);
    Layout {
        items,
        height: y - top,
    }
}

/// 【贴图】【单行定位】按一行的图片尺寸与对齐起算横坐标，依次写入已定位项目
fn place_line(
    line: &[(usize, f32, f32)],
    align: Align,
    available: f32,
    y: f32,
    items: &mut Vec<Placed>,
) {
    let width: f32 = line.iter().map(|(_, width, _)| *width).sum::<f32>()
        + GAP * line.len().saturating_sub(1) as f32;
    let mut x = line_x(align, width, available);
    for (index, item_width, item_height) in line {
        items.push(Placed {
            x,
            y,
            width: *item_width,
            height: *item_height,
            index: *index,
        });
        x += item_width + GAP;
    }
}

/// 【贴图】【行高】按一行内最高的图片返回该行高度
fn line_height(line: &[(usize, f32, f32)]) -> f32 {
    line.iter()
        .map(|(_, _, height)| *height)
        .fold(0.0_f32, f32::max)
}

/// 【贴图】【等比缩放】按可用宽度等比缩放图片尺寸，宽度不足时保持原尺寸
fn fit(width: f32, height: f32, available: f32) -> (f32, f32) {
    let mut width = width.max(1.0);
    let mut height = height.max(1.0);
    if width > available {
        height *= available / width;
        width = available;
    }
    (width, height)
}

/// 【贴图】【行起点】按对齐方式在可用宽度内返回一行的左边界
fn line_x(align: Align, width: f32, available: f32) -> f32 {
    let extra = (available - width).max(0.0);
    PADDING
        + match align {
            Align::Left => 0.0,
            Align::Center => extra / 2.0,
            Align::Right => extra,
        }
}

/// 【贴图】【图片尺寸】按 display_width 或固有宽度返回项目宽度，至少为 1 像素
fn item_width(image: &ImageBlock) -> f32 {
    image.display_width.unwrap_or(image.width as f32).max(1.0)
}

/// 【贴图】【图片高度】按 display_height 或固有高度返回项目高度，至少为 1 像素
fn item_height(image: &ImageBlock) -> f32 {
    image.display_height.unwrap_or(image.height as f32).max(1.0)
}

/// 【贴图】【图片绘制】逐项绘制图片行，本地图片缩放绘制，占位图片绘制圆角框与标签
pub(super) fn draw(painter: &mut Painter, kind: &BlockKind, layout: &Layout, palette: &Palette) {
    let images: &[ImageBlock] = match kind {
        BlockKind::Image(image) => std::slice::from_ref(image),
        BlockKind::ImageRow(images) => images,
        _ => return,
    };
    for placed in &layout.items {
        let Some(image) = images.get(placed.index) else {
            continue;
        };
        match &image.data {
            Some(data) => draw_pixmap(painter, data, placed),
            None => draw_placeholder(painter, image, placed, palette),
        }
    }
}

/// 【贴图】【像素绘制】按目标矩形以双线性过滤绘制解码后的图片，解码失败时留白
fn draw_pixmap(painter: &mut Painter, data: &[u8], placed: &Placed) {
    if placed.width <= 0.0 || placed.height <= 0.0 {
        return;
    }
    let Some(pixmap) = decode(data) else {
        return;
    };
    let scale = painter.scale();
    let transform = Transform::from_scale(
        placed.width * scale / pixmap.width() as f32,
        placed.height * scale / pixmap.height() as f32,
    )
    .post_translate(placed.x * scale, placed.y * scale);
    painter.canvas().draw_pixmap(
        0,
        0,
        pixmap.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..PixmapPaint::default()
        },
        transform,
        None,
    );
}

/// 【贴图】【占位绘制】按项目矩形绘制圆角底色并把截断后的 alt 标签居中写入
fn draw_placeholder(painter: &mut Painter, image: &ImageBlock, placed: &Placed, palette: &Palette) {
    let Some(rect) = Rect::from_xywh(placed.x, placed.y, placed.width, placed.height) else {
        return;
    };
    painter.fill_rounded(rect, RADIUS, palette.inline_code.color());
    let style = TextStyle::new(label_size(placed.height), label_color(palette));
    let label = fitted_label(painter, &image.label, placed.width - LABEL_PADDING, style);
    if !label.is_empty() {
        painter.text(rect, &label, style.center());
    }
}

/// 【贴图】【标签字号】按占位高度返回小标签或稍大标签的字号
fn label_size(height: f32) -> f32 {
    if height >= 40.0 {
        LABEL_SIZE_LARGE
    } else if height >= 20.0 {
        LABEL_SIZE
    } else {
        9.0
    }
}

/// 【贴图】【标签颜色】按画布明暗返回可读的次要文字色或前景色
fn label_color(palette: &Palette) -> crate::theme::Rgba {
    if is_light(palette.canvas) {
        crate::theme::color::muted()
    } else {
        palette.foreground
    }
}

/// 【贴图】【标签截断】按最大宽度截断标签并补省略号，空标签或过窄时返回空文本
fn fitted_label(painter: &mut Painter, label: &str, max_width: f32, style: TextStyle) -> String {
    let label = label.trim();
    if label.is_empty() || max_width <= 0.0 {
        return String::new();
    }
    if painter.measure_styled(label, style) <= max_width {
        return label.to_string();
    }
    let mut fitted = String::new();
    for ch in label.chars() {
        let mut candidate = fitted.clone();
        candidate.push(ch);
        candidate.push('…');
        if painter.measure_styled(&candidate, style) > max_width {
            break;
        }
        fitted.push(ch);
    }
    fitted.push('…');
    fitted
}

/// 【贴图】【图片解码】按编码字节复用贴图解码器，返回预乘像素画布
fn decode(data: &[u8]) -> Option<Pixmap> {
    crate::features::pinned_image::model::PinnedImage::decode(data, 1.0)
        .ok()
        .map(|pinned| pinned.image)
}
