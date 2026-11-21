//! 【贴图】【内容分类】颜色、HTML 和复制文件转换，不执行网页脚本或外部链接

use super::{IMAGE_LIMIT, html, render, rich_text, source::Clipboard};

/// 【贴图】【色值解析】按 text 识别 HEX 与三分量 RGB，返回 RGBA；越界及非有限值返回空值
pub(super) fn color(text: &str) -> Option<[u8; 4]> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        let hex = match hex.len() {
            3 | 4 => hex.chars().flat_map(|ch| [ch, ch]).collect::<String>(),
            6 | 8 => hex.into(),
            _ => return None,
        };
        if !hex.is_ascii() {
            return None;
        }
        let mut color = [0, 0, 0, 255];
        for (index, part) in hex.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            color[index] = u8::from_str_radix(std::str::from_utf8(part).ok()?, 16).ok()?;
        }
        return Some(color);
    }
    let rgb = text
        .strip_prefix("rgb(")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(text);
    let parts: Vec<_> = rgb
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 {
        return None;
    }
    let floating = parts
        .iter()
        .any(|part| part.contains('.') || part.contains(['e', 'E']));
    let mut color = [0, 0, 0, 255];
    for (index, part) in parts.iter().enumerate() {
        if floating {
            let value: f32 = part.parse().ok()?;
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return None;
            }
            color[index] = (value * 255.0).round() as u8;
        } else {
            color[index] = part.parse().ok()?;
        }
    }
    Some(color)
}

/// 【贴图】【HTML正文】按 html 保留换行及字符实体，忽略脚本、样式、标签与 Windows HTML 传输头，返回可读正文
pub(super) fn html_text(html: &str) -> String {
    // 1. 【贴图】【HTML正文】移除系统传输头和不可见内容，保留段落换行
    let html = html
        .split_once("<!--StartFragment-->")
        .map_or(html, |(_, html)| {
            html.split("<!--EndFragment-->").next().unwrap_or(html)
        });
    let hidden =
        regex::Regex::new(r"(?is)<(script|style)\b[^>]*>.*?</(?:script|style)\s*>|<!--.*?-->")
            .unwrap();
    let breaks = regex::Regex::new(r"(?i)<(?:br\b[^>]*|/(?:p|div|li|h[1-6]|tr))\s*>").unwrap();
    let tags = regex::Regex::new(r"(?s)<[^>]*>").unwrap();
    let hidden = hidden.replace_all(html, "");
    let breaks = breaks.replace_all(&hidden, "\n");
    let text = tags.replace_all(&breaks, "");
    // 2. 【贴图】【字符实体】将常见命名实体与数值实体转换为原文字符
    let entities =
        regex::Regex::new(r"&(#x[0-9a-fA-F]+|#[0-9]+|amp|lt|gt|quot|apos|nbsp);").unwrap();
    entities
        .replace_all(&text, |capture: &regex::Captures<'_>| {
            let value = &capture[1];
            let character = match value {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some(' '),
                value if value.starts_with("#x") => u32::from_str_radix(&value[2..], 16)
                    .ok()
                    .and_then(char::from_u32),
                value if value.starts_with('#') => value[1..].parse().ok().and_then(char::from_u32),
                _ => None,
            };
            character.map_or_else(|| capture[0].into(), |character| character.to_string())
        })
        .trim()
        .into()
}

/// 【贴图】【内容图像】按 clipboard、repeated 与 paths_enabled 生成 PNG，返回图像或明确的不可贴图错误
pub(super) fn image(
    clipboard: &Clipboard,
    repeated: bool,
    paths_enabled: bool,
) -> crate::AppResult<Vec<u8>> {
    if !clipboard.files.is_empty() {
        // 1. 【贴图】【文件图像】首次贴图尝试单张图片文件，解码有尺寸限制，非图片继续显示路径
        if !repeated
            && clipboard.files.len() == 1
            && std::fs::metadata(&clipboard.files[0])
                .is_ok_and(|metadata| metadata.is_file() && metadata.len() <= IMAGE_LIMIT)
        {
            use std::io::Read;
            let mut options = std::fs::OpenOptions::new();
            options.read(true);
            #[cfg(target_os = "linux")]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(nix::libc::O_NONBLOCK);
            }
            if let Ok(file) = options.open(&clipboard.files[0])
                && file.metadata()?.is_file()
                && file.metadata()?.len() <= IMAGE_LIMIT
            {
                let mut bytes = Vec::new();
                file.take(IMAGE_LIMIT + 1).read_to_end(&mut bytes)?;
                if bytes.len() as u64 <= IMAGE_LIMIT
                    && let Ok(image) =
                        crate::features::pinned_image::model::PinnedImage::decode(&bytes, 1.0)
                {
                    return image.png();
                }
            }
        }
        if !paths_enabled {
            return Err("文件路径贴图已关闭 / File path pinning is disabled".into());
        }
        return render::text_image(
            &clipboard
                .files
                .iter()
                .map(|path| path.to_string_lossy())
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    // 2. 【贴图】【剪贴板内容】图片优先于富文本和纯文本，颜色转换为带数值的色卡
    if let Some(image) = &clipboard.image {
        return Ok(image.clone());
    }
    if let Some(html) = &clipboard.html {
        let rich = html::parse(html);
        if rich.has_content() {
            return rich_text::image(&rich);
        }
        let text = html_text(html);
        if !text.is_empty() {
            return render::text_image(&text);
        }
    }
    if let Some(text) = &clipboard.text {
        return color(text).map_or_else(|| render::text_image(text), render::color_image);
    }
    Err("剪贴板中没有可贴图的内容 / No pinnable clipboard content".into())
}
