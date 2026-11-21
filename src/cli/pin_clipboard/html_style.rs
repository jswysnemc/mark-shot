//! 【贴图】【HTML样式】解析内联 CSS 的颜色、字号、字重、斜体与等宽声明，不依赖外部解析库

use crate::theme::Rgba;

/// 【贴图】【CSS颜色】按 value 解析十六进制、rgb()/rgba() 与常见颜色名，返回 RGBA；透明与 currentcolor 返回空值
pub(super) fn color(value: &str) -> Option<Rgba> {
    let value = strip_important(value);
    if value.is_empty() {
        return None;
    }
    if let Some(hex) = value.strip_prefix('#') {
        return hex_color(hex);
    }
    let lower = value.to_ascii_lowercase();
    if let Some(args) = rgb_arguments(&lower) {
        return rgb_function(args);
    }
    named(&lower)
}

/// 【贴图】【十六进制】按 3/4/6/8 位十六进制解析颜色，返回带默认不透明度的 RGBA
fn hex_color(hex: &str) -> Option<Rgba> {
    if !hex.is_ascii() {
        return None;
    }
    let nibble = |byte: u8| (byte as char).to_digit(16).map(|value| value as u8);
    let pair = |chars: &[u8], at: usize| {
        let high = nibble(*chars.get(at)?)?;
        let low = nibble(*chars.get(at + 1)?)?;
        Some(high * 16 + low)
    };
    let chars = hex.as_bytes();
    match chars.len() {
        3 => {
            let (r, g, b) = (nibble(chars[0])?, nibble(chars[1])?, nibble(chars[2])?);
            Some(Rgba(r * 17, g * 17, b * 17, 255))
        }
        4 => {
            let (r, g, b, a) = (
                nibble(chars[0])?,
                nibble(chars[1])?,
                nibble(chars[2])?,
                nibble(chars[3])?,
            );
            Some(Rgba(r * 17, g * 17, b * 17, a * 17))
        }
        6 => Some(Rgba(pair(chars, 0)?, pair(chars, 2)?, pair(chars, 4)?, 255)),
        8 => Some(Rgba(
            pair(chars, 0)?,
            pair(chars, 2)?,
            pair(chars, 4)?,
            pair(chars, 6)?,
        )),
        _ => None,
    }
}

/// 【贴图】【函数颜色】返回 rgb()/rgba() 括号内的参数文本，非函数语法返回空值
fn rgb_arguments(lower: &str) -> Option<&str> {
    lower
        .strip_prefix("rgb(")
        .and_then(|rest| rest.strip_suffix(')'))
        .or_else(|| {
            lower
                .strip_prefix("rgba(")
                .and_then(|rest| rest.strip_suffix(')'))
        })
}

/// 【贴图】【函数分量】按逗号、空格或斜杠拆分 rgb 参数，解析数字与百分比分量，返回 RGBA
fn rgb_function(args: &str) -> Option<Rgba> {
    let normalized = args.replace('/', " ");
    let parts: Vec<&str> = normalized
        .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if !(3..=4).contains(&parts.len()) {
        return None;
    }
    let channel = |part: &str| -> Option<u8> {
        let (value, percent) = parse_number(part)?;
        if !value.is_finite() {
            return None;
        }
        let scaled = if percent {
            value / 100.0 * 255.0
        } else {
            value
        };
        Some(scaled.round().clamp(0.0, 255.0) as u8)
    };
    let alpha = |part: &str| -> Option<u8> {
        let (value, percent) = parse_number(part)?;
        if !value.is_finite() {
            return None;
        }
        let unit = if percent {
            value / 100.0
        } else if value > 1.0 {
            value / 255.0
        } else {
            value
        };
        Some((unit * 255.0).round().clamp(0.0, 255.0) as u8)
    };
    Some(Rgba(
        channel(parts[0])?,
        channel(parts[1])?,
        channel(parts[2])?,
        parts.get(3).map_or(Some(255), |part| alpha(part))?,
    ))
}

/// 【贴图】【数值解析】按 value 解析百分比或浮点数，返回数值与是否为百分比
fn parse_number(value: &str) -> Option<(f32, bool)> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        return Some((percent.trim().parse().ok()?, true));
    }
    Some((value.parse().ok()?, false))
}

/// 【贴图】【颜色名】按 lower 查询常见 CSS 颜色名，返回 RGBA；透明与 currentcolor 返回空值
fn named(lower: &str) -> Option<Rgba> {
    let rgb = |r, g, b| Some(Rgba(r, g, b, 255));
    match lower {
        "black" => rgb(0, 0, 0),
        "white" => rgb(255, 255, 255),
        "red" => rgb(255, 0, 0),
        "green" => rgb(0, 128, 0),
        "blue" => rgb(0, 0, 255),
        "yellow" => rgb(255, 255, 0),
        "orange" => rgb(255, 165, 0),
        "purple" => rgb(128, 0, 128),
        "gray" | "grey" => rgb(128, 128, 128),
        "silver" => rgb(192, 192, 192),
        "maroon" => rgb(128, 0, 0),
        "navy" => rgb(0, 0, 128),
        "teal" => rgb(0, 128, 128),
        "olive" => rgb(128, 128, 0),
        "lime" => rgb(0, 255, 0),
        "aqua" | "cyan" => rgb(0, 255, 255),
        "fuchsia" | "magenta" => rgb(255, 0, 255),
        _ => None,
    }
}

