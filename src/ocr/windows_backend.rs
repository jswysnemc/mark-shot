//! 【Windows识别】【系统引擎】使用系统已安装语言包，识别对象只在后台工作线程创建

use super::{OcrBackend, OcrError, OcrImage, OcrLine, OcrText};
use windows::{
    Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
};

pub struct NativeBackend;
struct Apartment(bool);

impl Apartment {
    /// 【Windows识别】【线程初始化】无参数，初始化当前线程 WinRT，已有单线程环境保持原状态，返回所属清理句柄
    fn enter() -> Result<Self, OcrError> {
        let result = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        match result {
            Ok(()) => Ok(Self(true)),
            Err(error) if error.code().0 == 0x80010106u32 as i32 => Ok(Self(false)),
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for Apartment {
    /// 【Windows识别】【线程清理】无参数，仅配对本次成功初始化，不清理调用者持有的线程环境
    fn drop(&mut self) {
        if self.0 {
            unsafe { RoUninitialize() };
        }
    }
}

/// 【Windows识别】【语言检查】无参数，返回系统已安装 OCR 语言，失败包含语言设置指引，不安装系统资源
pub fn languages() -> Result<Vec<String>, String> {
    let query = || -> Result<Vec<String>, OcrError> {
        let _apartment = Apartment::enter()?;
        let available = OcrEngine::AvailableRecognizerLanguages()?;
        let mut names = Vec::new();
        for index in 0..available.Size()? {
            names.push(available.GetAt(index)?.LanguageTag()?.to_string());
        }
        if names.is_empty() {
            return Err("请在 Windows 语言设置中安装 OCR 语言包 / Install an OCR language pack in Windows language settings".into());
        }
        OcrEngine::TryCreateFromUserProfileLanguages().map_err(|error| format!("请在 Windows 语言设置中启用已安装的 OCR 语言 / Enable an installed OCR language in Windows language settings: {error}"))?;
        Ok(names)
    };
    query().map_err(|error| error.to_string())
}

impl OcrBackend for NativeBackend {
    /// 【Windows识别】【文本识别】识别 image 原图并换回原始坐标；cancelled 控制结果交付，返回文本或系统语言错误
    fn recognize(
        &self,
        image: OcrImage,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<OcrText, OcrError> {
        if cancelled() {
            return Err(super::CANCELLED.into());
        }
        let _apartment = Apartment::enter()?;
        let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|error| format!("系统 OCR 没有匹配的语言包，请在 Windows 语言设置中安装 / No matching Windows OCR language pack: {error}"))?;
        let limit = OcrEngine::MaxImageDimension()?;
        let pixels = image::RgbImage::from_raw(image.width, image.height, image.rgb)
            .ok_or("Invalid OCR image")?;
        let scale = (limit as f32 / image.width.max(image.height).max(1) as f32).min(1.0);
        let width = ((image.width as f32 * scale).round() as u32).max(1);
        let height = ((image.height as f32 * scale).round() as u32).max(1);
        let pixels = if width == image.width && height == image.height {
            pixels
        } else {
            image::imageops::resize(
                &pixels,
                width,
                height,
                image::imageops::FilterType::Triangle,
            )
        };
        let mut bytes = Vec::with_capacity(width as usize * height as usize * 4);
        for pixel in pixels.pixels() {
            bytes.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
        }
        // 1. 【Windows识别】【图像交接】通过 WinRT 缓冲复制 BGRA，不暴露指向临时 Rust 内存的指针
        let writer = DataWriter::new()?;
        writer.WriteBytes(&bytes)?;
        let bitmap = SoftwareBitmap::CreateWithAlpha(
            BitmapPixelFormat::Bgra8,
            width as i32,
            height as i32,
            BitmapAlphaMode::Ignore,
        )?;
        bitmap.CopyFromBuffer(&writer.DetachBuffer()?)?;
        let result = engine.RecognizeAsync(&bitmap)?.join()?;
        if cancelled() {
            return Err(super::CANCELLED.into());
        }
        let lines = result.Lines()?;
        let mut output = Vec::new();
        for index in 0..lines.Size()? {
            let line = lines.GetAt(index)?;
            let text = line.Text()?.to_string();
            let words = line.Words()?;
            let mut bounds: Option<tiny_skia::Rect> = None;
            for word in 0..words.Size()? {
                let rect = words.GetAt(word)?.BoundingRect()?;
                if let Some(rect) = tiny_skia::Rect::from_xywh(
                    rect.X / scale + image.origin.0,
                    rect.Y / scale + image.origin.1,
                    rect.Width / scale,
                    rect.Height / scale,
                ) {
                    bounds =
                        Some(bounds.map_or(rect, |bounds| bounds.join(&rect).unwrap_or(bounds)));
                }
            }
            if let Some(bounds) = bounds
                && !text.is_empty()
            {
                // 2. 【Windows识别】【字符选区】系统只提供词级矩形，逐字符边界使用行内插值，不声明精确字符定位
                let count = text.chars().count();
                let char_x = (0..=count)
                    .map(|index| bounds.left() + bounds.width() * index as f32 / count as f32)
                    .collect();
                output.push(OcrLine {
                    text,
                    bounds,
                    char_x,
                });
            }
        }
        Ok(OcrText { lines: output })
    }
}
