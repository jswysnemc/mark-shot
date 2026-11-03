use std::path::Path;

use tiny_skia::{IntRect, IntSize, Pixmap, Rect};

use crate::AppResult;

pub struct FileCanvas {
    pub original: Pixmap,
    pub viewport: Rect,
}

pub struct ImageCrop {
    pub pixels: Pixmap,
    pub global_origin: (f32, f32),
    pub scale: f32,
}

pub struct SourceRegion {
    pub bounds: IntRect,
    pub global_origin: (f32, f32),
    pub scale: f32,
}

impl FileCanvas {
    /// 【图片编辑】【读取】解码文件并保留原始像素与透明度；参数 bounds 指定显示区域
    pub fn load(path: &Path, bounds: Rect) -> AppResult<Self> {
        let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(65_536);
        limits.max_image_height = None;
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        let decoded = reader.decode()?;
        if u64::from(decoded.width()) * u64::from(decoded.height()) > 64 * 1024 * 1024 {
            return Err("image pixels exceed the supported limit".into());
        }
        let image = decoded.into_rgba8();
        let (width, height) = image.dimensions();
        let mut bytes = image.into_raw();
        crate::utils::premultiply(&mut bytes);
        let size = IntSize::from_wh(width, height).ok_or("the image is empty")?;
        let original = Pixmap::from_vec(bytes, size).ok_or("invalid image pixels")?;
        Self::new(original, bounds).ok_or_else(|| "invalid image viewport".into())
    }

    /// 【图片编辑】【视口布局】将原图等比居中到 bounds；原图不缩小，仅显示预览按比例缩放
    pub fn new(original: Pixmap, bounds: Rect) -> Option<Self> {
        let available_width = (bounds.width() - 64.0).max(1.0);
        let available_height = (bounds.height() - 96.0).max(1.0);
        let scale = (available_width / original.width() as f32)
            .min(available_height / original.height() as f32)
            .min(1.0);
        let width = original.width() as f32 * scale;
        let height = original.height() as f32 * scale;
        let viewport = Rect::from_xywh(
            bounds.left() + (bounds.width() - width) / 2.0,
            bounds.top() + (bounds.height() - height) / 2.0,
            width,
            height,
        )?;
        Some(Self { original, viewport })
    }

    /// 【图片编辑】【原图裁切】把全局逻辑选区映射回原始像素；返回裁切图、精确原点与比例
    pub fn crop(&self, region: Rect) -> Option<ImageCrop> {
        let source = self.source_region(region)?;
        Some(ImageCrop {
            pixels: self.original.clone_rect(source.bounds)?,
            global_origin: source.global_origin,
            scale: source.scale,
        })
    }

    /// 【图片编辑】【区域映射】计算 region 对应的原图像素范围；返回坐标和比例，不复制图像缓冲
    pub fn source_region(&self, region: Rect) -> Option<SourceRegion> {
        let region = region.intersect(&self.viewport)?;
        let scale = self.original.width() as f32 / self.viewport.width();
        let x = ((region.left() - self.viewport.left()) * scale)
            .floor()
            .max(0.0) as i32;
        let y = ((region.top() - self.viewport.top()) * scale)
            .floor()
            .max(0.0) as i32;
        let right = ((region.right() - self.viewport.left()) * scale)
            .ceil()
            .min(self.original.width() as f32) as i32;
        let bottom = ((region.bottom() - self.viewport.top()) * scale)
            .ceil()
            .min(self.original.height() as f32) as i32;
        let rect = IntRect::from_xywh(x, y, (right - x) as u32, (bottom - y) as u32)?;
        Some(SourceRegion {
            bounds: rect,
            global_origin: (
                self.viewport.left() + x as f32 / scale,
                self.viewport.top() + y as f32 / scale,
            ),
            scale,
        })
    }
}
