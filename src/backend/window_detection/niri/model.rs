//! 【niri 窗口识别】【协议数据】只读取几何和活动工作区所需字段

use serde::Deserialize;
use std::collections::BTreeMap;

pub(super) type Outputs = BTreeMap<String, Output>;

#[derive(Deserialize)]
pub(super) struct Output {
    pub logical: Option<Logical>,
}

#[derive(Deserialize)]
pub(super) struct Logical {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Deserialize)]
pub(super) struct Workspace {
    pub id: u64,
    pub output: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub is_focused: bool,
    pub active_window_id: Option<u64>,
}

#[derive(Deserialize)]
pub(super) struct Window {
    pub id: u64,
    pub workspace_id: Option<u64>,
    #[serde(default)]
    pub is_floating: bool,
    #[serde(default)]
    pub is_focused: bool,
    #[serde(default)]
    pub focus_timestamp: Option<FocusStamp>,
    pub layout: Option<Layout>,
}

#[derive(Deserialize, Default)]
pub(super) struct FocusStamp {
    pub secs: u64,
    pub nanos: u32,
}

#[derive(Deserialize)]
pub(super) struct Layout {
    pub pos_in_scrolling_layout: Option<[f64; 2]>,
    pub tile_pos_in_workspace_view: Option<[f64; 2]>,
    pub tile_size: Option<[f64; 2]>,
    pub window_size: Option<[f64; 2]>,
    pub window_offset_in_tile: Option<[f64; 2]>,
}

#[derive(Deserialize)]
pub(super) struct Layer {
    pub namespace: String,
    pub output: Option<String>,
}