/// 【贴图】【优先级去除】按 value 去掉首尾空白及尾随的大小写不敏感 !important，返回有效值
fn strip_important(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed
        .to_ascii_lowercase()
        .strip_suffix("!important")
        .is_none()
    {
        return trimmed;
    }
    trimmed
        .get(..trimmed.len().saturating_sub("!important".len()))
        .map_or(trimmed, str::trim_end)
}

/// 【贴图】【声明拆分】按 css 拆分分号声明并返回属性与值，忽略空属性
pub(super) fn declarations(css: &str) -> Vec<(&str, &str)> {
    css.split(';')
        .filter_map(|declaration| {
            let (property, value) = declaration.split_once(':')?;
            let property = property.trim();
            if property.is_empty() {
                return None;
            }
            Some((property, strip_important(value)))
        })
        .collect()
}

/// 【贴图】【字号解析】按 value 与 base 解析 px/pt/em/rem/% 字号，返回限制在 6..=96 的像素值
pub(super) fn font_size(value: &str, base: f32) -> Option<f32> {
    let lower = value.trim().to_ascii_lowercase();
    let split = lower
        .char_indices()
        .find(|(_, ch)| !(ch.is_ascii_digit() || *ch == '.' || *ch == '-' || *ch == '+'))
        .map_or(lower.len(), |(index, _)| index);
    if split == 0 {
        return None;
    }
    let number: f32 = lower[..split].parse().ok()?;
    if !number.is_finite() {
        return None;
    }
    let pixels = match &lower[split..] {
        "" | "px" => number,
        "pt" => number * (4.0 / 3.0),
        "em" => number * base,
        "rem" => number * 16.0,
        "%" => number / 100.0 * base,
        _ => return None,
    };
    if !pixels.is_finite() {
        return None;
    }
    Some(pixels.clamp(6.0, 96.0))
}

/// 【贴图】【长度解析】按 value 解析 px/pt/in/cm/mm 为像素长度，拒绝负数与非有限值并限制上限
pub(super) fn length_px(value: &str) -> Option<f32> {
    let lower = value.trim().to_ascii_lowercase();
    let split = lower
        .char_indices()
        .find(|(_, ch)| !(ch.is_ascii_digit() || *ch == '.' || *ch == '-' || *ch == '+'))
        .map_or(lower.len(), |(index, _)| index);
    if split == 0 {
        return None;
    }
    let number: f32 = lower[..split].parse().ok()?;
    if !number.is_finite() || number < 0.0 {
        return None;
    }
    let pixels = match &lower[split..] {
        "px" => number,
        "pt" => number * (4.0 / 3.0),
        "in" => number * 96.0,
        "cm" => number * (96.0 / 2.54),
        "mm" => number * (96.0 / 25.4),
        _ => return None,
    };
    if !pixels.is_finite() {
        return None;
    }
    Some(pixels.clamp(0.0, 100_000.0))
}

/// 【贴图】【百分比解析】按 value 解析 0..=100000 的百分比数值，非百分比语法返回空值
pub(super) fn percent(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    let number: f32 = trimmed.strip_suffix('%')?.trim().parse().ok()?;
    if !number.is_finite() || number < 0.0 {
        return None;
    }
    Some(number.clamp(0.0, 100_000.0))
}

/// 【贴图】【字重解析】按 value 解析 bold/bolder/数值/常规关键字，返回是否加粗
pub(super) fn bold(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "bold" | "bolder" => Some(true),
        "normal" | "lighter" => Some(false),
        other => Some(other.parse::<u32>().ok()? >= 600),
    }
}

/// 【贴图】【斜体解析】按 value 解析 italic/oblique/normal，返回是否倾斜
pub(super) fn italic(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "italic" | "oblique" => Some(true),
        "normal" => Some(false),
        _ => None,
    }
}

/// 【贴图】【衬线字体】按 value 的逗号分隔字体族识别已知衬线族，返回是否使用衬线字体
pub(super) fn serif(value: &str) -> bool {
    value.split(',').any(|token| {
        let token = token
            .trim()
            .trim_matches(|ch| ch == '\'' || ch == '"')
            .trim()
            .to_ascii_lowercase();
        matches!(
            token.as_str(),
            "serif"
                | "times"
                | "times new roman"
                | "georgia"
                | "garamond"
                | "palatino"
                | "book antiqua"
                | "cambria"
        )
    })
}

/// 【贴图】【行高解析】按 value 与 size 解析无单位、百分比、em、px、pt 行高，返回 0.8..=3.0 的倍数
pub(super) fn line_height(value: &str, size: f32) -> Option<f32> {
    if !size.is_finite() || size <= 0.0 {
        return None;
    }
    let lower = value.trim().to_ascii_lowercase();
    if lower.is_empty() || lower == "normal" {
        return None;
    }
    let split = lower
        .char_indices()
        .find(|(_, ch)| !(ch.is_ascii_digit() || *ch == '.' || *ch == '-' || *ch == '+'))
        .map_or(lower.len(), |(index, _)| index);
    if split == 0 {
        return None;
    }
    let number: f32 = lower[..split].parse().ok()?;
    if !number.is_finite() {
        return None;
    }
    let multiplier = match &lower[split..] {
        "" => number,
        "%" => number / 100.0,
        "em" | "rem" => number,
        "px" => number / size,
        "pt" => number * (4.0 / 3.0) / size,
        _ => return None,
    };
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return None;
    }
    Some(multiplier.clamp(0.8, 3.0))
}

/// 【贴图】【等宽字体】按 value 识别 monospace/courier/consolas/menlo 等字体族，返回是否使用等宽字体
pub(super) fn monospace(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("mono")
        || lower.contains("courier")
        || lower.contains("consolas")
        || lower.contains("menlo")
}
