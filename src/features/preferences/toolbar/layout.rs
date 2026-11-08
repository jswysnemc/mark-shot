//! 【工具栏设置】【共享几何】两栏预览、候选按钮与滚动条共用逻辑像素布局

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
    Item(Profile, usize),
    Add(ToolbarItem),
    Reset(Profile),
    Orientation(Profile),
    Left,
    Right,
    Remove,
    Shortcut,
}

pub(in crate::features::preferences) struct Bar {
    pub profile: Profile,
    pub panel: Rect,
    pub preview: Rect,
    pub cells: Vec<Rect>,
    pub caption: Rect,
    columns: usize,
}

impl Bar {
    /// 【工具栏设置】【槽位位置】按 index 返回当前栏的按钮位置，允许预留一个拖入槽位
    pub fn slot(&self, index: usize) -> Rect {
        rect(
            self.preview.left() + 9.0 + (index % self.columns) as f32 * 34.0,
            self.preview.top() + 7.0 + (index / self.columns) as f32 * 36.0,
            32.0,
            32.0,
        )
    }
}

pub(in crate::features::preferences) struct Layout {
    pub viewport: Rect,
    pub bars: Vec<Bar>,
    pub controls: Vec<(Hit, Rect)>,
    pub available: Vec<(ToolbarItem, Rect)>,
    pub pool: Rect,
    pub height: usize,
    pub scrollbar: Option<scrollbar::Geometry>,
}

/// 【工具栏设置】【矩形构造】按 left、top、width、height 返回非空逻辑矩形
pub(super) fn rect(left: f32, top: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(left, top, width.max(1.0), height.max(1.0)).unwrap()
}

impl Layout {
    /// 【工具栏设置】【响应式排版】按 state 窗口宽度换行两栏按钮和候选项，返回内容与可视范围
    pub fn new(state: &Preferences) -> Self {
        let viewport = super::super::layout::viewport(state);
        let left = viewport.left() + 14.0;
        let width = viewport.width() - 42.0;
        let top = viewport.top() - state.toolbar.scroll as f32;
        let mut next_top = top;
        let mut controls = Vec::new();
        let mut bars = Vec::new();
        for profile in Profile::ALL {
            let items = profile.items(&state.toolbar.config);
            let columns = ((width - 20.0) / 34.0).floor().max(1.0) as usize;
            let extra = usize::from(state.toolbar.drag.as_ref().is_some_and(|drag| {
                drag.active && matches!(drag.target, Some(super::drag::Target::Bar(target, _)) if target == profile)
            }));
            let rows = (items.len() + extra).div_ceil(columns).max(1);
            let preview = rect(left, next_top + 46.0, width, rows as f32 * 36.0 + 14.0);
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
            let selected = state.toolbar.profile == profile && state.toolbar.selected.is_some();
            let caption = rect(
                left,
                preview.bottom() + 6.0,
                width - if selected { 202.0 } else { 0.0 },
                26.0,
            );
            controls.push((
                Hit::Profile(profile),
                rect(left, next_top + 10.0, width - 180.0, 28.0),
            ));
            controls.push((
                Hit::Orientation(profile),
                rect(left + width - 174.0, next_top + 10.0, 76.0, 28.0),
            ));
            controls.push((
                Hit::Reset(profile),
                rect(left + width - 90.0, next_top + 10.0, 90.0, 28.0),
            ));
            if selected {
                if super::shortcuts::key(state).is_some() {
                    controls.push((
                        Hit::Shortcut,
                        rect(left + width - 194.0, caption.top(), 118.0, 26.0),
                    ));
                }
                controls.push((
                    Hit::Remove,
                    rect(left + width - 68.0, caption.top(), 68.0, 26.0),
                ));
            }
            let panel = rect(
                viewport.left(),
                next_top,
                viewport.width() - 14.0,
                caption.bottom() + 10.0 - next_top,
            );
            next_top = panel.bottom() + 12.0;
            bars.push(Bar {
                profile,
                panel,
                preview,
                cells,
                caption,
                columns,
            });
        }
        let columns = (width / 158.0).floor().max(2.0) as usize;
        let available: Vec<_> = catalog::ENTRIES
            .iter()
            .map(|entry| ToolbarItem::Button(entry.button))
            .filter(|item| {
                !state.toolbar.config.main.contains(item)
                    && !state.toolbar.config.actions.contains(item)
            })
            .chain([ToolbarItem::Seperator])
            .enumerate()
            .map(|(index, item)| {
                (
                    item,
                    rect(
                        left + (index % columns) as f32 * width / columns as f32,
                        next_top + 44.0 + (index / columns) as f32 * 38.0,
                        width / columns as f32 - 6.0,
                        32.0,
                    ),
                )
            })
            .collect();
        let bottom = available
            .last()
            .map_or(next_top + 44.0, |(_, area)| area.bottom())
            + 12.0;
        let pool = rect(
            viewport.left(),
            next_top,
            viewport.width() - 14.0,
            bottom - next_top,
        );
        let height = (bottom - top).ceil() as usize;
        let scrollbar = scrollbar::geometry(
            viewport,
            height,
            viewport.height() as usize,
            state.toolbar.scroll,
        );
        Self {
            viewport,
            bars,
            controls,
            available,
            pool,
            height,
            scrollbar,
        }
    }

    /// 【工具栏设置】【内容命中】按 point 返回可见控件及所属工具栏，屏幕外内容不可点击
    pub fn hit(&self, point: (f64, f64)) -> Option<Hit> {
        if !contains(self.viewport, point) {
            return None;
        }
        self.controls
            .iter()
            .find(|(_, area)| contains(*area, point))
            .map(|(hit, _)| *hit)
            .or_else(|| {
                self.bars.iter().find_map(|bar| {
                    bar.cells
                        .iter()
                        .position(|area| contains(*area, point))
                        .map(|index| Hit::Item(bar.profile, index))
                })
            })
            .or_else(|| {
                self.available
                    .iter()
                    .find(|(_, area)| contains(*area, point))
                    .map(|(item, _)| Hit::Add(*item))
            })
    }

    /// 【工具栏设置】【插入位置】按 profile 与 point 返回栏内插槽，行尾支持追加，其他工具栏不接受投放
    pub fn insertion(&self, profile: Profile, point: (f64, f64)) -> Option<usize> {
        let bar = self.bars.iter().find(|bar| bar.profile == profile)?;
        if !contains(self.viewport, point) || !contains(bar.preview, point) {
            return None;
        }
        Some(
            bar.cells
                .iter()
                .position(|area| {
                    point.1 < f64::from(area.bottom())
                        && (point.1 < f64::from(area.top())
                            || point.0 < f64::from(area.left() + area.width() / 2.0))
                })
                .unwrap_or(bar.cells.len()),
        )
    }

    /// 【工具栏设置】【滚动边界】无参数，返回当前内容最大纵向偏移
    pub fn maximum(&self) -> usize {
        self.height.saturating_sub(self.viewport.height() as usize)
    }
}
