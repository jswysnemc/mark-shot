//! 【框选翻译】【语言列表】当前目标可自定义，只有完整左键点击或键盘确认会选中语言

use crate::{keys::Key, types::MouseButton};
use tiny_skia::{Pixmap, Rect};

pub struct Popup {
    pub id: u64,
    pub monitor: usize,
    pub rect: Rect,
    pub options: Vec<String>,
    pub current: usize,
    pub active: usize,
    pub pressed: Option<usize>,
    pub native: bool,
    pub dirty: bool,
    pub pixels: Option<Pixmap>,
    pub theme: Option<crate::theme::Mode>,
}

pub enum Event {
    None,
    Close,
    Choose(String),
}

impl Popup {
    /// 【框选翻译】【列表创建】按 id、monitor、target 和 fonts 返回包含自由目标的紧凑语言列表
    pub fn new(id: u64, monitor: usize, target: &str, fonts: &mut cosmic_text::FontSystem) -> Self {
        let mut options = crate::translation::language::OPTIONS
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>();
        if !options.iter().any(|s| s == target) && !target.trim().is_empty() {
            options.insert(0, target.into());
        }
        let current = options.iter().position(|s| s == target).unwrap_or(0);
        let language = crate::ui::language::Language::current();
        for option in &options {
            crate::fonts::ocr_ui::ensure_text(
                fonts,
                crate::translation::language::display_for(option, language),
            );
        }
        let mut width = super::font::width(language.text("Translate to"), fonts);
        for option in &options {
            width = width.max(super::font::width(
                crate::translation::language::display_for(option, language),
                fonts,
            ));
        }
        let size = (
            (width.round() + 49.0).clamp(132.0, 240.0),
            28.0 + options.len() as f32 * 26.0,
        );
        Self {
            id,
            monitor,
            rect: Rect::from_xywh(0.0, 0.0, size.0, size.1).unwrap(),
            options,
            current,
            active: current,
            pressed: None,
            native: false,
            dirty: true,
            pixels: None,
            theme: None,
        }
    }

    /// 【框选翻译】【语言行】按 index 返回列表本地行矩形，尺寸与原版一致
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_xywh(
            4.0,
            24.0 + index as f32 * 26.0,
            self.rect.width() - 8.0,
            26.0,
        )
        .unwrap()
    }

    /// 【框选翻译】【行命中】按 point 返回有效语言行，标题与留白返回空值
    pub fn hit(&self, point: (f64, f64)) -> Option<usize> {
        (0..self.options.len()).find(|&i| super::draw::contains(self.row(i), point))
    }

    /// 【框选翻译】【悬停更新】按 point 更新活动行，返回是否改变外观
    pub fn pointer(&mut self, point: (f64, f64)) -> bool {
        if let Some(index) = self.hit(point)
            && index != self.active
        {
            self.active = index;
            self.dirty = true;
            return true;
        }
        false
    }

    /// 【框选翻译】【按钮确认】按 point、button 和 pressed 确认完整左键序列，返回选中动作，右键始终隔离
    pub fn button(&mut self, point: (f64, f64), button: MouseButton, pressed: bool) -> Event {
        if !matches!(button, MouseButton::Left) {
            return Event::None;
        }
        let hit = self.hit(point);
        if pressed {
            self.pressed = hit;
            Event::None
        } else {
            let armed = self.pressed.take();
            hit.filter(|i| Some(*i) == armed)
                .map_or(Event::None, |i| Event::Choose(self.options[i].clone()))
        }
    }

    /// 【框选翻译】【键盘导航】按 key、shift 返回语言选择或关闭动作，其他键全部由弹层消费
    pub fn key(&mut self, key: Key, shift: bool) -> Event {
        let count = self.options.len();
        match key {
            Key::Escape => return Event::Close,
            Key::Enter | Key::Space => return Event::Choose(self.options[self.active].clone()),
            Key::Up => self.active = (self.active + count - 1) % count,
            Key::Down => self.active = (self.active + 1) % count,
            Key::Tab => self.active = (self.active + if shift { count - 1 } else { 1 }) % count,
            Key::Home => self.active = 0,
            Key::End => self.active = count - 1,
            _ => return Event::None,
        }
        self.dirty = true;
        Event::None
    }
}
