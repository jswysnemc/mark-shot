//! 【niri 窗口识别】【面板独占区】原版 DMS、Waybar 及显式边缘覆盖

use super::{layout, model::Layer};
use serde_json::Value;

pub(super) struct Settings {
    pub(super) dms: Value,
    pub(super) waybar: Value,
    pub(super) edge: Option<String>,
    pub(super) size: Option<f64>,
}

/// 【niri 窗口识别】【设置读取】无参数，有界读取 DMS、Waybar 和面板覆盖配置，返回一次性设置快照
pub(super) fn load() -> Settings {
    let home = layout::config_home();
    let mut budget = 1024 * 1024;
    let dms = layout::bounded_text(&home.join("DankMaterialShell/settings.json"), &mut budget)
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null);
    let mut directories = vec![home.join("waybar")];
    directories.extend(
        std::env::var("XDG_CONFIG_DIRS")
            .unwrap_or_else(|_| "/etc/xdg".into())
            .split(':')
            .filter(|path| !path.is_empty())
            .map(|path| std::path::Path::new(path).join("waybar")),
    );
    let waybar = directories
        .iter()
        .flat_map(|directory| {
            ["config", "config.jsonc", "config.json"].map(|name| directory.join(name))
        })
        .find_map(|path| {
            let text = layout::bounded_text(&path, &mut budget)?;
            let value: Value = serde_json::from_str(&layout::uncomment(&text)).ok()?;
            if value.is_object() {
                Some(value)
            } else {
                value
                    .as_array()?
                    .iter()
                    .find(|value| value.is_object())
                    .cloned()
            }
        })
        .unwrap_or(Value::Null);
    Settings {
        dms,
        waybar,
        edge: std::env::var("MARK_SHOT_NIRI_PANEL_EDGE")
            .ok()
            .map(|edge| edge.trim().to_lowercase()),
        size: std::env::var("MARK_SHOT_NIRI_PANEL_SIZE")
            .ok()
            .and_then(|value| value.parse::<i32>().ok())
            .map(f64::from),
    }
}

/// 【niri 窗口识别】【表面命中】按 layers、output、names 判断目标输出是否存在指定命名空间，返回命中结果
fn present(layers: &[Layer], output: &str, names: &[&str]) -> bool {
    layers.iter().any(|layer| {
        layer.output.as_deref() == Some(output) && names.contains(&layer.namespace.as_str())
    })
}

/// 【niri 窗口识别】【有限数值】按 value 与 default 读取数值或数值字符串，返回有限值
fn number(value: &Value, default: f64) -> f64 {
    value
        .as_f64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

/// 【niri 窗口识别】【布尔设置】按 value 和 default 读取原版布尔设置，返回实际开关
fn enabled(value: &Value, default: bool) -> bool {
    value.as_bool().unwrap_or(default)
}

/// 【niri 窗口识别】【边缘索引】按 value 与 default 读取名称或枚举，返回上、下、左、右索引
fn edge(value: &Value, default: usize) -> usize {
    let names = ["top", "bottom", "left", "right"];
    value
        .as_str()
        .and_then(|name| {
            names
                .iter()
                .position(|edge| *edge == name.trim().to_lowercase())
        })
        .or_else(|| value.as_u64().map(|number| number as usize))
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|index| *index < 4)
        .unwrap_or(default)
}

/// 【niri 窗口识别】【输出偏好】按 value 与 output 判断组件是否分配到当前输出，返回匹配结果
fn screen(value: &Value, output: &str) -> bool {
    value.as_array().is_none_or(|names| {
        names.is_empty() || names.iter().any(|name| name == "all" || name == output)
    })
}

