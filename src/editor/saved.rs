// saved responsible for saving and reading user's options
// such as font width, stroke width, color choice, and etc

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use log::info;
use serde::{Deserialize, Serialize};
use tiny_skia::Color;

use crate::theme::Rgba;
use crate::types::{StrokeGroup, ToolSettings};
use crate::ui::color_popover::color_to_hex_string;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct File {
    color: Option<String>,
    /// one width for every tool, how it was stored before the per-tool ones, only read
    stroke_width: Option<f32>,
    font_size: Option<f32>,
    font_size_unit: Option<String>,
    text_family: Option<String>,
    text_background: Option<String>,
    strength: Option<f32>,
    filter_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pixelate: Option<bool>,
    bold: Option<bool>,
    italic: Option<bool>,
    outline: Option<bool>,
    outline_color: Option<String>,
    fill: Option<bool>,
    marker_shape: Option<String>,
    marker_filled: Option<bool>,
    ruler_mode: Option<String>,
    magnifier_shape: Option<String>,
    magnifier_zoom: Option<f32>,
    number_style: Option<String>,
    arrow_style: Option<String>,
    arrow_line_style: Option<String>,
    line_style: Option<String>,
    magnifier: Option<bool>,
    recent_colors: Vec<String>,
    stroke_widths: BTreeMap<String, f32>,
}

impl File {
    /// 【文字偏好】【字号兼容】把已保存点字号或旧像素字号转换为八至三百点整数；未知单位与非有限值返回空值
    fn resolved_points(&self) -> Option<f32> {
        let size = self.font_size.filter(|size| size.is_finite())?;
        let points = match self.font_size_unit.as_deref() {
            Some("pt") => size,
            Some("px") | None => size * 72.0 / 96.0,
            Some(unit) => {
                info!("【文字偏好】【字号兼容】未知字号单位 {unit:?}");
                return None;
            }
        };
        Some(crate::features::text::geometry::points(points))
    }

    /// 【滤镜】【偏好兼容】返回明确模式或旧像素化开关对应模式；未知模式只忽略该字段
    fn resolved_filter_mode(&self) -> Option<crate::features::filter::FilterMode> {
        self.filter_mode
            .as_deref()
            .and_then(parse_filter_mode)
            .or_else(|| {
                self.pixelate
                    .map(crate::features::filter::FilterMode::from_pixelate)
            })
    }
}

#[derive(Debug, Default)]
pub struct Saved {
    pub color: Option<Rgba>,
    stroke_width: Option<f32>,
    stroke_widths: BTreeMap<String, f32>,
    pub font_size: Option<f32>,
    pub text_family: Option<String>,
    pub text_background: Option<Rgba>,
    pub strength: Option<f32>,
    pub filter_mode: Option<crate::features::filter::FilterMode>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub outline: Option<bool>,
    pub outline_color: Option<Rgba>,
    pub fill: Option<bool>,
    pub marker_shape: Option<crate::types::MarkerShape>,
    pub marker_filled: Option<bool>,
    pub ruler_mode: Option<crate::features::ruler::RulerMode>,
    pub magnifier_shape: Option<crate::features::magnifier::MagnifierShape>,
    pub magnifier_zoom: Option<f32>,
    pub number_style: Option<crate::features::number::NumberStyle>,
    pub arrow_style: Option<crate::features::arrow::ArrowStyle>,
    pub arrow_line_style: Option<crate::features::line::LineStyle>,
    pub line_style: Option<crate::features::line::LineStyle>,
    pub magnifier: Option<bool>,
    pub recent_colors: Vec<Rgba>,
}

fn path() -> Option<PathBuf> {
    Some(dirs::state_dir()?.join("mark-shot").join("tools.toml"))
}

fn parse_color(s: &str) -> Option<Rgba> {
    let color = Rgba::parse_hex(s);
    if color.is_none() {
        info!("tools state: invalid color {s:?}, ignoring it");
    }
    color
}

fn hex(color: Color) -> String {
    format!("#{}", color_to_hex_string(color))
}

