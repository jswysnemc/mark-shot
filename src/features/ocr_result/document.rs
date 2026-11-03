//! 【文字识别】【结果文档】保存原文、识别位置和未经标注的原图

use tiny_skia::Pixmap;

use crate::{AppResult, editor::EditorState, ocr::OcrLine};

pub struct OcrDocument {
    pub text: String,
    pub png: Vec<u8>,
    pub lines: Vec<OcrLine>,
}

impl OcrDocument {
    /// 【文字识别】【结果快照】从 editor 当前 OCR 结果取得全文及原始选区像素；无有效结果返回错误
    pub fn from_editor(editor: &EditorState) -> AppResult<Self> {
        let region = editor
            .ocr
            .view
            .region()
            .ok_or("no recognized region is available")?;
        if !editor.ocr.view.is_active() {
            return Err("no recognized text is available".into());
        }
        let image = if let Some(file) = &editor.file_canvas {
            file.crop(region)
                .ok_or("cannot crop the source image")?
                .pixels
        } else {
            crate::renderer::composite(&editor.captures, &editor.placements, region, None)
                .ok_or("cannot composite the recognized region")?
                .0
        };
        Ok(Self {
            text: editor.ocr.view.all_text(),
            png: crate::utils::encode_png(&image),
            lines: editor.ocr.view.lines.clone(),
        })
    }

    /// 【文字识别】【原图解码】按容量上限解码 png；返回保留透明度的预乘像素图
    pub fn image(&self) -> AppResult<Pixmap> {
        let mut reader =
            image::ImageReader::new(std::io::Cursor::new(&self.png)).with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(32768);
        limits.max_image_height = Some(32768);
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        let rgba = reader.decode()?.into_rgba8();
        let (width, height) = rgba.dimensions();
        let mut bytes = rgba.into_raw();
        crate::utils::premultiply(&mut bytes);
        Pixmap::from_vec(
            bytes,
            tiny_skia::IntSize::from_wh(width, height).ok_or("empty source image")?,
        )
        .ok_or_else(|| "invalid source image".into())
    }
}
