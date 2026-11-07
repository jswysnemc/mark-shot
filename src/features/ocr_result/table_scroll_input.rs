//! 【文字识别】【滚动条输入】滑块拖动和轨道重复各自拥有事务，非左键不触发表格选择

use super::{
    layout::contains,
    table_scroll_layout::{Axis, Bar, Part},
    table_scroll_model::Range,
    table_view::TableView,
};
use crate::backend::window::{LEFT_BUTTON, RIGHT_BUTTON, ScrollInput};
use std::time::{Duration, Instant};
use tiny_skia::Rect;

#[derive(Clone, Copy)]
struct Drag {
    axis: Axis,
    anchor: f32,
    button: u32,
}

#[derive(Clone, Copy)]
struct Repeat {
    axis: Axis,
    part: Part,
    target: usize,
    deadline: Instant,
}

#[derive(Default)]
pub(super) struct Interaction {
    pub hovered: Option<(Axis, Part)>,
    drag: Option<Drag>,
    repeat: Option<Repeat>,
    pointer: Option<(f64, f64)>,
}

impl Interaction {
    /// 【文字识别】【重复期限】返回按住箭头或轨道时的最近期限，未按住时无期限，不启动计时线程
    pub fn deadline(&self) -> Option<Instant> {
        self.repeat.map(|repeat| repeat.deadline)
    }

    /// 【文字识别】【按下外观】返回 axis、part 是否正处于按下事务，另一方向和部件不继承状态
    pub fn pressed(&self, axis: Axis, part: Part) -> bool {
        if part == Part::Slider {
            self.drag.is_some_and(|drag| drag.axis == axis)
        } else {
            self.repeat
                .is_some_and(|repeat| repeat.axis == axis && repeat.part == part)
        }
    }

    /// 【文字识别】【事务取消】清理拖动和重复期限，返回是否有按下事务；失焦、切页或缩放后不继续旧动作
    pub fn cancel(&mut self) -> bool {
        let changed = self.drag.take().is_some() | self.repeat.take().is_some();
        self.hovered = None;
        self.pointer = None;
        changed
    }

    /// 【文字识别】【按钮释放】按 button 释放所属事务，返回是否确实结束；右键不能结束正在执行的左键拖动
    pub fn release(&mut self, button: u32) -> bool {
        let mut changed = false;
        if self.drag.is_some_and(|drag| drag.button == button) {
            self.drag = None;
            changed = true;
        }
        if button == LEFT_BUTTON && self.repeat.is_some() {
            self.repeat = None;
            changed = true;
        }
        changed
    }
}

impl TableView {
    /// 【文字识别】【两方向范围】按 body 返回当前两方向逐项范围，正文与滚动条使用同一内容尺寸
    pub(super) fn scroll_ranges(&self, body: Rect) -> [Range; 2] {
        let grid = self.grid(body);
        [
            Range::horizontal(&grid),
            Range::vertical(&grid, self.table.rows.len()),
        ]
    }

    /// 【文字识别】【滚动值读取】按 ranges 将当前像素位置映射为两方向合法索引，不改变当前位置
    fn values(&self, ranges: &[Range; 2]) -> [usize; 2] {
        [
            ranges[0].index(self.scroll.0),
            ranges[1].index(self.scroll.1),
        ]
    }

    /// 【文字识别】【滚动值交付】按 ranges、values 更新正文像素位置，返回是否变化，表格数据与选择不改变
    fn set_values(&mut self, ranges: &[Range; 2], values: [usize; 2]) -> bool {
        let next = (ranges[0].offset(values[0]), ranges[1].offset(values[1]));
        let changed = self.scroll != next;
        self.scroll = next;
        changed
    }

    /// 【文字识别】【滚动位置恢复】按 body 限制窗口缩放后的索引和末端对齐，返回是否变化
    pub(super) fn normalize_scroll(&mut self, body: Rect) -> bool {
        let ranges = self.scroll_ranges(body);
        self.set_values(&ranges, self.values(&ranges))
    }

