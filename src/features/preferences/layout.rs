//! 【设置窗口】【共享布局】绘制、命中和滚动使用同一逻辑坐标，分组随宽度适配

use super::{
    catalog::{ENTRIES, Kind},
    sections,
    window::{Control, Preferences, contains},
};
use tiny_skia::Rect;

/// 【设置窗口】【矩形创建】按逻辑位置和尺寸返回非空矩形
pub(super) fn rect(left: f32, top: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(left, top, width.max(1.0), height.max(1.0)).unwrap()
}

/// 【设置窗口】【侧栏尺寸】按 state 返回窄窗口或标准导航宽度
pub(super) fn sidebar(state: &Preferences) -> f32 {
    if state.size.0 < 720 { 144.0 } else { 176.0 }
}
/// 【设置窗口】【导航位置】按 state、index 返回紧凑导航范围，底部保留保存说明
pub(super) fn nav_rect(state: &Preferences, index: usize) -> Rect {
    rect(
        10.0,
        56.0 + index as f32
            * ((state.size.1 as f32 - 98.0) / super::catalog::Page::ALL.len() as f32).min(36.0),
        sidebar(state) - 20.0,
        ((state.size.1 as f32 - 98.0) / super::catalog::Page::ALL.len() as f32 - 3.0)
            .clamp(26.0, 32.0),
    )
}
/// 【设置窗口】【搜索位置】按 state 返回顶栏搜索范围，宽窗口限制最大长度
pub(super) fn search_rect(state: &Preferences) -> Rect {
    let width = (state.size.0 as f32 - sidebar(state) - 92.0).min(384.0);
    rect(
        sidebar(state) + (state.size.0 as f32 - sidebar(state) - width - 48.0) / 2.0,
        9.0,
        width,
        30.0,
    )
}
/// 【设置窗口】【关闭位置】按 state 返回右上角关闭控件
pub(super) fn close_rect(state: &Preferences) -> Rect {
    rect(state.size.0 as f32 - 42.0, 9.0, 30.0, 30.0)
}
/// 【设置窗口】【路径位置】按 state 返回底栏可复制路径区域
pub(super) fn path_rect(state: &Preferences) -> Rect {
    rect(
        14.0,
        state.size.1 as f32 - 29.0,
        (state.size.0 as f32 * 0.45).min(420.0),
        26.0,
    )
}
/// 【设置窗口】【内容区域】按 state 返回标题下方的页面滚动范围，不覆盖顶栏与底栏
pub(super) fn viewport(state: &Preferences) -> Rect {
    rect(
        sidebar(state) + 20.0,
        126.0,
        state.size.0 as f32 - sidebar(state) - 32.0,
        state.size.1 as f32 - 170.0,
    )
}

pub(super) struct Row {
    pub index: usize,
    pub bounds: Rect,
    pub control: Rect,
    pub stacked: bool,
    pub compact: bool,
}
pub(super) struct Group {
    pub bounds: Rect,
    pub title: &'static str,
}
pub(super) struct Layout {
    pub viewport: Rect,
    pub rows: Vec<Row>,
    pub groups: Vec<Group>,
    pub maximum: usize,
    pub preview: Option<Rect>,
}

impl Layout {
    /// 【设置窗口】【分组排版】按 state 构造完整内容，返回行、面板与最大滚动距离；窄窗口长字段上下排列
    pub fn new(state: &Preferences) -> Self {
        let viewport = viewport(state);
        let width = viewport.width() - 14.0;
        let visible = state.visible();
        let mut groups = Vec::new();
        let mut rows = Vec::new();
        let mut top = viewport.top() - state.scroll as f32;
        let mut last_group = None;
        let mut preview = None;
        let mut frame_left = viewport.left();
        for index in visible {
            let entry = &ENTRIES[index];
            let group = if state.search.text().is_empty() {
                sections::group(entry)
            } else {
                (0, "搜索结果", "Search results")
            };
            if last_group != Some(group.0) {
                if !groups.is_empty() {
                    top += 16.0;
                }
                groups.push(Group {
                    bounds: rect(viewport.left(), top, width, 34.0),
                    title: if state.chinese() { group.1 } else { group.2 },
                });
                top += 34.0;
                frame_left = viewport.left();
                if entry.section == "export.image_frame" && state.search.text().is_empty() {
                    let wide = width >= 600.0;
                    let preview_width = if wide {
                        (width * 0.40).min(260.0)
                    } else {
                        width - 32.0
                    };
                    preview = Some(rect(
                        viewport.left() + 16.0,
                        top + 8.0,
                        preview_width,
                        if wide { 248.0 } else { 144.0 },
                    ));
                    if wide {
                        frame_left += preview_width + 16.0;
                    } else {
                        top += 160.0;
                    }
                }
                last_group = Some(group.0);
            }
            let numeric = entry.step().is_some();
            let compact = numeric || entry.section == "export.image_frame";
            let stacked = width < 520.0 && matches!(entry.kind, Kind::Text) && !numeric;
            let height = if compact {
                44.0
            } else if stacked {
                84.0
            } else if matches!(entry.kind, Kind::Shortcut) {
                48.0
            } else {
                60.0
            };
            let control_width = if matches!(entry.kind, Kind::Toggle) {
                38.0
            } else if numeric {
                132.0
            } else if stacked {
                width - 32.0
            } else {
                (width * 0.40).clamp(132.0, 300.0)
            };
            let control = rect(
                viewport.left() + width - control_width - 16.0,
                top + if stacked { 44.0 } else { (height - 28.0) / 2.0 },
                control_width,
                28.0,
            );
            rows.push(Row {
                index,
                bounds: rect(
                    frame_left,
                    top,
                    width - (frame_left - viewport.left()),
                    height,
                ),
                control,
                stacked,
                compact,
            });
            top += height;
            if let Some(group) = groups.last_mut() {
                group.bounds = rect(
                    group.bounds.left(),
                    group.bounds.top(),
                    width,
                    top - group.bounds.top(),
                );
            }
        }
        let height = (top - viewport.top() + state.scroll as f32).ceil() as usize;
        Self {
            viewport,
            rows,
            groups,
            maximum: height.saturating_sub(viewport.height() as usize),
            preview,
        }
    }