/// 【工具偏好】【状态读取】读取已保存工具参数；返回校验后的偏好，未知样式单独忽略
fn load() -> Saved {
    let Some(path) = path() else {
        return Saved::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Saved::default();
    };
    let file: File = match toml::from_str(&text) {
        Ok(file) => file,
        Err(e) => {
            info!("tools state: can't read {}: {e}", path.display());
            return Saved::default();
        }
    };

    let filter_mode = file.resolved_filter_mode();
    let font_size = file.resolved_points();
    let mut recent_colors = Vec::new();
    for color in file.recent_colors.iter().filter_map(|s| parse_color(s)) {
        if !recent_colors.contains(&color) {
            recent_colors.push(color);
        }
    }

    Saved {
        color: file.color.as_deref().and_then(parse_color),
        stroke_width: file.stroke_width.filter(|w| w.is_finite()),
        stroke_widths: file
            .stroke_widths
            .into_iter()
            .filter(|(_, w)| w.is_finite())
            .collect(),
        font_size,
        text_family: file.text_family.filter(|family| !family.trim().is_empty()),
        text_background: file.text_background.as_deref().and_then(parse_color),
        strength: file.strength.filter(|s| s.is_finite()),
        filter_mode,
        bold: file.bold,
        italic: file.italic,
        outline: file.outline,
        outline_color: file.outline_color.as_deref().and_then(parse_color),
        fill: file.fill,
        marker_shape: file.marker_shape.as_deref().and_then(parse_marker_shape),
        marker_filled: file.marker_filled,
        ruler_mode: file.ruler_mode.as_deref().and_then(parse_ruler_mode),
        magnifier_shape: file
            .magnifier_shape
            .as_deref()
            .and_then(parse_magnifier_shape),
        magnifier_zoom: file.magnifier_zoom.filter(|value| value.is_finite()),
        number_style: file.number_style.as_deref().and_then(parse_number_style),
        arrow_style: file.arrow_style.as_deref().and_then(parse_arrow_style),
        arrow_line_style: file.arrow_line_style.as_deref().and_then(parse_line_style),
        line_style: file.line_style.as_deref().and_then(parse_line_style),
        magnifier: file.magnifier,
        recent_colors,
    }
}

impl Saved {
    pub fn stroke(&self, group: StrokeGroup) -> Option<f32> {
        let width = self.stroke_widths.get(group.key()).copied().or_else(|| {
            if group == StrokeGroup::Highlighter {
                self.stroke_widths.get("freehand").copied()
            } else {
                None
            }
        });
        if group == StrokeGroup::Number {
            width
        } else {
            width.or(self.stroke_width)
        }
    }
}

pub fn get() -> &'static Saved {
    static SAVED: OnceLock<Saved> = OnceLock::new();
    SAVED.get_or_init(load)
}

/// 【标注】【形状设置】将 value 转为稳定形状类型；未知名称单独忽略，不丢弃其他工具设置
fn parse_marker_shape(value: &str) -> Option<crate::types::MarkerShape> {
    match value.parse() {
        Ok(shape) => Some(shape),
        Err(error) => {
            info!("【标注】【形状设置】{error}");
            None
        }
    }
}

/// 【尺子】【模式设置】将 value 转为稳定测量模式；未知名称单独忽略，返回空值
fn parse_ruler_mode(value: &str) -> Option<crate::features::ruler::RulerMode> {
    let mode = crate::features::ruler::RulerMode::parse(value);
    if mode.is_none() {
        info!("【尺子】【模式设置】未知模式 {value:?}");
    }
    mode
}

/// 【放大镜】【形状设置】将 value 转为稳定框形；未知名称单独忽略，返回空值
fn parse_magnifier_shape(value: &str) -> Option<crate::features::magnifier::MagnifierShape> {
    let shape = crate::features::magnifier::MagnifierShape::parse(value);
    if shape.is_none() {
        info!("【放大镜】【形状设置】未知形状 {value:?}");
    }
    shape
}

/// 【序号】【样式偏好】将 value 转为序号样式；未知值单独忽略并记录原因，返回样式或空值
fn parse_number_style(value: &str) -> Option<crate::features::number::NumberStyle> {
    let style = crate::features::number::NumberStyle::parse(value);
    if style.is_none() {
        info!("【序号】【样式偏好】未知样式 {value:?}");
    }
    style
}

