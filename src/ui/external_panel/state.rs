//! 【外部动作面板】【输入状态】只接受左键完整点击，滚轮和键盘停留在列表内，不修改画布

use super::{
    icons::IconCache,
    layout::{Layout, ROW},
};
use crate::{
    features::external_actions::Action, keys::Key, types::MouseButton, ui::panel::UiPanel,
    ui::settings_panel::SettingsWidget,
};
use tiny_skia::{Pixmap, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    OpenWith,
    Extensions,
}

impl Kind {
    /// 【外部动作面板】【标题选择】返回 kind 对应原版标题，无参数
    pub fn title(self) -> &'static str {
        match self {
            Self::OpenWith => "Open with",
            Self::Extensions => "Extensions",
        }
    }
}

#[derive(Default)]
pub struct ExternalPanel {
    pub kind: Option<Kind>,
    pub actions: Vec<Action>,
    pub message: String,
    pub layout: Option<Layout>,
    pub render_pos: (f32, f32),
    pub monitor_idx: usize,
    pub offset: f32,
    pub active: Option<usize>,
    pub hovered: Option<usize>,
    pub dirty: bool,
    pub pixmap: Option<Pixmap>,
    pub(super) icons: IconCache,
    pressed: Option<usize>,
    scrollbar_drag: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    None,
    Changed,
    Close,
    Activate(usize),
}

impl ExternalPanel {
    /// 【外部动作面板】【打开状态】按 kind、actions、message、layout、position 与 monitor 返回无悬停和点击记录的面板
    pub fn open(
        kind: Kind,
        actions: Vec<Action>,
        message: String,
        layout: Layout,
        position: (f32, f32),
        monitor: usize,
    ) -> Self {
        Self {
            kind: Some(kind),
            actions,
            message,
            layout: Some(layout),
            render_pos: position,
            monitor_idx: monitor,
            dirty: true,
            ..Self::default()
        }
    }
    /// 【外部动作面板】【范围检查】返回 monitor、本地 point 是否落入面板，不包含不可见面板
    pub fn contains(&self, monitor: usize, point: (f64, f64)) -> bool {
        monitor == self.monitor_idx
            && self.rect().is_some_and(|rect| {
                crate::ui::toolbar::contains(rect, (point.0 as f32, point.1 as f32))
            })
    }

    /// 【外部动作面板】【行命中】根据 monitor 与 point 返回可见列表行，空白及滚动条不作为动作
    pub fn hit(&self, monitor: usize, point: (f64, f64)) -> Option<usize> {
        if !self.contains(monitor, point) {
            return None;
        }
        let layout = self.layout?;
        let point = (
            point.0 as f32 - self.render_pos.0,
            point.1 as f32 - self.render_pos.1,
        );
        let mut list = layout.list;
        if self.max_offset() > 0.0 {
            list = Rect::from_xywh(list.left(), list.top(), list.width() - 12.0, list.height())?;
        }
        if !crate::ui::toolbar::contains(list, point) {
            return None;
        }
        let index = ((point.1 - list.top() + self.offset) / ROW).floor() as usize;
        (index < self.actions.len()).then_some(index)
    }

    /// 【外部动作面板】【滚动上限】返回正文高度超出可见列表的像素数量，无参数
    pub fn max_offset(&self) -> f32 {
        self.layout.map_or(0.0, |layout| {
            (self.actions.len() as f32 * ROW - layout.list.height()).max(0.0)
        })
    }

    /// 【外部动作面板】【滑块几何】返回可滚动列表的本地滑块，全部内容可见时为空，无参数
    pub fn thumb(&self) -> Option<Rect> {
        let layout = self.layout?;
        let maximum = self.max_offset();
        if maximum <= 0.0 {
            return None;
        }
        let height = (layout.list.height().powi(2) / (self.actions.len() as f32 * ROW))
            .max(22.0)
            .min(layout.list.height());
        Rect::from_xywh(
            layout.list.right() - 10.0,
            layout.list.top() + self.offset / maximum * (layout.list.height() - height),
            8.0,
            height,
        )
    }

    /// 【外部动作面板】【指针移动】按 monitor、point 更新悬停或已开始的滑块拖动，返回重绘事件
    pub fn pointer(&mut self, monitor: usize, point: (f64, f64)) -> Event {
        if let Some(grab) = self.scrollbar_drag
            && monitor == self.monitor_idx
            && let (Some(layout), Some(thumb)) = (self.layout, self.thumb())
        {
            let distance = layout.list.height() - thumb.height();
            if distance > 0.0 {
                return self.scroll_to(
                    (point.1 as f32 - self.render_pos.1 - layout.list.top() - grab) / distance
                        * self.max_offset(),
                );
            }
        }
        let hovered = self.hit(monitor, point);
        if self.hovered == hovered {
            Event::None
        } else {
            self.hovered = hovered;
            self.dirty = true;
            Event::Changed
        }
    }

