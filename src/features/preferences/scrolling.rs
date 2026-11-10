//! 【设置窗口】【列表滚动】复用历史列表滚动条，滚轮、拖动和翻页保持相同边界

use super::window::Preferences;
use crate::{keys::Key, ui::scrollbar};

impl Preferences {
    /// 【设置窗口】【数值滚轮】按 position 与 delta 微调悬停数值，先提交草稿；返回是否消费，边界不滚动页面
    pub(super) fn scroll_number(&mut self, position: (f64, f64), delta: f64) -> bool {
        let layout = super::layout::Layout::new(self);
        let Some(index) = layout
            .rows
            .iter()
            .find(|row| {
                super::catalog::ENTRIES[row.index].step().is_some()
                    && super::window::contains(row.control, position)
                    && super::window::contains(layout.viewport, position)
            })
            .map(|row| row.index)
        else {
            return false;
        };
        if delta != 0.0 && self.commit_pending() {
            self.step(index, if delta < 0.0 { 1 } else { -1 });
        }
        true
    }
    /// 【设置窗口】【滚动几何】无参数，返回溢出内容的可见轨道，短列表不占用滚动条
    pub fn scrollbar(&self) -> Option<scrollbar::Geometry> {
        if self.toolbar_page() || self.translation_page() {
            return None;
        }
        let layout = super::layout::Layout::new(self);
        let page = layout.viewport.height() as usize;
        scrollbar::geometry(layout.viewport, layout.maximum + page, page, self.scroll)
    }

    /// 【设置窗口】【滚动条按下】按 point 开始滑块拖动或轨道翻页，返回是否消费；不激活被滚动条覆盖的字段
    pub fn press_scrollbar(&mut self, point: (f64, f64)) -> bool {
        let Some(bar) = self.scrollbar() else {
            return false;
        };
        if !super::window::contains(bar.track, point) {
            return false;
        }
        if super::window::contains(bar.handle, point) {
            self.scrollbar_drag = Some(scrollbar::Drag::new(point, &bar));
        } else if point.1 < f64::from(bar.handle.top()) {
            self.scroll = self.scroll.saturating_sub(bar.page);
        } else {
            self.scroll = (self.scroll + bar.page).min(bar.maximum);
        }
        self.dropdown = None;
        true
    }

    /// 【设置窗口】【列表滚动】按 delta 调整滚轮步数并限制首尾，保留未提交值，返回是否改变位置
    pub fn scroll_rows(&mut self, delta: isize) -> bool {
        let next = self
            .scroll
            .saturating_add_signed(delta * 64)
            .min(super::layout::Layout::new(self).maximum);
        let changed = next != self.scroll;
        self.scroll = next;
        self.dropdown = None;
        changed
    }

    /// 【设置窗口】【键盘翻页】按 key 滚动内容，返回是否识别该键；文字字段由调用方优先消费
    pub fn scroll_key(&mut self, key: Option<Key>) -> bool {
        let layout = super::layout::Layout::new(self);
        let page = layout.viewport.height() as usize;
        self.scroll = match key {
            Some(Key::PageDown) => (self.scroll + page).min(layout.maximum),
            Some(Key::PageUp) => self.scroll.saturating_sub(page),
            Some(Key::Home) => 0,
            Some(Key::End) => layout.maximum,
            _ => return false,
        };
        self.dropdown = None;
        true
    }

    /// 【设置窗口】【焦点可见】按 index 将键盘焦点所在字段完整移入视口，无返回值
    pub fn reveal_row(&mut self, index: usize) {
        let layout = super::layout::Layout::new(self);
        if let Some(row) = layout.rows.iter().find(|row| row.index == index) {
            let delta = if row.bounds.top() < layout.viewport.top() {
                row.bounds.top() - layout.viewport.top()
            } else {
                (row.bounds.bottom() - layout.viewport.bottom()).max(0.0)
            };
            self.scroll = self
                .scroll
                .saturating_add_signed(delta.ceil() as isize)
                .min(layout.maximum);
        }
    }
}
