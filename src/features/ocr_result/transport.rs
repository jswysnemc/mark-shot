//! 【文字识别】【结果传递】通过有长度上限的二进制帧传递原图，避免 JSON 像素数组占用

use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use tiny_skia::Rect;

use super::document::OcrDocument;
use crate::{AppResult, ocr::OcrLine};

pub const RESULT_ARG: &str = "--ocr-result";
const MAGIC: &[u8; 8] = b"MSOCR001";
const HEADER_LIMIT: usize = 8 * 1024 * 1024;
const IMAGE_LIMIT: usize = 64 * 1024 * 1024;
const LINE_LIMIT: usize = 16384;

#[derive(Serialize, Deserialize)]
struct Header {
    text: String,
    lines: Vec<Line>,
}

#[derive(Serialize, Deserialize)]
struct Line {
    text: String,
    bounds: [f32; 4],
}

/// 【文字识别】【编码结果】将 document 编码到 writer；原图使用 PNG 字节，过大输入返回错误
pub fn write(document: &OcrDocument, mut writer: impl Write) -> AppResult {
    let text_bytes = document
        .lines
        .iter()
        .fold(document.text.len(), |total, line| {
            total.saturating_add(line.text.len())
        });
    if text_bytes > HEADER_LIMIT
        || document.png.is_empty()
        || document.png.len() > IMAGE_LIMIT
        || document.lines.len() > LINE_LIMIT
    {
        return Err("OCR result exceeds the transport limit".into());
    }
    let header = Header {
        text: document.text.clone(),
        lines: document
            .lines
            .iter()
            .map(|line| Line {
                text: line.text.clone(),
                bounds: [
                    line.bounds.x(),
                    line.bounds.y(),
                    line.bounds.width(),
                    line.bounds.height(),
                ],
            })
            .collect(),
    };
    let json = serde_json::to_vec(&header)?;
    if json.len() > HEADER_LIMIT {
        return Err("OCR text exceeds the transport limit".into());
    }
    writer.write_all(MAGIC)?;
    writer.write_all(&(json.len() as u32).to_le_bytes())?;
    writer.write_all(&(document.png.len() as u32).to_le_bytes())?;
    writer.write_all(&json)?;
    writer.write_all(&document.png)?;
    Ok(())
}

/// 【文字识别】【解码结果】从 reader 读取一份有界结果文档；损坏帧、非法文字框及截断输入返回错误
pub fn read(mut reader: impl Read) -> AppResult<OcrDocument> {
    let mut prefix = [0u8; 16];
    reader.read_exact(&mut prefix)?;
    if &prefix[..8] != MAGIC {
        return Err("invalid OCR result frame".into());
    }
    let header_size = u32::from_le_bytes(prefix[8..12].try_into()?) as usize;
    let image_size = u32::from_le_bytes(prefix[12..16].try_into()?) as usize;
    if header_size == 0 || header_size > HEADER_LIMIT || image_size == 0 || image_size > IMAGE_LIMIT
    {
        return Err("OCR result frame exceeds the transport limit".into());
    }
    let mut json = vec![0u8; header_size];
    reader.read_exact(&mut json)?;
    let header: Header = serde_json::from_slice(&json)?;
    if header.text.len() > HEADER_LIMIT || header.lines.len() > LINE_LIMIT {
        return Err("OCR result text exceeds the transport limit".into());
    }
    let lines = header
        .lines
        .into_iter()
        .map(|line| {
            let [x, y, width, height] = line.bounds;
            let bounds = Rect::from_xywh(x, y, width, height).ok_or("invalid OCR text bounds")?;
            Ok(OcrLine {
                text: line.text,
                bounds,
                char_x: vec![],
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    let mut png = vec![0u8; image_size];
    reader.read_exact(&mut png)?;
    Ok(OcrDocument {
        text: header.text,
        png,
        lines,
    })
}

/// 【文字识别】【独立窗口】将 document 发送给独立结果进程；不在结果窗口中加载识别模型
pub fn spawn(document: &OcrDocument) -> AppResult {
    let mut bytes = Vec::with_capacity(document.png.len() + 4096);
    write(document, &mut bytes)?;
    crate::utils::spawn_self_ready(&[RESULT_ARG], &bytes, crate::backend::window::READY_TIMEOUT)
}

/// 【文字识别】【结果入口】读取标准输入中的文档并打开普通窗口；args 非空时返回参数错误
pub fn cli(args: &[String]) -> AppResult {
    use std::io::Write;
    if !args.is_empty() {
        return Err("--ocr-result does not accept additional arguments".into());
    }
    let result = read(std::io::stdin().lock()).and_then(super::run);
    if let Err(error) = &result {
        let _ = writeln!(std::io::stdout().lock(), "{error}");
    }
    result
}