/// 【箭头】【头部偏好】将 value 转为头部样式；未知值单独忽略并记录原因，返回样式或空值
fn parse_arrow_style(value: &str) -> Option<crate::features::arrow::ArrowStyle> {
    let style = crate::features::arrow::ArrowStyle::parse(value);
    if style.is_none() {
        info!("【箭头】【头部偏好】未知样式 {value:?}");
    }
    style
}

/// 【线条】【线型偏好】将 value 转为独立线型；未知值单独忽略并记录原因，返回线型或空值
fn parse_line_style(value: &str) -> Option<crate::features::line::LineStyle> {
    let style = crate::features::line::LineStyle::parse(value);
    if style.is_none() {
        info!("【线条】【线型偏好】未知线型 {value:?}");
    }
    style
}

/// 【滤镜】【模式偏好】把 value 转为稳定模式；未知值单独忽略，不丢弃强度或其他工具偏好
fn parse_filter_mode(value: &str) -> Option<crate::features::filter::FilterMode> {
    let mode = crate::features::filter::FilterMode::parse(value);
    if mode.is_none() {
        info!("【滤镜】【模式偏好】未知模式 {value:?}");
    }
    mode
}

/// 【工具偏好】【状态保存】保存 settings、history 与 magnifier；initial_history 用于判断变化，无返回值，不保存会话骨架
pub fn save(
    settings: &ToolSettings,
    history: &[Color],
    initial_history: &[Color],
    magnifier: bool,
) {
    let magnifier_unchanged = crate::config::get()
        .startup
        .selection_loupe
        .or(get().magnifier)
        .unwrap_or(true)
        == magnifier;
    if *settings == ToolSettings::default() && history == initial_history && magnifier_unchanged {
        return;
    }
    let Some(path) = path() else {
        return;
    };

    let file = File {
        color: Some(hex(settings.color)),
        stroke_width: None,
        stroke_widths: StrokeGroup::ALL
            .iter()
            .map(|&g| (g.key().to_string(), settings.stroke(g)))
            .collect(),
        font_size: Some(settings.font_size),
        font_size_unit: Some("pt".into()),
        text_family: Some(settings.text_family.clone()),
        text_background: Some(hex(settings.text_background)),
        strength: Some(settings.strength),
        filter_mode: Some(settings.filter_mode.key().to_string()),
        pixelate: None,
        bold: Some(settings.bold),
        italic: Some(settings.italic),
        outline: Some(settings.outline),
        outline_color: Some(hex(settings.outline_color)),
        fill: Some(settings.fill),
        marker_shape: Some(settings.marker_shape.key().to_string()),
        marker_filled: Some(settings.marker_filled),
        ruler_mode: Some(settings.ruler_mode.key().to_string()),
        magnifier_shape: Some(settings.magnifier_shape.key().to_string()),
        magnifier_zoom: Some(settings.magnifier_zoom),
        number_style: Some(settings.number_style.key().to_string()),
        arrow_style: Some(settings.arrow_style.key().to_string()),
        arrow_line_style: Some(settings.arrow_line_style.key().to_string()),
        line_style: Some(settings.line_style.key().to_string()),
        magnifier: Some(magnifier),
        recent_colors: history.iter().map(|&c| hex(c)).collect(),
    };
    let text = match toml::to_string(&file) {
        Ok(text) => text,
        Err(e) => {
            info!("tools state: can't serialize: {e}");
            return;
        }
    };

    // using temp file
    // small edge case bug fix: adding pid to name, so executing capture two times, wouldn't
    // write into the same file
    let tmp = path.with_extension(format!("toml.{}.tmp", std::process::id()));
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&tmp, text))
        .and_then(|()| std::fs::rename(&tmp, &path));
    if let Err(e) = written {
        info!("tools state: can't write {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    /// 【荧光笔偏好】【旧值兼容】独立分组优先，缺失时读取旧画笔分组，设置一项不覆盖另一项
    #[test]
    fn highlighter_width_keeps_explicit_values_and_legacy_freehand_fallback() {
        let mut saved = Saved {
            stroke_widths: BTreeMap::from([("freehand".into(), 5.0)]),
            ..Saved::default()
        };
        assert_eq!(saved.stroke(StrokeGroup::Highlighter), Some(5.0));
        saved.stroke_widths.insert("highlighter".into(), 48.0);
        assert_eq!(saved.stroke(StrokeGroup::Highlighter), Some(48.0));
        assert_eq!(saved.stroke(StrokeGroup::Freehand), Some(5.0));
    }
    use super::*;

    /// 【文字偏好】【单位迁移】旧像素字号、新点字号与边界转换一致，未知单位不影响字体家族和背景
    #[test]
    fn text_sizes_migrate_from_pixels_and_keep_point_units() {
        for (input, expected) in [
            ("font_size = 32.0", Some(24.0)),
            ("font_size = 32.0\nfont_size_unit = 'px'", Some(24.0)),
            ("font_size = 20.0\nfont_size_unit = 'pt'", Some(20.0)),
            ("font_size = 4.0\nfont_size_unit = 'pt'", Some(8.0)),
            ("font_size = 400.0\nfont_size_unit = 'pt'", Some(300.0)),
            ("font_size = nan\nfont_size_unit = 'pt'", None),
            ("font_size = 20.0\nfont_size_unit = 'unknown'", None),
        ] {
            let file: File = toml::from_str(input).unwrap();
            assert_eq!(file.resolved_points(), expected);
        }
        let file: File = toml::from_str("font_size = 20.0\nfont_size_unit = 'pt'\ntext_family = 'Inter'\ntext_background = '#12345678'").unwrap();
        let text = toml::to_string(&file).unwrap();
        let restored: File = toml::from_str(&text).unwrap();
        assert_eq!(restored.resolved_points(), Some(20.0));
        assert_eq!(restored.text_family.as_deref(), Some("Inter"));
        assert_eq!(restored.text_background, file.text_background);
    }

    /// 【滤镜测试】【偏好兼容】旧开关保持含义，新模式优先，未知模式不丢弃强度与颜色
    #[test]
    fn saved_filter_modes_accept_old_flags_and_preserve_unrelated_values() {
        use crate::features::filter::FilterMode;
        let old: File = toml::from_str("pixelate = false\nstrength = 23.0\n").unwrap();
        assert_eq!(old.resolved_filter_mode(), Some(FilterMode::Blur));
        let new: File = toml::from_str("pixelate = false\nfilter_mode = \"invert\"\n").unwrap();
        assert_eq!(new.resolved_filter_mode(), Some(FilterMode::Invert));
        let invalid: File =
            toml::from_str("filter_mode = \"other\"\nstrength = 31.0\ncolor = \"#112233\"\n")
                .unwrap();
        assert_eq!(invalid.resolved_filter_mode(), None);
        assert_eq!(invalid.strength, Some(31.0));
        assert_eq!(
            invalid.color.as_deref().and_then(parse_color),
            Some(Rgba(17, 34, 51, 255))
        );
    }

    /// 【滤镜测试】【偏好往返】保存五模式与强度，不保存旧开关、旋转、草稿或原图像素
    #[test]
    fn filter_preferences_round_trip_without_saving_geometry() {
        for mode in crate::features::filter::FilterMode::ALL {
            let file = File {
                filter_mode: Some(mode.key().to_string()),
                strength: Some(31.0),
                ..File::default()
            };
            let text = toml::to_string(&file).unwrap();
            let other: File = toml::from_str(&text).unwrap();
            assert_eq!(other.resolved_filter_mode(), Some(mode));
            assert_eq!(other.strength, Some(31.0));
            for excluded in [
                "pixelate =",
                "rotation",
                "pending",
                "filter_cache",
                "pixels",
            ] {
                assert!(!text.contains(excluded));
            }
        }
    }

    /// 【线条偏好】【稳定标识】头部和线型读取名称与原版编号，未知头部不丢弃另一线型或颜色
    #[test]
    fn saved_head_and_line_names_reject_unknown_fields_individually() {
        use crate::features::{arrow::ArrowStyle, line::LineStyle};

        assert_eq!(
            parse_arrow_style("BIDIRECTIONAL_KDE"),
            Some(ArrowStyle::BidirectionalKde)
        );
        assert_eq!(parse_arrow_style("3"), Some(ArrowStyle::BidirectionalKde));
        assert_eq!(parse_line_style("DASHED"), Some(LineStyle::Dashed));
        assert_eq!(parse_line_style("2"), Some(LineStyle::Dotted));
        assert_eq!(parse_arrow_style("missing"), None);
        assert_eq!(parse_line_style("missing"), None);
        let file: File = toml::from_str("color = \"#112233\"\narrow_style = \"missing\"\narrow_line_style = \"dashed\"\nline_style = \"dotted\"\n").unwrap();
        assert_eq!(
            file.arrow_style.as_deref().and_then(parse_arrow_style),
            None
        );
        assert_eq!(
            file.arrow_line_style.as_deref().and_then(parse_line_style),
            Some(LineStyle::Dashed)
        );
        assert_eq!(
            file.line_style.as_deref().and_then(parse_line_style),
            Some(LineStyle::Dotted)
        );
        assert_eq!(
            file.color.as_deref().and_then(parse_color),
            Some(Rgba(17, 34, 51, 255))
        );
    }

    /// 【线条偏好】【独立往返】偏好往返保存三种独立样式和笔宽，不保存控制点、草稿或选择索引
    #[test]
    fn line_preferences_round_trip_without_storing_session_geometry() {
        let file = File {
            arrow_style: Some("bidirectional_fletched".to_string()),
            arrow_line_style: Some("dashed".to_string()),
            line_style: Some("dotted".to_string()),
            stroke_widths: BTreeMap::from([
                ("arrow".to_string(), 72.0),
                ("line".to_string(), 7.0),
                ("numbered".to_string(), 11.0),
            ]),
            ..File::default()
        };
        let text = toml::to_string(&file).unwrap();
        let after: File = toml::from_str(&text).unwrap();
        assert_eq!(after.arrow_style, file.arrow_style);
        assert_eq!(after.arrow_line_style, file.arrow_line_style);
        assert_eq!(after.line_style, file.line_style);
        assert_eq!(after.stroke_widths, file.stroke_widths);
        for excluded in ["controls", "selected_point", "pending", "line_session"] {
            assert!(!text.contains(excluded));
        }
    }

    /// 【序号偏好】【独立笔宽】旧全局笔宽不覆盖新增序号默认值，明确的序号分组笔宽仍可读取
    #[test]
    fn new_number_width_never_inherits_the_legacy_global_width() {
        let mut saved = Saved {
            stroke_width: Some(12.0),
            ..Saved::default()
        };
        assert_eq!(saved.stroke(StrokeGroup::Number), None);
        assert_eq!(saved.stroke(StrokeGroup::Numbered), Some(12.0));
        saved.stroke_widths.insert("number".to_string(), 7.0);
        assert_eq!(saved.stroke(StrokeGroup::Number), Some(7.0));
    }

    /// 【序号偏好】【样式读取】合法样式保留大小写兼容，未知样式只返回空值，不误用阿拉伯数字覆盖默认设置
    #[test]
    fn number_styles_are_parsed_without_overwriting_other_preferences() {
        assert_eq!(
            parse_number_style("UPPER_ROMAN"),
            Some(crate::features::number::NumberStyle::UpperRoman)
        );
        assert_eq!(
            parse_number_style("chinese"),
            Some(crate::features::number::NumberStyle::Chinese)
        );
        assert_eq!(parse_number_style("missing"), None);
        let file: File = toml::from_str("color = \"#112233\"\nnumber_style = \"missing\"\n[stroke_widths]\nnumber = 8.0\nnumbered = 11.0\n").unwrap();
        assert_eq!(
            file.color.as_deref().and_then(parse_color),
            Some(Rgba(17, 34, 51, 255))
        );
        assert_eq!(file.stroke_widths["number"], 8.0);
        assert_eq!(file.stroke_widths["numbered"], 11.0);
    }

    /// 【序号偏好】【会话隔离】工具偏好保存样式和独立笔宽，不包含会话编号或撤销快照
    #[test]
    fn preference_serialization_never_contains_the_number_sequence() {
        let file = File {
            number_style: Some("heavenly_stem".to_string()),
            stroke_widths: BTreeMap::from([("number".to_string(), 6.0)]),
            ..File::default()
        };
        let text = toml::to_string(&file).unwrap();
        assert!(text.contains("heavenly_stem"));
        assert!(text.contains("number = 6.0"));
        assert!(!text.contains("number_sequence") && !text.contains("next_number"));
    }
}