    /// 【设置窗口】【可见命中】按 point 返回表单动作，滚动裁切外不响应，主题分段分别保存
    pub fn hit(&self, state: &Preferences, point: (f64, f64)) -> Option<Control> {
        if !contains(self.viewport, point) {
            return None;
        }
        for row in &self.rows {
            if ENTRIES[row.index].step().is_some() {
                if state
                    .editing
                    .as_ref()
                    .is_some_and(|(index, _)| *index == row.index)
                    && contains(number_reset(row.control), point)
                {
                    return Some(Control::Reset(row.index));
                }
                for direction in [-1, 1] {
                    if contains(step_rect(row.control, direction), point) {
                        return Some(Control::Step(row.index, direction));
                    }
                }
                if contains(row.bounds, point) {
                    return Some(Control::Row(row.index));
                }
                continue;
            }
            if state
                .editing
                .as_ref()
                .is_some_and(|(index, _)| *index == row.index)
            {
                if contains(reset_rect(row.control), point) {
                    return Some(Control::Reset(row.index));
                }
                if matches!(ENTRIES[row.index].kind, Kind::Shortcut)
                    && contains(apply_rect(row.control), point)
                {
                    return Some(Control::Clear(row.index));
                }
            }
            if state
                .editing
                .as_ref()
                .is_some_and(|(index, _)| *index == row.index)
                && matches!(ENTRIES[row.index].kind, Kind::Text)
                && contains(apply_rect(row.control), point)
            {
                return Some(Control::Apply);
            }
            if contains(row.control, point) {
                if let Kind::Segments(choices) = ENTRIES[row.index].kind {
                    return Some(Control::Segment(
                        row.index,
                        (((point.0 as f32 - row.control.left())
                            / (row.control.width() / choices.len() as f32))
                            as usize)
                            .min(choices.len() - 1),
                    ));
                }
                return Some(Control::Row(row.index));
            }
            if contains(row.bounds, point) {
                return Some(Control::Row(row.index));
            }
        }
        None
    }
}

/// 【设置窗口】【数值恢复】按 control 返回数值框左侧的恢复按钮，编辑时展示
pub(super) fn number_reset(control: Rect) -> Rect {
    rect(control.left() - 46.0, control.top(), 40.0, control.height())
}

/// 【设置窗口】【步进按钮】按 control 和 direction 返回减号或加号的紧凑命中范围
pub(super) fn step_rect(control: Rect, direction: i8) -> Rect {
    rect(
        if direction < 0 {
            control.left()
        } else {
            control.right() - 28.0
        },
        control.top(),
        28.0,
        control.height(),
    )
}

/// 【设置窗口】【编辑范围】按 row 返回文字编辑区域，数值输入与两侧步进按钮互不遮挡
pub(super) fn editor_rect(row: &Row) -> Rect {
    if ENTRIES[row.index].step().is_some() {
        rect(
            row.control.left() + 32.0,
            row.control.top() + 1.0,
            row.control.width() - 64.0,
            26.0,
        )
    } else {
        rect(
            row.control.left() + 8.0,
            row.control.top() + 1.0,
            row.control.width() - 110.0,
            26.0,
        )
    }
}

/// 【设置窗口】【保存按钮】按 control 返回输入框末端的提交按钮范围
pub(super) fn apply_rect(control: Rect) -> Rect {
    rect(
        control.right() - 48.0,
        control.top(),
        48.0,
        control.height(),
    )
}

/// 【设置窗口】【恢复按钮】按 control 返回编辑框内恢复继承的操作范围
pub(super) fn reset_rect(control: Rect) -> Rect {
    rect(
        control.right() - 96.0,
        control.top(),
        48.0,
        control.height(),
    )
}

/// 【设置窗口】【边缘调整】按 size、position 返回六像素边缘或角落方向，内容区域返回空值
pub(super) fn resize_edge(
    size: (u32, u32),
    position: (f64, f64),
) -> Option<crate::backend::window::WindowEdge> {
    use crate::backend::window::WindowEdge::*;
    let left = position.0 < 6.0;
    let right = position.0 >= size.0 as f64 - 6.0;
    let top = position.1 < 6.0;
    let bottom = position.1 >= size.1 as f64 - 6.0;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(TopLeft),
        (_, true, true, _) => Some(TopRight),
        (true, _, _, true) => Some(BottomLeft),
        (_, true, _, true) => Some(BottomRight),
        (true, _, _, _) => Some(Left),
        (_, true, _, _) => Some(Right),
        (_, _, true, _) => Some(Top),
        (_, _, _, true) => Some(Bottom),
        _ => None,
    }
}
