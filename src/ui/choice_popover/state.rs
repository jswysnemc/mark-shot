//! 【选择面板】【交互状态】管理选择、滚动、键盘导航和按需透明度动画

use std::time::{Duration, Instant};

use tiny_skia::{Pixmap, Rect};

use super::ChoiceLayout;
use crate::interaction::ScrollAccumulator;
use crate::keys::Key;
use crate::theme::anim;
use crate::ui::panel::{AnimatedPanel, HoverablePanel, PanelItem, UiPanel};

#[derive(Clone, Copy, Debug)]
pub struct ChoiceOption {
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChoiceEvent {
    Unchanged,
    Changed,
    Selected(usize),
    Close,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct ChoiceLook {
    theme: crate::theme::Mode,
    active: Option<usize>,
    hovered: Option<usize>,
    first_row: usize,
    layout: ChoiceLayout,
}

pub struct ChoicePopover {
    pub title: &'static str,
    pub(super) options: Vec<ChoiceOption>,
    pub layout: ChoiceLayout,
    pub first_row: usize,
    pub active: Option<usize>,
    pub hovered: Option<usize>,
    pub position: (f32, f32),
    pub render_pos: (f32, f32),
    pub monitor_idx: usize,
    pub open: bool,
    pub opacity: f32,
    pub dirty: bool,
    pub pixmap: Option<Pixmap>,
    pub(super) drawn_look: Option<ChoiceLook>,
    last_tick: Option<Instant>,
    scroll: ScrollAccumulator<u8>,
}

impl ChoicePopover {
    /// 【选择面板】【状态构造】使用 title、options 初始化关闭状态；返回面板，不生成图形路径或加载字体
    pub fn new(title: &'static str, options: Vec<ChoiceOption>) -> Self {
        let layout = ChoiceLayout::new(options.len(), (f32::MAX, f32::MAX));
        Self {
            title,
            options,
            layout,
            first_row: 0,
            active: None,
            hovered: None,
            position: (0.0, 0.0),
            render_pos: (0.0, 0.0),
            monitor_idx: 0,
            open: false,
            opacity: 0.0,
            dirty: true,
            pixmap: None,
            drawn_look: None,
            last_tick: None,
            scroll: ScrollAccumulator::default(),
        }
    }

    /// 【选择面板】【空间适配】依据 max_size 重新布局并约束滚动位置；返回布局是否变化
    pub fn resize(&mut self, max_size: (f32, f32)) -> bool {
        let layout = ChoiceLayout::new(self.options.len(), max_size);
        if layout == self.layout {
            return false;
        }
        self.layout = layout;
        self.first_row = self
            .first_row
            .min(self.layout.rows.saturating_sub(self.layout.visible_rows));
        if let Some(index) = self.hovered.or(self.active) {
            self.ensure_visible(index);
        }
        self.dirty = true;
        true
    }

    /// 【选择面板】【面板命中】判断显示器本地 local 是否位于可见面板内；返回布尔值
    pub fn hit_test(&self, local: (f64, f64)) -> bool {
        self.rect().is_some_and(|rect| {
            local.0 >= rect.left() as f64
                && local.0 <= rect.right() as f64
                && local.1 >= rect.top() as f64
                && local.1 <= rect.bottom() as f64
        })
    }

    /// 【选择面板】【索引命中】将显示器本地 local 映射到可见条目；返回索引或空值
    pub fn item_at(&self, local: (f64, f64)) -> Option<usize> {
        self.rect()?;
        self.layout.item_at(
            (
                local.0 as f32 - self.render_pos.0,
                local.1 as f32 - self.render_pos.1,
            ),
            self.first_row,
        )
    }

    /// 【选择面板】【滚动行数】按有符号 rows 调整可见首行；返回滚动是否变化，不改变实际选择
    pub fn scroll_rows(&mut self, rows: i32) -> bool {
        let limit = self.layout.rows.saturating_sub(self.layout.visible_rows);
        let next = self
            .first_row
            .saturating_add_signed(rows as isize)
            .min(limit);
        if next == self.first_row {
            return false;
        }
        self.first_row = next;
        self.hovered = None;
        self.dirty = true;
        true
    }

    /// 【选择面板】【滚动累积】累计以行数为单位的 delta 并限制重复输入；返回可见行是否变化
    pub fn scroll(&mut self, delta: f32) -> bool {
        let rows = self.scroll.step(0, delta);
        self.scroll_rows(rows)
    }

    /// 【选择面板】【滚动取消】清除累计滚轮输入，避免重新打开后继承旧滚动；无返回值
    pub fn cancel_scroll(&mut self) {
        self.scroll.cancel();
    }

    /// 【选择面板】【选择可见】滚动到 index 所在行；越界索引不处理，无返回值
    pub fn ensure_visible(&mut self, index: usize) {
        if index >= self.options.len() {
            return;
        }
        let row = index / self.layout.columns;
        let next = if row < self.first_row {
            row
        } else if row >= self.first_row + self.layout.visible_rows {
            row + 1 - self.layout.visible_rows
        } else {
            self.first_row
        };
        if self.first_row != next {
            self.first_row = next;
            self.dirty = true;
        }
    }

    /// 【选择面板】【键盘导航】处理 key 与 shift 的导航、选择或关闭；返回面板事件，不支持的按键返回空值
    pub fn key(&mut self, key: Key, shift: bool) -> Option<ChoiceEvent> {
        if key == Key::Escape {
            return Some(ChoiceEvent::Close);
        }
        let last = self.options.len().checked_sub(1)?;
        let index = self.hovered.or(self.active).unwrap_or(0).min(last);
        let next = match key {
            Key::Enter | Key::Space => return Some(ChoiceEvent::Selected(index)),
            Key::Left => index.saturating_sub(1),
            Key::Right => (index + 1).min(last),
            Key::Up => {
                if index >= self.layout.columns {
                    index - self.layout.columns
                } else {
                    index
                }
            }
            Key::Down => {
                if index / self.layout.columns + 1 < self.layout.rows {
                    (index + self.layout.columns).min(last)
                } else {
                    index
                }
            }
            Key::Home => 0,
            Key::End => last,
            Key::Tab if shift => {
                if index == 0 {
                    last
                } else {
                    index - 1
                }
            }
            Key::Tab => {
                if index == last {
                    0
                } else {
                    index + 1
                }
            }
            _ => return None,
        };
        self.hovered = Some(next);
        self.ensure_visible(next);
        self.dirty = true;
        Some(if next == index {
            ChoiceEvent::Unchanged
        } else {
            ChoiceEvent::Changed
        })
    }

    /// 【选择面板】【绘制快照】返回影响网格内容的状态，不包含位置或透明度
    pub(super) fn look(&self) -> ChoiceLook {
        ChoiceLook {
            theme: crate::theme::Mode::current(),
            active: self.active,
            hovered: self.hovered,
            first_row: self.first_row,
            layout: self.layout,
        }
    }
}

impl PanelItem for ChoiceOption {
    /// 【选择面板】【条目尺寸】返回配置中的网格单元逻辑尺寸
    fn size(&self) -> f32 {
        crate::config::get().choice_picker.cell_size
    }
    /// 【选择面板】【条目间距】返回配置中的网格留白
    fn trailing_padding(&self) -> f32 {
        crate::config::get().choice_picker.gap
    }
    /// 【选择面板】【条目按钮】返回条目支持点击交互
    fn is_button(&self) -> bool {
        true
    }
}

impl UiPanel for ChoicePopover {
    type Item = ChoiceOption;
    /// 【选择面板】【面板位置】返回实际绘制的显示器本地位置
    fn render_pos(&self) -> (f32, f32) {
        self.render_pos
    }
    /// 【选择面板】【面板尺寸】返回当前布局尺寸
    fn size(&self) -> (f32, f32) {
        self.layout.size
    }
    /// 【选择面板】【面板条目】返回只读选择项
    fn items(&self) -> &[Self::Item] {
        &self.options
    }
    /// 【选择面板】【面板留白】返回网格外围留白
    fn padding(&self) -> f32 {
        self.layout.padding
    }
    /// 【选择面板】【显示器索引】返回面板所属显示器
    fn monitor_idx(&self) -> usize {
        self.monitor_idx
    }
    /// 【选择面板】【重绘标记】标记内容需要更新；无返回值
    fn set_dirty(&mut self) {
        self.dirty = true;
    }
    /// 【选择面板】【重绘判断】返回内容是否需要更新
    fn is_dirty(&self) -> bool {
        self.dirty
    }
    /// 【选择面板】【可见判断】返回面板是否打开或仍在消失动画中
    fn is_visible(&self) -> bool {
        self.open || self.opacity > 0.0
    }
    /// 【选择面板】【可见范围】返回可见面板矩形；完全消失时返回空值
    fn rect(&self) -> Option<Rect> {
        (self.opacity > 0.0)
            .then(|| {
                Rect::from_xywh(
                    self.render_pos.0,
                    self.render_pos.1,
                    self.layout.size.0,
                    self.layout.size.1,
                )
            })
            .flatten()
    }
}

impl HoverablePanel for ChoicePopover {
    type Hover = Option<usize>;
    /// 【选择面板】【悬停读取】返回当前悬停条目
    fn hovered(&self) -> Self::Hover {
        self.hovered
    }
    /// 【选择面板】【悬停设置】用 hover 更新悬停条目；无返回值
    fn set_hovered(&mut self, hover: Self::Hover) {
        self.hovered = hover;
    }
}

impl AnimatedPanel for ChoicePopover {
    /// 【选择面板】【动画时钟】返回最后刷新时刻
    fn last_tick(&self) -> Option<Instant> {
        self.last_tick
    }
    /// 【选择面板】【动画时钟设置】保存 at 刷新时刻；无返回值
    fn set_last_tick(&mut self, at: Option<Instant>) {
        self.last_tick = at;
    }
    /// 【选择面板】【动画间隔】返回共用动画间隔
    fn anim_interval(&self) -> Duration {
        anim::frame()
    }
    /// 【选择面板】【动画步长】返回共用动画时间步长
    fn anim_dt(&self) -> f32 {
        anim::dt()
    }
    /// 【选择面板】【动画判断】返回透明度是否尚未到达打开或关闭目标
    fn is_animating(&self) -> bool {
        ((if self.open { 1.0 } else { 0.0 }) - self.opacity).abs() > anim::OPACITY_EPSILON
    }
    /// 【选择面板】【动画更新】按 dt 推进透明度；返回是否需要重绘
    fn animate_step(&mut self, dt: f32) -> bool {
        let target = if self.open { 1.0 } else { 0.0 };
        let difference = target - self.opacity;
        if difference.abs() <= anim::OPACITY_EPSILON {
            return false;
        }
        let cfg = &crate::config::get().animation;
        self.opacity +=
            difference.signum() * (cfg.popover_fade * cfg.speed * dt).min(difference.abs());
        true
    }
}
