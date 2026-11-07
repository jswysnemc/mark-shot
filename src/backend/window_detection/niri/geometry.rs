//! 【niri 窗口识别】【窗口几何】优先可见 tile 坐标，旧版按活动列与工作区推算

use super::{layout, model, panels};
use crate::backend::window_detection::{Window, geometry};
use std::collections::{BTreeMap, HashSet};

struct Entry<'a> {
    window: &'a model::Window,
    layout: &'a model::Layout,
    column: i64,
    row: i64,
    size: [f64; 2],
}

/// 【niri 窗口识别】【尺寸校验】按 size 判断是否具有有限正宽高，返回有效结果
fn valid(size: [f64; 2]) -> bool {
    size.iter().all(|value| value.is_finite() && *value > 1.0)
}

/// 【niri 窗口识别】【精确边界】按 layout 和 output 返回直接上报的装饰边界，缺少 tile_size 时使用内容偏移
fn exact(layout: &model::Layout, output: &model::Logical) -> Option<tiny_skia::Rect> {
    let [x, y] = layout.tile_pos_in_workspace_view?;
    if !valid(layout.window_size?) {
        return None;
    }
    let (size, offset) = match layout.tile_size {
        Some(size) => (size, [0.0, 0.0]),
        None => (
            layout.window_size?,
            layout.window_offset_in_tile.unwrap_or([0.0, 0.0]),
        ),
    };
    geometry::rect(
        output.x + x + offset[0],
        output.y + y + offset[1],
        size[0],
        size[1],
    )
}

/// 【niri 窗口识别】【平铺条目】按 windows 与 workspace 读取有效平铺列，返回保持原版顺序的条目
fn entries(windows: &[model::Window], workspace: u64) -> Vec<Entry<'_>> {
    windows
        .iter()
        .filter_map(|window| {
            if window.workspace_id != Some(workspace) || window.is_floating {
                return None;
            }
            let layout = window.layout.as_ref()?;
            let position = layout.pos_in_scrolling_layout?;
            let size = layout.tile_size?;
            if !valid(size)
                || layout.window_size.is_none()
                || position.iter().any(|value| !value.is_finite())
            {
                return None;
            }
            Some(Entry {
                window,
                layout,
                column: position[0].round_ties_even() as i64,
                row: position[1].round_ties_even() as i64,
                size,
            })
        })
        .collect()
}

