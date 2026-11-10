//! 【框选翻译】【结果浮条】语言、显示、复制、浮窗和退出分别命中，留白消费点击

use crate::types::MouseButton;
use tiny_skia::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Language,
    Original,
    Translated,
    Copy,
    OpenWindow,
    Exit,
}

pub struct Bar {
    pub monitor: usize,
    pub rect: Rect,
    pub cells: Vec<(Action, Rect)>,
    pub label: String,
    pub translated: bool,
    pub hovered: Option<Action>,
    pub pressed: Option<Action>,
}

impl Bar {
    /// 【框选翻译】【浮条布局】按 label、translated 和 fonts 返回原版布局，文字按钮宽度由字体度量决定
    pub fn new(label: String, translated: bool, fonts: &mut cosmic_text::FontSystem) -> Self {
        let language = crate::ui::language::Language::current();
        crate::fonts::ocr_ui::ensure_text(fonts, &label);
        crate::fonts::ocr_ui::ensure_text(fonts, language.text("Original"));
        crate::fonts::ocr_ui::ensure_text(fonts, language.text("Translated"));
        let mut x = 23.0;
        let mut cells = Vec::new();
        for (action, text, icon) in [
            (Action::Language, label.as_str(), true),
            (Action::Original, language.text("Original"), false),
            (Action::Translated, language.text("Translated"), false),
            (Action::Copy, "", true),
            (Action::OpenWindow, "", true),
            (Action::Exit, "", true),
        ] {
            let width = (super::font::width(text, fonts).ceil()
                + 14.0
                + if icon { 13.0 } else { 0.0 }
                + if action == Action::Language { 4.0 } else { 0.0 })
            .max(if text.is_empty() { 31.0 } else { 27.0 });
            let (top, height) = if text.is_empty() {
                (5.0, 19.0)
            } else {
                (3.0, 24.0)
            };
            cells.push((action, Rect::from_xywh(x, top, width, height).unwrap()));
            x += width
                + if matches!(action, Action::Language | Action::Translated) {
                    6.0
                } else {
                    2.0
                };
        }
        Self {
            monitor: 0,
            rect: Rect::from_xywh(0.0, 0.0, x + 1.0, 30.0).unwrap(),
            cells,
            label,
            translated,
            hovered: None,
            pressed: None,
        }
    }

    /// 【框选翻译】【动作命中】按全局于显示器的 point 返回浮条动作，留白返回空值
    pub fn hit(&self, point: (f64, f64)) -> Option<Action> {
        let local = (
            point.0 - f64::from(self.rect.left()),
            point.1 - f64::from(self.rect.top()),
        );
        self.cells
            .iter()
            .find_map(|(action, rect)| super::draw::contains(*rect, local).then_some(*action))
    }

    /// 【框选翻译】【浮条点击】按 point、button、pressed 返回完整左键序列的动作，右键不消费左键身份
    pub fn button(
        &mut self,
        point: (f64, f64),
        button: MouseButton,
        pressed: bool,
    ) -> Option<Action> {
        if !matches!(button, MouseButton::Left) {
            return None;
        }
        let hit = self.hit(point);
        if pressed {
            self.pressed = hit;
            None
        } else {
            let armed = self.pressed.take();
            hit.filter(|hit| Some(*hit) == armed)
        }
    }
}