    /// 【文字识别】【完整滚轮】按 body、input 应用原版刻度累计，返回是否滚动；连续位移与刻度不重复使用
    pub(super) fn wheel(&mut self, body: Rect, input: ScrollInput) -> bool {
        let ranges = self.scroll_ranges(body);
        let previous = self.values(&ranges);
        let steps = self.wheel.steps(input, [&ranges[0], &ranges[1]]);
        let next = [
            ranges[0].moved(previous[0], steps[0]),
            ranges[1].moved(previous[1], steps[1]),
        ];
        for axis in 0..2 {
            if steps[axis] != 0 && previous[axis] == next[axis] {
                self.wheel.reset_axis(axis);
            }
        }
        self.set_values(&ranges, next)
    }

    /// 【文字识别】【滚动条读取】按 body 返回两方向可见滚动条，隐藏方向返回空值，部件几何对应当前索引
    pub(super) fn scroll_bars(&self, body: Rect) -> [Option<Bar>; 2] {
        let grid = self.grid(body);
        let ranges = self.scroll_ranges(body);
        let values = self.values(&ranges);
        [
            grid.horizontal_scroll
                .map(|bounds| Bar::new(Axis::Horizontal, bounds, &ranges[0], values[0])),
            grid.vertical_scroll
                .map(|bounds| Bar::new(Axis::Vertical, bounds, &ranges[1], values[1])),
        ]
    }

    /// 【文字识别】【滚动条按下】用 body、position、button、now 开始所属事务，返回是否命中滚动条；右键只保留控件身份，不执行左键动作
    pub(super) fn bar_press(
        &mut self,
        body: Rect,
        position: (f64, f64),
        button: u32,
        now: Instant,
    ) -> Option<bool> {
        let point = (
            position.0 - f64::from(body.x()),
            position.1 - f64::from(body.y()),
        );
        let bars = self.scroll_bars(body);
        let bar = bars
            .iter()
            .flatten()
            .find(|bar| contains(bar.bounds, point))?;
        if self.bars.drag.is_some() || self.bars.repeat.is_some() {
            return Some(false);
        }
        self.bars.pointer = Some(point);
        let part = bar.part(point);
        self.bars.hovered = part.map(|part| (bar.axis, part));
        if button == RIGHT_BUTTON || !matches!(button, LEFT_BUTTON | 0x112) {
            return Some(false);
        }
        let Some(part) = part else {
            return Some(false);
        };
        let ranges = self.scroll_ranges(body);
        let mut values = self.values(&ranges);
        let axis = bar.axis.index();
        let pointer = bar.axis.coordinate(point);
        let (slider_start, slider_length) = bar.axis.geometry(bar.slider);
        if part == Part::Slider || button == 0x112 {
            let anchor = if part == Part::Slider && button != 0x112 {
                pointer - slider_start
            } else {
                slider_length / 2.0
            };
            values[axis] = bar.value_at(pointer, anchor);
            self.bars.drag = Some(Drag {
                axis: bar.axis,
                anchor,
                button,
            });
        } else {
            let delta = match part {
                Part::PreviousLine => -1,
                Part::NextLine => 1,
                Part::PreviousPage => -(ranges[axis].page as i32),
                Part::NextPage => ranges[axis].page as i32,
                Part::Slider => 0,
            };
            let next = ranges[axis].moved(values[axis], delta);
            if next != values[axis] {
                values[axis] = next;
                self.bars.repeat = Some(Repeat {
                    axis: bar.axis,
                    part,
                    target: bar.value_at(pointer, slider_length / 2.0),
                    deadline: now + Duration::from_millis(500),
                });
            }
        }
        Some(self.set_values(&ranges, values))
    }

