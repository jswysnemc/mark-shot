use std::path::Path;

use tiny_skia::{IntRect, IntSize, Pixmap, Rect};

use crate::AppResult;

pub struct FileCanvas {
    pub original: crate::image_source::ImageSource,
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

    /// 【图片编辑】【视口布局】按original和bounds返回等比适配窗口的原版视口，小图允许放大，大原件按需使用只读缓存，不改变原始像素
    pub fn new(original: Pixmap, bounds: Rect) -> Option<Self> {
        let view = crate::features::canvas_navigation::model::Navigation::new(
            (original.width(), original.height()),
            (
                bounds.width().round() as u32,
                bounds.height().round() as u32,
            ),
            true,
        )?;
        let fitted = view.projection().widget_rect;
        let viewport = Rect::from_xywh(
            bounds.left() + fitted.x as f32,
            bounds.top() + fitted.y as f32,
            fitted.width as f32,
            fitted.height as f32,
        )?;
        Some(Self {
            original: crate::image_source::ImageSource::store(original),
            viewport,
        })
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
        let scale = f64::from(self.original.width()) / f64::from(self.viewport.width());
        let rect = crate::renderer::pixel_bounds::map(
            region,
            (self.viewport.left(), self.viewport.top()),
            scale,
        )?
        .intersect(&IntRect::from_xywh(
            0,
            0,
            self.original.width(),
            self.original.height(),
        )?)?;
        Some(SourceRegion {
            bounds: rect,
            global_origin: (
                (f64::from(self.viewport.left()) + f64::from(rect.x()) / scale) as f32,
                (f64::from(self.viewport.top()) + f64::from(rect.y()) / scale) as f32,
            ),
            scale: scale as f32,
        })
    }
}