    /// 【外部动作面板】【按钮输入】按 monitor、point、button 和 pressed 返回动作；左右键职责隔离，列表仅左键释放交付
    pub fn button(
        &mut self,
        monitor: usize,
        point: (f64, f64),
        button: MouseButton,
        pressed: bool,
    ) -> Event {
        if !matches!(button, MouseButton::Left) {
            return Event::None;
        }
        if pressed && !self.contains(monitor, point) {
            return Event::Close;
        }
        let local = (
            point.0 as f32 - self.render_pos.0,
            point.1 as f32 - self.render_pos.1,
        );
        if pressed
            && let (Some(thumb), Some(layout)) = (self.thumb(), self.layout)
            && local.0 >= layout.list.right() - 12.0
            && crate::ui::toolbar::contains(layout.list, local)
        {
            self.pressed = None;
            if crate::ui::toolbar::contains(thumb, local) {
                self.scrollbar_drag = Some(local.1 - thumb.top());
                return Event::None;
            }
            return self.scroll_to(
                self.offset
                    + if local.1 < thumb.top() {
                        -layout.list.height()
                    } else {
                        layout.list.height()
                    },
            );
        }
        if !pressed && self.scrollbar_drag.take().is_some() {
            return Event::None;
        }
        let hit = self.hit(monitor, point);
        if pressed {
            self.pressed = hit;
            return Event::None;
        }
        let armed = self.pressed.take();
        if let Some(index) = hit.filter(|index| Some(*index) == armed) {
            self.active = Some(index);
            Event::Activate(index)
        } else {
            Event::None
        }
    }

    /// 【外部动作面板】【滚动更新】把 offset 限制到正文范围，返回是否需要重绘；滚动取消未完成的行点击
    pub fn scroll_to(&mut self, offset: f32) -> Event {
        if !offset.is_finite() {
            return Event::None;
        }
        let offset = offset.clamp(0.0, self.max_offset());
        if self.offset == offset {
            return Event::None;
        }
        self.offset = offset;
        self.pressed = None;
        self.hovered = None;
        self.dirty = true;
        Event::Changed
    }

    /// 【外部动作面板】【键盘列表】按 key 导航、关闭或执行当前行，返回事件；空列表不触发动作
    pub fn key(&mut self, key: Key) -> Event {
        if key == Key::Escape {
            return Event::Close;
        }
        if matches!(key, Key::Enter | Key::Space) {
            return self.active.map_or(Event::None, Event::Activate);
        }
        if self.actions.is_empty() {
            return Event::None;
        }
        let current = self.active.unwrap_or(0);
        let page = self
            .layout
            .map_or(1, |layout| (layout.list.height() / ROW).floor() as usize)
            .max(1);
        let last = self.actions.len() - 1;
        let next = match key {
            Key::Down => self.active.map_or(0, |_| (current + 1).min(last)),
            Key::Up => self.active.map_or(last, |_| current.saturating_sub(1)),
            Key::Home => 0,
            Key::End => last,
            Key::PageDown => (current + page).min(last),
            Key::PageUp => current.saturating_sub(page),
            _ => return Event::None,
        };
        self.active = Some(next);
        if let Some(layout) = self.layout {
            let top = next as f32 * ROW;
            if top < self.offset {
                self.scroll_to(top);
            } else if top + ROW > self.offset + layout.list.height() {
                self.scroll_to(top + ROW - layout.list.height());
            }
        }
        self.dirty = true;
        Event::Changed
    }
}

impl UiPanel for ExternalPanel {
    type Item = SettingsWidget;
    /// 【外部动作面板】【位置读取】返回显示器本地绘制位置，无参数
    fn render_pos(&self) -> (f32, f32) {
        self.render_pos
    }
    /// 【外部动作面板】【尺寸读取】返回当前布局尺寸，关闭时返回零，无参数
    fn size(&self) -> (f32, f32) {
        self.layout.map_or((0.0, 0.0), |layout| layout.size)
    }
    /// 【外部动作面板】【通用项目】返回空控件切片，实际动作列表使用独立行模型，无参数
    fn items(&self) -> &[Self::Item] {
        &[]
    }
    /// 【外部动作面板】【边距读取】返回原版八像素面板内边距，无参数
    fn padding(&self) -> f32 {
        8.0
    }
    /// 【外部动作面板】【输出读取】返回所属显示器编号，无参数
    fn monitor_idx(&self) -> usize {
        self.monitor_idx
    }
    /// 【外部动作面板】【损坏标记】要求下一帧重绘面板，无参数、无返回值
    fn set_dirty(&mut self) {
        self.dirty = true;
    }
    /// 【外部动作面板】【损坏读取】返回当前面板是否需要重绘，无参数
    fn is_dirty(&self) -> bool {
        self.dirty
    }
    /// 【外部动作面板】【显隐读取】返回是否已打开并具有有效布局，无参数
    fn is_visible(&self) -> bool {
        self.kind.is_some() && self.layout.is_some()
    }
    /// 【外部动作面板】【绘制矩形】返回当前可见面板范围，关闭时为空，无参数
    fn rect(&self) -> Option<Rect> {
        self.is_visible()
            .then(|| {
                Rect::from_xywh(
                    self.render_pos.0,
                    self.render_pos.1,
                    self.size().0,
                    self.size().1,
                )
            })
            .flatten()
    }
}