    /// 【文字识别】【滚动条移动】按 body、position 更新悬停或所属拖动，返回是否变化；移出窗口后拖动仍限制在轨道边缘
    pub(super) fn bar_motion(&mut self, body: Rect, position: (f64, f64)) -> bool {
        let point = (
            position.0 - f64::from(body.x()),
            position.1 - f64::from(body.y()),
        );
        self.bars.pointer = Some(point);
        let bars = self.scroll_bars(body);
        let hovered = bars
            .iter()
            .flatten()
            .find_map(|bar| bar.part(point).map(|part| (bar.axis, part)));
        let changed = self.bars.hovered != hovered;
        self.bars.hovered = hovered;
        let Some(drag) = self.bars.drag else {
            return changed;
        };
        let Some(bar) = &bars[drag.axis.index()] else {
            return self.bars.cancel() || changed;
        };
        let ranges = self.scroll_ranges(body);
        let mut values = self.values(&ranges);
        values[drag.axis.index()] = bar.value_at(drag.axis.coordinate(point), drag.anchor);
        let moved = self.set_values(&ranges, values);
        let hovered = self
            .scroll_bars(body)
            .iter()
            .flatten()
            .find_map(|bar| bar.part(point).map(|part| (bar.axis, part)));
        let changed = changed || self.bars.hovered != hovered;
        self.bars.hovered = hovered;
        moved || changed
    }

    /// 【文字识别】【轨道重复】按 body、now 执行确实到期的一次重复，返回是否变化；滑块到达点击目标后取消期限
    pub(super) fn bar_wake(&mut self, body: Rect, now: Instant) -> bool {
        let Some(mut repeat) = self.bars.repeat.filter(|repeat| repeat.deadline <= now) else {
            return false;
        };
        let ranges = self.scroll_ranges(body);
        let mut values = self.values(&ranges);
        let axis = repeat.axis.index();
        if matches!(repeat.part, Part::PreviousLine | Part::NextLine)
            && self.bars.hovered != Some((repeat.axis, repeat.part))
        {
            repeat.deadline = now + Duration::from_millis(50);
            self.bars.repeat = Some(repeat);
            return false;
        }
        let delta = match repeat.part {
            Part::PreviousLine => -1,
            Part::NextLine => 1,
            Part::PreviousPage => -(ranges[axis].page as i32),
            Part::NextPage => ranges[axis].page as i32,
            Part::Slider => 0,
        };
        let mut next = ranges[axis].moved(values[axis], delta);
        if matches!(repeat.part, Part::PreviousPage | Part::NextPage)
            && (delta > 0 && next >= repeat.target || delta < 0 && next <= repeat.target)
        {
            next = repeat.target;
            self.bars.repeat = None;
        } else if next == values[axis] {
            self.bars.repeat = None;
        } else {
            repeat.deadline = now + Duration::from_millis(50);
            self.bars.repeat = Some(repeat);
        }
        values[axis] = next;
        self.set_values(&ranges, values)
    }
}

impl super::window::ResultWindow {
    /// 【文字识别】【窗口滚动入口】按 input 向表格交付完整刻度，菜单抓取时不滚动，其他页面使用原事件，返回实际窗口动作
    pub(super) fn scroll_input(
        &mut self,
        input: ScrollInput,
    ) -> crate::backend::window::WindowAction {
        use crate::backend::window::{WindowAction, WindowEvent};
        if self.source_menu.is_some() || self.translation.language_menu.is_some() {
            return WindowAction::None;
        }
        if self.view == super::window::View::Table {
            if !contains(self.layout.table_body, input.position) {
                return WindowAction::None;
            }
            return if self
                .table
                .as_mut()
                .is_some_and(|table| table.wheel(self.layout.table_body, input))
            {
                WindowAction::Redraw
            } else {
                WindowAction::None
            };
        }
        self.input_event(WindowEvent::Scroll {
            position: input.position,
            delta: input.delta,
        })
    }
}

#[cfg(test)]
mod tests;