/// 【niri 窗口识别】【DMS 独占区】按 settings、layers、output 汇总 frame、bar、dock，返回四边占用或无 DMS 表面
fn dms(settings: &Value, layers: &[Layer], output: &str) -> Option<[f64; 4]> {
    if !present(
        layers,
        output,
        &[
            "dms:bar",
            "dms:dock",
            "dms:dock-exclusion",
            "dms:frame",
            "dms:frame-exclusion",
        ],
    ) {
        return None;
    }
    let bars = settings["barConfigs"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let mut bar = [0.0; 4];
    if present(layers, output, &["dms:bar"]) {
        for config in bars {
            if !enabled(&config["enabled"], true)
                || !enabled(&config["visible"], true)
                || enabled(&config["autoHide"], false)
                || !screen(&config["screenPreferences"], output)
            {
                continue;
            }
            let padding = number(&config["innerPadding"], 4.0);
            let thickness = (26.0 + padding * 0.6).max(20.0);
            let amount = (thickness + padding + 4.0).max(36.0 + padding)
                + number(&config["spacing"], 4.0)
                + number(&config["bottomGap"], 0.0);
            if amount > 0.0 {
                bar[edge(&config["position"], 0)] += amount;
            }
        }
    }
    let mut dock = [0.0; 4];
    if present(layers, output, &["dms:dock", "dms:dock-exclusion"])
        && enabled(&settings["showDock"], true)
        && !enabled(&settings["dockAutoHide"], false)
        && !enabled(&settings["dockSmartAutoHide"], false)
    {
        let index = edge(&settings["dockPosition"], 1);
        if bar[index] <= 0.0 {
            let border = if enabled(&settings["dockBorderEnabled"], false) {
                number(&settings["dockBorderThickness"], 1.0)
            } else {
                0.0
            };
            dock[index] = (number(&settings["dockIconSize"], 35.0)
                + number(&settings["dockSpacing"], 4.0) * 3.0
                + 10.0
                + border * 2.0
                + number(&settings["dockBottomGap"], 0.0)
                + number(&settings["dockMargin"], 10.0))
            .max(0.0);
        }
    }
    // 1. 【niri 窗口识别】【框架优先】框架和停靠栏按较大占用合并，普通 bar 与 dock 才累加
    if enabled(&settings["frameEnabled"], false)
        && screen(&settings["frameScreenPreferences"], output)
        && present(layers, output, &["dms:frame", "dms:frame-exclusion"])
    {
        let thickness = number(&settings["frameThickness"], 16.0).max(0.0);
        let bar_size = number(&settings["frameBarSize"], 40.0).max(thickness);
        return Some(std::array::from_fn(|index| {
            let has_bar = bars.iter().any(|config| {
                enabled(&config["enabled"], true)
                    && screen(&config["screenPreferences"], output)
                    && edge(&config["position"], 0) == index
            });
            dock[index].max(if has_bar { bar_size } else { thickness })
        }));
    }
    Some(std::array::from_fn(|index| bar[index] + dock[index]))
}

/// 【niri 窗口识别】【面板几何】按 settings、layers、output、layout、height、tallest 返回四边独占区，未知水平面板由最高列反推
pub(super) fn insets(
    settings: &Settings,
    layers: &[Layer],
    output: &str,
    layout: &layout::Layout,
    height: f64,
    tallest: Option<f64>,
) -> [f64; 4] {
    if let Some(insets) = dms(&settings.dms, layers, output) {
        return insets;
    }
    let mut inset = [0.0; 4];
    let (index, size) = if let Some(name) = settings
        .edge
        .as_deref()
        .filter(|name| ["top", "bottom", "left", "right", "none"].contains(name))
    {
        if name == "none" {
            return inset;
        }
        (edge(&Value::String(name.into()), 0), settings.size)
    } else {
        if !present(layers, output, &["waybar"]) || !enabled(&settings.waybar["exclusive"], true) {
            return inset;
        }
        let index = edge(&settings.waybar["position"], 0);
        let size = number(
            &settings.waybar[if index < 2 { "height" } else { "width" }],
            0.0,
        );
        (index, (size > 0.0).then_some(size))
    };
    inset[index] = size
        .or_else(|| {
            (index < 2)
                .then(|| {
                    tallest.map(|height_used| {
                        (height - layout.struts[0] - layout.struts[1] - height_used).max(0.0)
                    })
                })
                .flatten()
        })
        .unwrap_or(0.0);
    inset
}
