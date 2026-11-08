//! 【工具栏设置】【共享几何】预览、控件、候选按钮与滚动条共用逻辑像素布局

use super::super::window::{Preferences, contains};
use crate::{
    config::toolbar_layout::Profile,
    ui::{
        scrollbar,
        toolbar::{ToolbarItem, catalog},
    },
};
use tiny_skia::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::features::preferences) enum Hit {
    Profile(Profile),
    Item(usize),
    Add(ToolbarItem),
    Reset,
    Left,
    Right,
    Remove,
    Shortcut,
    ClearShortcut,
    ResetShortcut,
    Direction,
}

pub(in crate::features::preferences) struct Layout {
    pub viewport: Rect,
    pub preview: Rect,
    pub cells: Vec<Rect>,
    pub controls: Vec<(Hit, Rect)>,
    pub available: Vec<(ToolbarItem, Rect)>,
    pub selected: Rect,
    pub add_title: Rect,
    pub height: usize,
    pub scrollbar: Option<scrollbar::Geometry>,
}

/// 【工具栏设置】【矩形构造】按 left、top、width、height 返回非空逻辑矩形
pub(super) fn rect(left: f32, top: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(left, top, width.max(1.0), height.max(1.0)).unwrap()
}

impl Layout {
    /// 【工具栏设置】【响应式排版】按 state 窗口宽度换行按钮与候选项，返回完整内容和可视范围
    pub fn new(state: &Preferences) -> Self {
        let left = super::super::draw::sidebar(state) + 24.0;
        let width = state.size.0 as f32 - left - 30.0;
        let viewport = rect(left, 140.0, width + 16.0, state.size.1 as f32 - 196.0);
        let top = viewport.top() - state.toolbar.scroll as f32;
        let mut controls = Vec::new();
        for (index, profile) in Profile::ALL.into_iter().enumerate() {
            controls.push((
                Hit::Profile(profile),
                rect(
                    left + index as f32 * (width / 3.0),
                    top,
                    width / 3.0 - 5.0,
                    30.0,
                ),
            ));
        }
        controls.push((
            Hit::Reset,
            rect(left + width - 112.0, top + 44.0, 112.0, 26.0),
        ));
        let items = state.toolbar.profile.items(&state.toolbar.config);
        let columns = ((width - 20.0) / 34.0).floor().max(1.0) as usize;
        let rows = items.len().div_ceil(columns).max(1);
        let preview = rect(left, top + 78.0, width, rows as f32 * 36.0 + 14.0);
        let cells = (0..items.len())
            .map(|index| {
                rect(
                    left + 9.0 + (index % columns) as f32 * 34.0,
                    preview.top() + 7.0 + (index / columns) as f32 * 36.0,
                    32.0,
                    32.0,
                )
            })
            .collect();
        let selected = rect(left, preview.bottom() + 8.0, width, 26.0);
        let actions_top = selected.bottom() + 4.0;
        for (index, hit) in [Hit::Left, Hit::Right, Hit::Remove, Hit::Direction]
            .into_iter()
            .enumerate()
        {
            controls.push((
                hit,
                rect(
                    left + index as f32 * width / 4.0,
                    actions_top,
                    width / 4.0 - 5.0,
                    28.0,
                ),
            ));
        }
        for (index, hit) in [Hit::Shortcut, Hit::ClearShortcut, Hit::ResetShortcut]
            .into_iter()
            .enumerate()
        {
            controls.push((
                hit,
                rect(
                    left + index as f32 * width / 3.0,
                    actions_top + 34.0,
                    width / 3.0 - 5.0,
                    28.0,
                ),
            ));
        }
        let add_title = rect(left, actions_top + 76.0, width, 26.0);
        let columns = (width / 158.0).floor().max(2.0) as usize;
        let candidates = catalog::ENTRIES
            .iter()
            .map(|entry| ToolbarItem::Button(entry.button))
            .filter(|item| !items.contains(item))
            .chain([ToolbarItem::Seperator]);
        let available: Vec<_> = candidates
            .enumerate()
            .map(|(index, item)| {
                (
                    item,
                    rect(
                        left + (index % columns) as f32 * width / columns as f32,
                        add_title.bottom() + 6.0 + (index / columns) as f32 * 38.0,
                        width / columns as f32 - 6.0,
                        32.0,
                    ),
                )
            })
            .collect();
        let bottom = available
            .last()
            .map_or(add_title.bottom(), |(_, rect)| rect.bottom())
            + 12.0;
        let height = (bottom - top).ceil() as usize;
        let scrollbar = scrollbar::geometry(
            viewport,
            height,
            viewport.height() as usize,
            state.toolbar.scroll,
        );
        Self {
            viewport,
            preview,
            cells,
            controls,
            available,
            selected,
            add_title,
            height,
            scrollbar,
        }
    }

    /// 【工具栏设置】【内容命中】按 point 返回可见控件，屏幕外内容不可点击
    pub fn hit(&self, point: (f64, f64)) -> Option<Hit> {
        if !contains(self.viewport, point) {
            return None;
        }
        self.controls
            .iter()
            .find(|(_, rect)| contains(*rect, point))
            .map(|(hit, _)| *hit)
            .or_else(|| {
                self.cells
                    .iter()
                    .position(|rect| contains(*rect, point))
                    .map(Hit::Item)
            })
            .or_else(|| {
                self.available
                    .iter()
                    .find(|(_, rect)| contains(*rect, point))
                    .map(|(item, _)| Hit::Add(*item))
            })
    }

    /// 【工具栏设置】【插入位置】按 point 返回拖动插槽，行尾支持追加；预览外不接受投放
    pub fn insertion(&self, point: (f64, f64)) -> Option<usize> {
        if !contains(self.viewport, point) || !contains(self.preview, point) {
            return None;
        }
        Some(
            self.cells
                .iter()
                .position(|rect| {
                    point.1 < f64::from(rect.bottom())
                        && (point.1 < f64::from(rect.top())
                            || point.0 < f64::from(rect.left() + rect.width() / 2.0))
                })
                .unwrap_or(self.cells.len()),
        )
    }

    /// 【工具栏设置】【滚动边界】无参数，返回当前内容最大纵向偏移
    pub fn maximum(&self) -> usize {
        self.height.saturating_sub(self.viewport.height() as usize)
    }
}
