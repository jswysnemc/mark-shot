//! 【上下文菜单】【输入状态】菜单关闭前不交付动作，禁用行不高亮，外部点击不穿透

use super::layout::{self, Layout};
use crate::{keys::Key, types::MouseButton};
use cosmic_text::FontSystem;
use tiny_skia::{Pixmap, Rect};

#[derive(Clone, Copy)]
pub struct Row {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub enabled: bool,
}

impl Row {
    /// 【上下文菜单】【分隔判断】返回当前行是否为分隔线，空标题不提供交互
    pub fn separator(&self) -> bool {
        self.label.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    None,
    Changed,
    Close,
    Activate(usize),
}

pub struct ContextMenu {
    pub id: u64,
    pub native: bool,
    pub anchor: (f32, f32),
    pub rows: Vec<Row>,
    pub layout: Layout,
    pub position: (f32, f32),
    pub monitor_idx: usize,
    pub active: Option<usize>,
    pub dirty: bool,
    pub pixmap: Option<Pixmap>,
    pub drawn_theme: Option<crate::theme::Mode>,
    pub submenu_rows: Vec<usize>,
    pub control_anchor: Option<Rect>,
    left_pressed: bool,
}

impl ContextMenu {
    /// 【上下文菜单】【菜单构造】按 rows、fonts、anchor、bounds 在 monitor_idx 打开菜单，返回未高亮状态
    pub fn new(
        rows: Vec<Row>,
        fonts: &mut FontSystem,
        anchor: (f32, f32),
        bounds: (f32, f32),
        monitor_idx: usize,
    ) -> Self {
        let layout = Layout::new(&rows, fonts);
        let position = layout::position(anchor, layout.size, bounds);
        Self {
            id: 0,
            native: false,
            anchor,
            rows,
            layout,
            position,
            monitor_idx,
            active: None,
            dirty: true,
            pixmap: None,
            drawn_theme: None,
            submenu_rows: Vec::new(),
            control_anchor: None,
            left_pressed: false,
        }
    }

    /// 【上下文菜单】【子菜单标记】用 index 和 fonts 为可用行保留箭头列，无返回值；同一行不重复标记
    pub fn submenu(&mut self, index: usize, fonts: &mut FontSystem) {
        if index < self.rows.len() && !self.submenu_rows.contains(&index) {
            self.submenu_rows.push(index);
            self.layout = Layout::with_submenus(&self.rows, fonts, &self.submenu_rows);
            self.dirty = true;
        }
    }

    /// 【上下文菜单】【面板范围】返回当前显示器本地菜单矩形，不包含阴影
    pub fn rect(&self) -> Rect {
        Rect::from_xywh(
            self.position.0,
            self.position.1,
            self.layout.size.0,
            self.layout.size.1,
        )
        .unwrap()
    }

    /// 【上下文菜单】【位置命中】返回 local 对应行索引，分隔线和禁用行仍保留几何命中
    pub fn item_at(&self, local: (f64, f64)) -> Option<usize> {
        let point = tiny_skia::Point::from_xy(
            local.0 as f32 - self.position.0,
            local.1 as f32 - self.position.1,
        );
        self.layout
            .rows
            .iter()
            .position(|rect| crate::ui::toolbar::contains(*rect, (point.x, point.y)))
    }

    /// 【上下文菜单】【可用判断】返回 index 对应行是否支持交互，空值、越界、禁用与分隔线均不可交互
    fn selectable(&self, index: Option<usize>) -> bool {
        index
            .and_then(|index| self.rows.get(index))
            .is_some_and(|row| row.enabled && !row.separator())
    }

    /// 【上下文菜单】【高亮更新】将可用 index 设为高亮，返回是否变化；禁用行取消高亮
    fn select(&mut self, index: Option<usize>) -> Event {
        let index = index.filter(|index| self.selectable(Some(*index)));
        if self.active == index {
            return Event::None;
        }
        self.active = index;
        self.dirty = true;
        Event::Changed
    }

    /// 【上下文菜单】【鼠标悬停】按 monitor 和 local 更新高亮，返回重绘事件；其他显示器取消高亮
    pub fn pointer(&mut self, monitor: usize, local: (f64, f64)) -> Event {
        self.select(
            (monitor == self.monitor_idx)
                .then(|| self.item_at(local))
                .flatten(),
        )
    }

    /// 【上下文菜单】【按钮分派】处理 monitor、local 上 button 的 pressed 状态，返回关闭或动作请求；只有左键释放执行动作
    pub fn button(
        &mut self,
        monitor: usize,
        local: (f64, f64),
        button: MouseButton,
        pressed: bool,
    ) -> Event {
        let inside = monitor == self.monitor_idx
            && crate::ui::toolbar::contains(self.rect(), (local.0 as f32, local.1 as f32));
        if pressed && !inside {
            return Event::Close;
        }
        if !matches!(button, MouseButton::Left) {
            return Event::None;
        }
        if pressed {
            self.left_pressed = inside;
            return Event::None;
        }
        let armed = std::mem::take(&mut self.left_pressed);
        let index = self.item_at(local).filter(|_| inside);
        if armed && self.selectable(index) {
            Event::Activate(index.unwrap())
        } else {
            Event::None
        }
    }

    /// 【上下文菜单】【循环导航】按 from 和 step 返回下一可用索引；全部禁用时返回空值
    fn next(&self, from: Option<usize>, step: isize) -> Option<usize> {
        let count = self.rows.len() as isize;
        if count == 0 {
            return None;
        }
        let mut index = from.map_or(if step > 0 { -1 } else { count }, |index| index as isize);
        for _ in 0..count {
            index = (index + step).rem_euclid(count);
            if self.selectable(Some(index as usize)) {
                return Some(index as usize);
            }
        }
        None
    }

    /// 【上下文菜单】【键盘导航】按 key、shift 返回导航、关闭或动作；其余按键不穿透到编辑器
    pub fn key(&mut self, key: Key, shift: bool) -> Event {
        let next = match key {
            Key::Escape => return Event::Close,
            Key::Enter | Key::Space => {
                return self
                    .active
                    .filter(|index| self.selectable(Some(*index)))
                    .map_or(Event::None, Event::Activate);
            }
            Key::Up => self.next(self.active, -1),
            Key::Down => self.next(self.active, 1),
            Key::Tab => self.next(self.active, if shift { -1 } else { 1 }),
            Key::Home | Key::PageUp => self.next(None, 1),
            Key::End | Key::PageDown => self.next(None, -1),
            _ => return Event::None,
        };
        self.select(next)
    }
}
