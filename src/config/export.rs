//! 【图像导出】【外框配置】兼容原版布尔开关、对象字段及别名，导出时读取最新配置

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use shot_image::export::FrameOptions;
use std::{io, path::Path};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Export {
    #[serde(alias = "imageFrame")]
    pub image_frame: ImageFrame,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct ImageFrame {
    pub enabled: bool,
    pub padding: i32,
    pub corner_radius: f64,
    pub shadow_radius: i32,
    pub shadow_offset_y: i32,
    pub shadow_opacity: f64,
}

impl Default for ImageFrame {
    /// 【图像导出】【默认外框】返回原版未启用外框时的完整参数，无参数
    fn default() -> Self {
        Self {
            enabled: false,
            padding: 112,
            corner_radius: 18.0,
            shadow_radius: 72,
            shadow_offset_y: 28,
            shadow_opacity: 0.32,
        }
    }
}

/// 【图像导出】【开关转换】按原版规则解析 value 的布尔、数字或字符串，返回识别结果
fn boolean(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(value) => value.as_f64().map(|value| value.abs() > 1e-12),
        Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" | "enabled" => Some(true),
            "0" | "false" | "no" | "off" | "disabled" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// 【图像导出】【数值转换】从 value 解析有限实数，返回数字或字符串对应的值
fn real(value: &Value) -> Option<f64> {
    match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
    .filter(|value: &f64| value.is_finite())
}

/// 【图像导出】【整数转换】解析 value 的原版整数规则，非整数数字返回零，字符串溢出返回空值
fn integer(value: &Value) -> Option<i32> {
    match value {
        Value::Number(_) => Some(
            real(value)
                .filter(|value| {
                    value.fract() == 0.0
                        && *value >= f64::from(i32::MIN)
                        && *value <= f64::from(i32::MAX)
                })
                .map_or(0, |value| value as i32),
        ),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
}

/// 【图像导出】【字段优先级】从 value 按 keys 顺序返回首个已定义字段，空值也阻止后续别名覆盖
fn first<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key))
}

impl ImageFrame {
    /// 【图像导出】【兼容解析】从 value 解析原版 imageFrame 布尔或对象，返回夹紧范围的参数
    pub fn from_value(value: &Value) -> Self {
        let mut frame = Self::default();
        if let Some(enabled) = boolean(value) {
            frame.enabled = enabled;
            return frame;
        }
        if !value.is_object() {
            return frame;
        }
        frame.enabled = first(value, &["enabled", "show", "visible"])
            .and_then(boolean)
            .unwrap_or(frame.enabled);
        // 1. 【图像导出】【整数范围】原生蛇形键优先，原版键和旧别名使用相同转换与范围
        for (target, keys, maximum) in [
            (&mut frame.padding, &["padding", "margin"][..], 256),
            (
                &mut frame.shadow_radius,
                &["shadow_radius", "shadowRadius", "shadowBlur"][..],
                128,
            ),
            (
                &mut frame.shadow_offset_y,
                &["shadow_offset_y", "shadowOffsetY", "shadowYOffset"][..],
                128,
            ),
        ] {
            if let Some(number) = first(value, keys).and_then(integer) {
                *target = number.clamp(0, maximum);
            }
        }
        // 2. 【图像导出】【实数范围】非法或非有限实数沿用默认值，不传入像素合成器
        for (target, keys, maximum) in [
            (
                &mut frame.corner_radius,
                &["corner_radius", "cornerRadius", "radius"][..],
                128.0,
            ),
            (
                &mut frame.shadow_opacity,
                &["shadow_opacity", "shadowOpacity", "shadowAlpha"][..],
                1.0,
            ),
        ] {
            if let Some(number) = first(value, keys).and_then(real) {
                *target = number.clamp(0.0, maximum);
            }
        }
        frame
    }

    /// 【图像导出】【核心参数】返回当前 frame 对应的物理像素合成参数，阴影颜色与原版配置保持黑色
    pub fn options(self) -> FrameOptions {
        FrameOptions {
            enabled: self.enabled,
            padding: self.padding,
            corner_radius: self.corner_radius,
            shadow_radius: self.shadow_radius,
            shadow_offset_y: self.shadow_offset_y,
            shadow_opacity: self.shadow_opacity,
            shadow_color: 0xff000000,
        }
    }
}

impl<'de> Deserialize<'de> for ImageFrame {
    /// 【图像导出】【原生解码】从 deserializer 读取完整外框值，返回兼容原版类型的配置
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Value::deserialize(deserializer).map(|value| Self::from_value(&value))
    }
}

/// 【图像导出】【独立读取】从 path 安全读取 TOML 或 JSON 外框配置，返回参数，不修改文件
pub(crate) fn read_at(path: &Path) -> io::Result<ImageFrame> {
    let text = super::persistence::read(path)?;
    let value = super::document::root(path, &text)?;
    let export = &value["export"];
    Ok(ImageFrame::from_value(
        first(export, &["image_frame", "imageFrame"]).unwrap_or(&Value::Null),
    ))
}

/// 【图像导出】【当前配置】返回命令行指定或默认文件中的最新外框，文件缺失沿用启动配置，其他错误原样返回
pub fn current() -> io::Result<ImageFrame> {
    let path = super::OVERRIDE_PATH
        .get()
        .cloned()
        .or_else(super::loading::config_path);
    match path.map(|path| read_at(&path)).transpose() {
        Ok(Some(frame)) => Ok(frame),
        Ok(None) => Ok(super::get().export.image_frame),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(super::get().export.image_frame)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "tests/export.rs"]
mod tests;
