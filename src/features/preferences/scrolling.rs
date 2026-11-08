//! 【设置窗口】【列表滚动】复用历史列表滚动条，滚轮、拖动和翻页保持相同边界

use super::window::Preferences;
use crate::{keys::Key, ui::scrollbar};
use tiny_skia::Rect;

impl Preferences {
    /// 【设置窗口】【滚动几何】无参数，返回溢出内容的可见轨道，短列表不占用滚动条
    pub fn scrollbar(&self) -> Option<scrollbar::Geometry> {
        if self.toolbar_page() {
            return None;
        }
        let viewport = Rect::from_xywh(
            self.size.0 as f32 - 20.0,
            146.0,
            12.0,
            (self.capacity() as f32 * 68.0 - 2.0).max(24.0),
        )
        .unwrap();
        scrollbar::geometry(viewport, self.visible().len(), self.capacity(), self.scroll)
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
        self.choices = None;
        true
    }

    /// 【设置窗口】【列表滚动】按 delta 调整行数并限制首尾，保留尚未提交的编辑值，返回是否改变位置
    pub fn scroll_rows(&mut self, delta: isize) -> bool {
        let next = self
            .scroll
            .saturating_add_signed(delta)
            .min(self.visible().len().saturating_sub(self.capacity()));
        let changed = next != self.scroll;
        self.scroll = next;
        self.choices = None;
        changed
    }

    /// 【设置窗口】【键盘翻页】按 key 滚动内容，返回是否识别该键；文字字段由调用方优先消费
    pub fn scroll_key(&mut self, key: Option<Key>) -> bool {
        let delta = match key {
            Some(Key::PageDown) => self.capacity() as isize,
            Some(Key::PageUp) => -(self.capacity() as isize),
            Some(Key::Home) => -(self.visible().len() as isize),
            Some(Key::End) => self.visible().len() as isize,
            _ => return false,
        };
        self.scroll_rows(delta);
        true
    }
}
