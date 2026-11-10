//! 【截图工具栏】【排列几何】横纵排列和窄屏换行共用同一套命中矩形

use super::{Toolbar, ToolbarItem};
use tiny_skia::Rect;

pub const GRIP_THICKNESS: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Toolbar {
    /// 【截图工具栏】【边距】返回主、副面板各自的原版内边距
    pub fn margin(&self) -> f32 {
        if self.secondary { 4.0 } else { 6.0 }
    }

    /// 【截图工具栏】【按钮尺寸】返回实际排版的逻辑边长，初始化前使用配置尺寸
    pub fn button_size(&self) -> f32 {
        self.items
            .iter()
            .zip(&self.cells)
            .find_map(|(item, rect)| {
                matches!(item, ToolbarItem::Button(_)).then_some(rect.width().min(rect.height()))
            })
            .unwrap_or(super::metrics::button(self.secondary))
    }

    /// 【截图工具栏】【布局更新】按 available 的窗口尺寸排版；返回尺寸或方向是否变化
    pub fn arrange(&mut self, available: (f32, f32)) -> bool {
        let old = (self.size, self.cells.clone(), self.orientation);
        let margin = self.margin();
        let requested = super::metrics::button(self.secondary);
        let gap = crate::config::get().toolbar.button_gap;
        let vertical = self.orientation == Orientation::Vertical;
        let limit = (if vertical { available.1 } else { available.0 } - margin * 2.0)
            .max(requested + GRIP_THICKNESS);
        // 1. 【截图工具栏】【紧凑适配】轻微溢出时缩小到至少二十六像素，避免只多一个工具便遮挡整行画布
        let buttons = self
            .items
            .iter()
            .filter(|item| matches!(item, ToolbarItem::Button(_)))
            .count()
            .max(1);
        let separators = self
            .items
            .iter()
            .filter(|item| matches!(item, ToolbarItem::Seperator))
            .count();
        let fitted = (limit - GRIP_THICKNESS - self.items.len() as f32 * gap - separators as f32)
            / buttons as f32;
        let button = if fitted >= (super::metrics::icon() + 4.0).max(26.0) {
            requested.min(fitted)
        } else {
            requested
        };
        let mut along = GRIP_THICKNESS + gap;
        let mut across = 0.0_f32;
        let mut longest = along;
        self.cells.clear();
        for item in &self.items {
            let length = if matches!(item, ToolbarItem::Seperator) {
                1.0
            } else {
                button
            };
            if along + length > limit && along > GRIP_THICKNESS + gap {
                longest = longest.max(along - gap);
                along = 0.0;
                across += button + gap;
            }
            let (x, y, w, h) = if vertical {
                (margin + across, margin + along, button, length)
            } else {
                (margin + along, margin + across, length, button)
            };
            self.cells.push(Rect::from_xywh(x, y, w, h).unwrap());
            along += length + gap;
        }
        longest = longest.max(along - gap);
        self.size = if vertical {
            (across + button + margin * 2.0, longest + margin * 2.0)
        } else {
            (longest + margin * 2.0, across + button + margin * 2.0)
        };
        let changed = old != (self.size, self.cells.clone(), self.orientation);
        self.dirty |= changed;
        changed
    }

    /// 【截图工具栏】【按钮区域】返回 index 对应的面板本地矩形，供绘制和输入共用
    pub fn cell(&self, index: usize) -> Option<Rect> {
        self.cells.get(index).copied()
    }

    /// 【截图工具栏】【握柄区域】返回首位握柄的面板本地矩形
    pub fn grip(&self) -> Rect {
        let margin = self.margin();
        let (w, h) = if self.orientation == Orientation::Vertical {
            (self.button_size(), GRIP_THICKNESS)
        } else {
            (GRIP_THICKNESS, self.button_size())
        };
        Rect::from_xywh(margin, margin, w, h).unwrap()
    }
}

/// 【截图工具栏】【矩形命中】检查 point 是否位于 rect 内；右下边界不重复命中相邻项目
pub fn contains(rect: Rect, point: (f32, f32)) -> bool {
    point.0 >= rect.left()
        && point.0 < rect.right()
        && point.1 >= rect.top()
        && point.1 < rect.bottom()
}
