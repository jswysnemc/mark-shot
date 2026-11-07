//! 【文字识别】【标题布局】单行标题栏保留原版控件位置，窄窗口只裁切页签

use tiny_skia::Rect;

use super::{labels, layout::contains, window::View};
use crate::ui::language::Language;

#[cfg(test)]
mod tests;

pub(super) struct Titlebar {
    pub language: Language,
    pub bounds: Rect,
    pub icon: Option<Rect>,
    pub viewport: Rect,
    pub tabs: Vec<(View, Rect)>,
    pub copy: Rect,
    // 1. 【文字识别】【操作槽位】按翻译、更多、置顶、关闭保留几何，内容层只启用已经接入的入口
    pub slots: [Option<Rect>; 4],
}

/// 【文字识别】【整数矩形】按 x、y、width、height 创建有效逻辑矩形，返回矩形，不改变整数几何
pub(super) fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(x, y, width.max(1.0), height.max(1.0)).unwrap()
}

impl Titlebar {
    /// 【文字识别】【标题排列】用 width、可选页签、view 和 copy_label 计算原版位置，返回标题布局
    #[cfg(test)]
    pub fn new(width: f32, has_table: bool, has_image: bool, view: View, copy_label: &str) -> Self {
        Self::localized(
            width,
            has_table,
            has_image,
            view,
            copy_label,
            Language::English,
        )
    }

    /// 【文字识别】【语言标题】按 width、可选页签、view、copy_label 和 language 返回原版语言对应几何，文案与宽度同步
    pub fn localized(
        width: f32,
        has_table: bool,
        has_image: bool,
        view: View,
        copy_label: &str,
        language: Language,
    ) -> Self {
        let copy_label = language.text(copy_label);
        let bounds = rect(8.0, 6.0, width - 16.0, 27.0);
        // 1. 【文字识别】【操作排列】从右向左固定操作位置，非文字页只省略翻译槽位
        let mut slots = [None; 4];
        let mut right = bounds.right();
        for index in (0..4).rev() {
            if index == 0 && view != View::Text {
                continue;
            }
            slots[index] = Some(rect(right - 26.0, 6.0, 26.0, 26.0));
            right -= 32.0;
        }
        let copy = rect(
            right - labels::copy_width(copy_label),
            6.0,
            labels::copy_width(copy_label),
            26.0,
        );
        let icon = (width >= 500.0).then(|| rect(8.0, 10.0, 18.0, 18.0));
        let left = if icon.is_some() { 32.0 } else { 8.0 };
        // 2. 【文字识别】【页签裁切】保留各页签原始宽度，视区不足时不重新排列或换行
        let mut x = left;
        let tabs = [View::Text, View::Table, View::Image]
            .into_iter()
            .filter(|view| match view {
                View::Text => true,
                View::Table => has_table,
                View::Image => has_image,
            })
            .map(|view| {
                let width = labels::localized_tab_width(view, language);
                let item = (view, rect(x, 6.0, width, 27.0));
                x += width;
                item
            })
            .collect();
        Self {
            language,
            bounds,
            icon,
            viewport: rect(left, 6.0, (x - left).min(copy.left() - 6.0 - left), 27.0),
            tabs,
            copy,
            slots,
        }
    }

    /// 【文字识别】【裁切命中】按 position 返回可见页签，超出 viewport 的原始页签部分不接收点击
    pub fn tab_at(&self, position: (f64, f64)) -> Option<View> {
        if !contains(self.viewport, position) {
            return None;
        }
        self.tabs
            .iter()
            .find_map(|(view, rect)| contains(*rect, position).then_some(*view))
    }
}
