use tiny_skia::Rect;

use super::{OcrImage, OcrText};
use crate::editor::{EditorState, file_canvas::FileCanvas};

pub struct PreparedImage {
    pub image: OcrImage,
    pub scale: f32,
}

/// 【文字识别】【像素准备】将预乘 RGBA 合成到白色底板并转换为 RGB；透明图片不会变成黑底
fn append_rgb(pixels: &[u8], result: &mut Vec<u8>) {
    for pixel in pixels.as_chunks::<4>().0 {
        let white = 255 - pixel[3];
        result.extend(
            pixel[..3]
                .iter()
                .map(|channel| channel.saturating_add(white)),
        );
    }
}

/// 【文字识别】【原图区域】从本地原图裁切 region；返回原始像素和逻辑坐标比例，不重复缩放预览
pub fn file_region(file: &FileCanvas, region: Rect) -> Option<PreparedImage> {
    let crop = file.source_region(region)?;
    let (width, height) = (crop.bounds.width(), crop.bounds.height());
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    let stride = file.original.width() as usize * 4;
    for row in 0..height as usize {
        let offset = (crop.bounds.y() as usize + row) * stride + crop.bounds.x() as usize * 4;
        append_rgb(
            &file.original.data()[offset..offset + width as usize * 4],
            &mut rgb,
        );
    }
    Some(PreparedImage {
        image: OcrImage {
            rgb,
            width,
            height,
            origin: (
                crop.global_origin.0 * crop.scale,
                crop.global_origin.1 * crop.scale,
            ),
        },
        scale: crop.scale,
    })
}

/// 【文字识别】【输入区域】根据编辑来源选择原图或截图像素；返回识别输入及结果坐标比例
pub fn region(editor: &EditorState, region: Rect) -> Option<PreparedImage> {
    if let Some(file) = &editor.file_canvas {
        return file_region(file, region);
    }
    Some(PreparedImage {
        image: super::composite_region(&editor.captures, &editor.placements, region)?,
        scale: 1.0,
    })
}

/// 【文字识别】【坐标映射】把原始像素识别结果映射回全局逻辑坐标；保留文字和字符边界数量
pub fn normalize(mut text: OcrText, scale: f32) -> OcrText {
    if scale == 1.0 || !scale.is_finite() || scale <= 0.0 {
        return text;
    }
    for line in &mut text.lines {
        let bounds = line.bounds;
        if let Some(rect) = Rect::from_ltrb(
            bounds.left() / scale,
            bounds.top() / scale,
            bounds.right() / scale,
            bounds.bottom() / scale,
        ) {
            line.bounds = rect;
        }
        for x in &mut line.char_x {
            *x /= scale;
        }
    }
    text
}
