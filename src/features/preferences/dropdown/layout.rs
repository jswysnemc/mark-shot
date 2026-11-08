//! 【设置下拉】【浮层几何】下拉优先向下展开，空间不足向上展开，列表与输入使用统一命中范围

use super::super::{layout::rect, window::contains};
use super::Dropdown;
use crate::ui::scrollbar;
use tiny_skia::Rect;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::features::preferences) enum Hit {
    Option(usize),
    Search,
    Scrollbar,
}

pub(in crate::features::preferences) struct Layout {
    pub bounds: Rect,
    pub search: Option<Rect>,
    pub list: Rect,
    pub rows: Vec<(usize, Rect)>,
    pub capacity: usize,
    pub scrollbar: Option<scrollbar::Geometry>,
}

impl Layout {
    /// 【设置下拉】【边缘适配】按 menu 和 size 返回紧贴字段的浮层，最大七行且不覆盖窗口顶栏和底栏
    pub fn new(menu: &Dropdown, size: (u32, u32)) -> Self {
        let header = if menu.search.is_some() { 42.0 } else { 0.0 };
        let desired = menu.options.len().clamp(1, 7) as f32 * 34.0 + header + 12.0;
        let bottom = size.1 as f32 - 38.0;
        let below = (bottom - menu.anchor.bottom() - 5.0).max(0.0);
        let above = (menu.anchor.top() - 57.0).max(0.0);
        let down = below >= desired || below >= above;
        let available = if down { below } else { above };
        let capacity = ((available - header - 12.0) / 34.0).floor().clamp(1.0, 7.0) as usize;
        let height = menu.options.len().clamp(1, capacity) as f32 * 34.0 + header + 12.0;
        let width = menu
            .anchor
            .width()
            .max(if menu.search.is_some() { 280.0 } else { 208.0 })
            .min(size.0 as f32 - 24.0);
        let left = menu
            .anchor
            .left()
            .min(size.0 as f32 - width - 12.0)
            .max(12.0);
        let top = if down {
            menu.anchor.bottom() + 5.0
        } else {
            menu.anchor.top() - height - 5.0
        }
        .clamp(52.0, (bottom - height).max(52.0));
        let bounds = rect(left, top, width, height);
        let search = menu
            .search
            .as_ref()
            .map(|_| rect(left + 8.0, top + 8.0, width - 16.0, 30.0));
        let list = rect(
            left + 5.0,
            top + header + 6.0,
            width - 10.0,
            height - header - 12.0,
        );
        let scrollbar = scrollbar::geometry(list, menu.filtered.len(), capacity, menu.offset);
        let rows = menu
            .filtered
            .iter()
            .skip(menu.offset)
            .take(capacity)
            .enumerate()
            .map(|(position, index)| {
                (
                    *index,
                    rect(
                        list.left(),
                        list.top() + position as f32 * 34.0,
                        list.width() - if scrollbar.is_some() { 12.0 } else { 0.0 },
                        34.0,
                    ),
                )
            })
            .collect();
        Self {
            bounds,
            search,
            list,
            rows,
            capacity,
            scrollbar,
        }
    }

    /// 【设置下拉】【局部命中】按 point 返回浮层中的具体控件，空白区域不映射选项
    pub fn hit(&self, point: (f64, f64)) -> Option<Hit> {
        if self.search.is_some_and(|bounds| contains(bounds, point)) {
            return Some(Hit::Search);
        }
        if self
            .scrollbar
            .as_ref()
            .is_some_and(|bar| contains(bar.track, point))
        {
            return Some(Hit::Scrollbar);
        }
        self.rows
            .iter()
            .find(|(_, bounds)| contains(*bounds, point))
            .map(|(index, _)| Hit::Option(*index))
    }
}
