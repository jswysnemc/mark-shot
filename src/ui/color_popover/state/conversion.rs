//! 【颜色面板】【颜色转换】处理颜色通道、HSV 和十六进制输入

use super::*;

/// h: 0.0..=360.0, s/v: 0.0..=1.0
pub fn hsv_to_color(h: f32, s: f32, v: f32) -> Color {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::from_rgba(r + m, g + m, b + m, 1.0).unwrap()
}

pub fn color_to_hsv(c: Color) -> (f32, f32, f32) {
    let r = c.red();
    let g = c.green();
    let b = c.blue();
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta <= f32::EPSILON {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let s = if max <= f32::EPSILON {
        0.0
    } else {
        delta / max
    };
    let v = max;

    (h.rem_euclid(360.0), s, v)
}

pub(super) fn color_bytes(c: Color) -> (u8, u8, u8, u8) {
    let c8 = c.to_color_u8();
    (c8.red(), c8.green(), c8.blue(), c8.alpha())
}

pub fn color_channel_u8(c: Color, field: ColorField) -> u8 {
    let c8 = c.to_color_u8();
    match field {
        ColorField::R => c8.red(),
        ColorField::G => c8.green(),
        ColorField::B => c8.blue(),
        ColorField::A => c8.alpha(),
        ColorField::Hex => 0,
    }
}

pub fn color_with_channel(c: Color, field: ColorField, value: u8) -> Color {
    let c8 = c.to_color_u8();
    let (r, g, b, a) = (c8.red(), c8.green(), c8.blue(), c8.alpha());
    let (r, g, b, a) = match field {
        ColorField::R => (value, g, b, a),
        ColorField::G => (r, value, b, a),
        ColorField::B => (r, g, value, a),
        ColorField::A => (r, g, b, value),
        ColorField::Hex => (r, g, b, a),
    };
    Color::from_rgba8(r, g, b, a)
}

pub fn color_to_hex_string(c: Color) -> String {
    let c8 = c.to_color_u8();
    if c8.alpha() == 255 {
        format!("{:02X}{:02X}{:02X}", c8.red(), c8.green(), c8.blue())
    } else {
        format!(
            "{:02X}{:02X}{:02X}{:02X}",
            c8.red(),
            c8.green(),
            c8.blue(),
            c8.alpha()
        )
    }
}

pub fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    if !s.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let (r, g, b, a) = match s.len() {
        3 => {
            let r = u8::from_str_radix(&s[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&s[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&s[2..3], 16).ok()? * 17;
            (r, g, b, 255)
        }
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            (r, g, b, 255)
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            (r, g, b, a)
        }
        _ => return None,
    };
    Some(Color::from_rgba8(r, g, b, a))
}

pub fn step_hex_text(text: &str, steps: i32) -> String {
    let trimmed = text.trim().trim_start_matches('#');
    let width = if trimmed.len() == 8 { 8usize } else { 6usize };
    let current = u32::from_str_radix(trimmed, 16).unwrap_or(0);
    let max: u32 = if width == 8 { 0xFFFF_FFFF } else { 0x00FF_FFFF };

    let new_value = if steps >= 0 {
        current.saturating_add(steps as u32).min(max)
    } else {
        current.saturating_sub(steps.unsigned_abs())
    };

    format!("{:0width$X}", new_value, width = width)
}