/// 【niri 窗口识别】【活动列】按 entries 与 workspace 定位活动、聚焦或最近聚焦的列，返回可见性锚点
fn active(entries: &[Entry<'_>], workspace: &model::Workspace) -> Option<i64> {
    if let Some(entry) = entries
        .iter()
        .find(|entry| Some(entry.window.id) == workspace.active_window_id)
        .or_else(|| entries.iter().find(|entry| entry.window.is_focused))
    {
        return Some(entry.column);
    }
    let mut latest = None;
    let mut timestamp = (0, 0);
    for entry in entries {
        let current = entry
            .window
            .focus_timestamp
            .as_ref()
            .map_or((0, 0), |stamp| (stamp.secs, stamp.nanos));
        if latest.is_none() || current > timestamp {
            latest = Some(entry.column);
            timestamp = current;
        }
    }
    latest
}

/// 【niri 窗口识别】【旧版平铺】按 windows、workspace、output、layout、panels 和 emitted 重建缺少可见坐标的平铺边界，返回矩形
fn tiled(
    windows: &[model::Window],
    workspace: &model::Workspace,
    output: &model::Logical,
    layout: &layout::Layout,
    panel_settings: &panels::Settings,
    layers: &[model::Layer],
    emitted: &HashSet<u64>,
) -> Vec<tiny_skia::Rect> {
    let entries = entries(windows, workspace.id);
    if entries.is_empty() {
        return Vec::new();
    }
    let mut columns: BTreeMap<i64, Vec<&Entry<'_>>> = BTreeMap::new();
    for entry in &entries {
        columns.entry(entry.column).or_default().push(entry);
    }
    let tallest = columns
        .values()
        .map(|column| {
            column.iter().map(|entry| entry.size[1]).sum::<f64>()
                + layout.gaps * (column.len() + 1) as f64
        })
        .max_by(f64::total_cmp);
    let inset = panels::insets(
        panel_settings,
        layers,
        workspace.output.as_deref().unwrap_or_default(),
        layout,
        output.height,
        tallest,
    );
    let left = layout.struts[2] + inset[2];
    let top = output.y + layout.struts[0] + inset[0];
    let width = output.width - left - layout.struts[3] - inset[3];
    let height = output.height - layout.struts[0] - layout.struts[1] - inset[0] - inset[1];
    if width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }
    let mut starts = BTreeMap::new();
    let mut widths = BTreeMap::new();
    let mut cursor = 0.0;
    // 1. 【niri 窗口识别】【列宽累积】每列取最大 tile 宽度，列内行号不影响横向起点
    for (&column, entries) in &columns {
        let width = entries
            .iter()
            .map(|entry| entry.size[0])
            .max_by(f64::total_cmp)
            .unwrap_or_default();
        starts.insert(column, cursor);
        widths.insert(column, width);
        cursor += width + layout.gaps;
    }
    let mut view = -(left + layout.gaps);
    if let Some((entry, point)) = entries
        .iter()
        .find_map(|entry| Some((entry, entry.layout.tile_pos_in_workspace_view?)))
    {
        view = starts[&entry.column] - point[0];
    } else if let Some(column) = active(&entries, workspace) {
        let start = starts[&column];
        let end = start + widths[&column];
        if start < view + left + layout.gaps {
            view = start - left - layout.gaps;
        } else if end + layout.gaps > view + left + width {
            view = end + layout.gaps - left - width;
        }
    }
    let mut result = Vec::new();
    // 2. 【niri 窗口识别】【纵向间隙】工作区顶边保留 gaps，全屏 tile 直接占满输出且不推进常规堆叠
    for (column, mut entries) in columns {
        entries.sort_by_key(|entry| entry.row);
        let x = output.x + starts[&column] - view;
        let mut y = top + layout.gaps;
        for entry in entries {
            let skip =
                emitted.contains(&entry.window.id) || !entry.layout.window_size.is_some_and(valid);
            let fullscreen = entry.size[0] >= output.width && entry.size[1] >= output.height;
            if !skip
                && let Some(rect) = geometry::rect(
                    if fullscreen { output.x } else { x },
                    if fullscreen { output.y } else { y },
                    entry.size[0],
                    entry.size[1],
                )
            {
                result.push(rect);
            }
            if !fullscreen {
                y += entry.size[1] + layout.gaps;
            }
        }
    }
    result
}

/// 【niri 窗口识别】【快照组装】按 outputs、workspaces、windows、layers、layout 和 panels 汇总各活动输出，返回独立逻辑矩形
pub(super) fn build(
    outputs: &model::Outputs,
    workspaces: &[model::Workspace],
    windows: &[model::Window],
    layers: &[model::Layer],
    layout: &layout::Layout,
    panels: &panels::Settings,
) -> Vec<Window> {
    let mut result = Vec::new();
    let mut emitted = HashSet::new();
    for workspace in workspaces
        .iter()
        .filter(|workspace| workspace.is_active || workspace.is_focused)
    {
        let Some(output) = workspace
            .output
            .as_ref()
            .and_then(|name| outputs.get(name))
            .and_then(|output| output.logical.as_ref())
        else {
            continue;
        };
        if output.width <= 0.0 || output.height <= 0.0 {
            continue;
        }
        for window in windows
            .iter()
            .filter(|window| window.workspace_id == Some(workspace.id))
        {
            if let Some(rect) = window
                .layout
                .as_ref()
                .and_then(|layout| exact(layout, output))
            {
                emitted.insert(window.id);
                geometry::append(&mut result, rect, None);
            }
        }
        for rect in tiled(windows, workspace, output, layout, panels, layers, &emitted) {
            geometry::append(&mut result, rect, None);
        }
    }
    result
}
