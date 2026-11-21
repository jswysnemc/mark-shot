//! 【贴图】【HTML图片】读取本地 file URL 的栅格图片，并为网络或不可读图片生成占位块

use super::super::super::html_style;
use super::super::model::ImageBlock;
use super::super::token::Tag;
use super::{IMAGE_BYTES_LIMIT, IMAGE_COUNT_LIMIT, Parser};
use std::path::PathBuf;

const MAX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ALLOC: u64 = 128 * 1024 * 1024;
const MAX_DIMENSION: u32 = 32768;
const PLACEHOLDER_MIN_WIDTH: f32 = 48.0;
const PLACEHOLDER_MAX_WIDTH: f32 = 220.0;
const PLACEHOLDER_PADDING: f32 = 24.0;
const PLACEHOLDER_ASCII: f32 = 7.0;
const PLACEHOLDER_WIDE: f32 = 12.0;
const PLACEHOLDER_HEIGHT: f32 = 20.0;
const PLACEHOLDER_FALLBACK: (f32, f32) = (120.0, 80.0);

impl Parser {
    /// 【贴图】【图片处理】按 tag 缓冲图片块，表格内回退 alt 文本，文本草稿先写出后再累计图片
    pub(super) fn handle_img(&mut self, tag: &Tag) {
        if self.table_depth > 0 {
            self.push_alt(tag);
            return;
        }
        if self.draft_has_text() {
            self.flush();
        }
        let Some(block) = load(tag) else {
            return;
        };
        let bytes = block.data.as_ref().map_or(0, Vec::len);
        if self.image_count >= IMAGE_COUNT_LIMIT
            || self.image_bytes.saturating_add(bytes) > IMAGE_BYTES_LIMIT
        {
            self.push_alt(tag);
            return;
        }
        if self.images.is_empty() {
            self.image_align = Some(self.effective_align(tag));
        }
        self.image_count += 1;
        self.image_bytes += bytes;
        self.images.push(block);
        self.at_start = true;
        self.space = false;
    }

    /// 【贴图】【alt回退】按 tag 的 alt 文本作为普通片段写入，缺失或空白时忽略
    fn push_alt(&mut self, tag: &Tag) {
        if let Some(alt) = tag.attr("alt").filter(|alt| !alt.trim().is_empty()) {
            let alt = alt.to_string();
            let style = self.span_style();
            self.push_span(&alt, &style);
        }
    }
}

/// 【贴图】【图片读取】按 tag 读取可读本地图片，否则在存在 src 或 alt 时生成占位块
pub(super) fn load(tag: &Tag) -> Option<ImageBlock> {
    let src = tag.attr("src").map(str::trim).filter(|src| !src.is_empty());
    let label = tag.attr("alt").unwrap_or_default().trim().to_string();
    if src.is_none() && label.is_empty() {
        return None;
    }
    if let Some(src) = src
        && let Some(block) = load_local(tag, src, &label)
    {
        return Some(block);
    }
    Some(placeholder(tag, label))
}

/// 【贴图】【本地读取】按 src 读取本地图片，tag 提供显示尺寸、label 提供替代文本；返回图片块，读取失败返回空值
fn load_local(tag: &Tag, src: &str, label: &str) -> Option<ImageBlock> {
    let path = local_path(src)?;
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return None;
    }
    let data = std::fs::read(&path).ok()?;
    let (width, height) = intrinsic(&data)?;
    let (display_width, display_height) = display_size(tag, width, height);
    Some(ImageBlock {
        data: Some(data),
        label: label.to_string(),
        width,
        height,
        display_width,
        display_height,
    })
}

/// 【贴图】【占位块】按声明的宽高或 alt 文本估算占位图尺寸，宽或高单独出现时按正方形处理
fn placeholder(tag: &Tag, label: String) -> ImageBlock {
    let declared_width = attribute(tag, "width").or_else(|| style_length(tag, "width"));
    let declared_height = attribute(tag, "height").or_else(|| style_length(tag, "height"));
    let (width, height) = match (declared_width, declared_height) {
        (Some(width), Some(height)) => (width, height),
        (Some(width), None) => (width, width),
        (None, Some(height)) => (height, height),
        (None, None) => estimate(&label),
    };
    let width = width.round().max(1.0) as u32;
    let height = height.round().max(1.0) as u32;
    ImageBlock {
        data: None,
        label,
        width,
        height,
        display_width: Some(width as f32),
        display_height: Some(height as f32),
    }
}

/// 【贴图】【占位估算】按 alt 文本的 ASCII 与宽字符数量估算宽度并限制在合理区间
fn estimate(label: &str) -> (f32, f32) {
    if label.trim().is_empty() {
        return PLACEHOLDER_FALLBACK;
    }
    let text: f32 = label
        .chars()
        .map(|ch| {
            if ch.is_ascii() {
                PLACEHOLDER_ASCII
            } else {
                PLACEHOLDER_WIDE
            }
        })
        .sum();
    let width = (text + PLACEHOLDER_PADDING).clamp(PLACEHOLDER_MIN_WIDTH, PLACEHOLDER_MAX_WIDTH);
    (width, PLACEHOLDER_HEIGHT)
}

/// 【贴图】【本地路径】按 src 解析 file URL，拒绝非文件协议、非空主机与无效路径
fn local_path(src: &str) -> Option<PathBuf> {
    let url = url::Url::parse(src.trim()).ok()?;
    if !url.scheme().eq_ignore_ascii_case("file") {
        return None;
    }
    if url.host_str().is_some_and(|host| !host.is_empty()) {
        return None;
    }
    url.to_file_path().ok()
}

/// 【贴图】【固有尺寸】按编码字节使用受限解码器读取图片头，返回宽高并拒绝非法图片
fn intrinsic(data: &[u8]) -> Option<(u32, u32)> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_ALLOC);
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    reader.limits(limits);
    let (width, height) = reader.into_dimensions().ok()?;
    (width > 0 && height > 0).then_some((width, height))
}

/// 【贴图】【显示尺寸】按属性或内联样式解析宽高，单边缺失时保持固有宽高比
fn display_size(tag: &Tag, width: u32, height: u32) -> (Option<f32>, Option<f32>) {
    let declared_width = attribute(tag, "width").or_else(|| style_length(tag, "width"));
    let declared_height = attribute(tag, "height").or_else(|| style_length(tag, "height"));
    let ratio = width as f32 / height as f32;
    match (declared_width, declared_height) {
        (Some(width), Some(height)) => (Some(width), Some(height)),
        (Some(width), None) => (Some(width), Some(width / ratio)),
        (None, Some(height)) => (Some(height * ratio), Some(height)),
        (None, None) => (None, None),
    }
}

/// 【贴图】【宽度属性】按 name 读取像素长度属性，无单位数值按像素处理并忽略百分比与 auto
fn attribute(tag: &Tag, name: &str) -> Option<f32> {
    let value = tag.attr(name)?.trim().to_ascii_lowercase();
    if value.is_empty() || value == "auto" || value.contains('%') {
        return None;
    }
    let pixels = if value.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        html_style::length_px(&format!("{value}px"))
    } else {
        html_style::length_px(&value)
    };
    pixels.filter(|pixels| *pixels > 0.0)
}

/// 【贴图】【样式长度】按 name 读取 style 中的像素长度，百分比与 auto 返回空值
fn style_length(tag: &Tag, name: &str) -> Option<f32> {
    let css = tag.attr("style")?;
    for (property, value) in html_style::declarations(css) {
        if property.eq_ignore_ascii_case(name) {
            return html_style::length_px(value).filter(|pixels| *pixels > 0.0);
        }
    }
    None
}
