//! 【颜色控件】【原生对话框】复用已有取色器几何、轨道、色板和渲染，确认后交付颜色

use super::{ColorPickerPopover, ColorPopoverElement, draw_color_popover};
use crate::{
    backend::window::{self, WindowAction, WindowContent, WindowEvent, WindowSpec},
    keys::Key,
    renderer::{Painter, text::TextStyle},
    theme::{Rgba, window::Palette},
    ui::{controls, fonts::UiFonts, single_line::SingleLine},
};
use std::sync::{Arc, Mutex};
use tiny_skia::{Pixmap, Rect};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Selection {
    Color(Rgba),
    Default,
}
pub type Completion = Arc<Mutex<Option<Selection>>>;

struct Picker {
    picker: ColorPickerPopover,
    fonts: UiFonts,
    cache: cosmic_text::SwashCache,
    hex: SingleLine,
    editing: bool,
    dragging: Option<ColorPopoverElement>,
    pressed: Option<usize>,
    completion: Completion,
    chinese: bool,
    error: bool,
}

/// 【颜色控件】【按钮范围】按index返回紧凑底部按钮范围，绘制和命中共用
fn button(index: usize) -> Rect {
    Rect::from_xywh(12.0 + index as f32 * 68.0, 286.0, 64.0, 28.0).unwrap()
}

/// 【颜色控件】【矩形命中】按rect和point判断内部坐标，返回命中状态
fn contains(rect: Rect, point: (f64, f64)) -> bool {
    point.0 >= f64::from(rect.left())
        && point.0 < f64::from(rect.right())
        && point.1 >= f64::from(rect.top())
        && point.1 < f64::from(rect.bottom())
}

impl Picker {
    /// 【颜色控件】【初始状态】按color、chinese和completion构造共用组件，返回尚未修改配置的窗口状态
    fn new(color: Rgba, chinese: bool, completion: Completion) -> Self {
        let mut picker = ColorPickerPopover::new();
        picker.open = true;
        picker.opacity = 1.0;
        picker.position = (12.0, 32.0);
        picker.render_pos = picker.position;
        picker.select_color(color.color());
        picker.refresh_palette();
        picker.sync_field_values();
        picker.dirty = true;
        let mut fonts = UiFonts::with_system_family("Sans Serif");
        fonts.ensure_text(
            "选择颜色 恢复 取消 确定 颜色格式无效 Color Reset Cancel Apply Invalid color",
        );
        let hex = SingleLine::new(
            &picker.confirmed_field_text(super::ColorField::Hex),
            &mut fonts,
            9,
        );
        Self {
            picker,
            fonts,
            cache: cosmic_text::SwashCache::new(),
            hex,
            editing: false,
            dragging: None,
            pressed: None,
            completion,
            chinese,
            error: false,
        }
    }
    /// 【颜色控件】【颜色同步】无参数，将轨道调整结果同步到共用字段和原生输入控件
    fn sync(&mut self) {
        self.picker.sync_field_values();
        self.picker.dirty = true;
        self.hex.set(
            &self.picker.confirmed_field_text(super::ColorField::Hex),
            &mut self.fonts,
            false,
        );
        self.error = false;
    }
    /// 【颜色控件】【文本确认】无参数，校验十六进制草稿并同步颜色，返回是否可以确认
    fn apply_text(&mut self) -> bool {
        if !self.editing {
            return true;
        }
        let Some(color) = Rgba::parse_hex(&self.hex.text()) else {
            self.error = true;
            return false;
        };
        self.picker.select_color(color.color());
        self.editing = false;
        self.sync();
        true
    }
    /// 【颜色控件】【轨道调整】按element和position调用既有取色状态，返回是否消费调整
    fn adjust(&mut self, element: ColorPopoverElement, position: (f64, f64)) -> bool {
        match element {
            ColorPopoverElement::SvSquare => self.picker.set_sv_from_local(position),
            ColorPopoverElement::HueSlider => self.picker.set_hue_from_local(position),
            ColorPopoverElement::AlphaSlider => self.picker.set_alpha_from_local(position),
            ColorPopoverElement::Swatch(index) => {
                if let Some(color) = self.picker.palette().get(index).copied() {
                    self.picker.select_color(color);
                }
            }
            _ => return false,
        }
        self.editing = false;
        self.sync();
        true
    }
    /// 【颜色控件】【结果交付】按index选择恢复、取消或确认，返回关闭或保留窗口动作
    fn finish(&mut self, index: usize) -> WindowAction {
        if index == 2 && !self.apply_text() {
            return WindowAction::Redraw;
        }
        let selection = match index {
            0 => Some(Selection::Default),
            2 => {
                let color = self.picker.sv_square.color().to_color_u8();
                Some(Selection::Color(Rgba(
                    color.red(),
                    color.green(),
                    color.blue(),
                    color.alpha(),
                )))
            }
            _ => None,
        };
        *self.completion.lock().unwrap() = selection;
        WindowAction::Close
    }
}

impl WindowContent for Picker {
    /// 【颜色控件】【绘制窗口】按canvas和scale绘制现有取色器及统一输入、按钮，无返回值
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        let palette = Palette::current();
        canvas.fill(palette.base.color());
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut self.fonts, &mut self.cache);
        painter.text(
            Rect::from_xywh(12.0, 3.0, 200.0, 26.0).unwrap(),
            if self.chinese {
                "选择颜色"
            } else {
                "Color"
            },
            TextStyle::new(12.0, palette.text),
        );
        draw_color_popover(&mut painter, &mut self.picker);
        let labels = if self.chinese {
            ["恢复", "取消", "确定"]
        } else {
            ["Reset", "Cancel", "Apply"]
        };
        for (index, label) in labels.into_iter().enumerate() {
            controls::button(
                &mut painter,
                button(index),
                label,
                index == 2,
                self.pressed == Some(index),
                true,
            );
        }
        if self.error {
            painter.text(
                Rect::from_xywh(12.0, 261.0, 200.0, 22.0).unwrap(),
                if self.chinese {
                    "颜色格式无效"
                } else {
                    "Invalid color"
                },
                TextStyle::new(11.0, palette.text),
            );
        }
        if self.editing {
            let bounds = self.picker.hex_field_rect().unwrap();
            controls::frame(&mut painter, bounds, palette.base, palette.accent, 6.0);
            self.hex.draw(
                &mut self.fonts,
                &mut self.cache,
                canvas,
                Rect::from_xywh(
                    bounds.left() + 8.0,
                    bounds.top() + 3.0,
                    bounds.width() - 16.0,
                    bounds.height() - 6.0,
                )
                .unwrap(),
                scale,
                true,
            );
        }
    }
    /// 【颜色控件】【原生输入】按event更新共用组件，取消不交付颜色，返回绘制或关闭动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Key(input) => match input.key {
                Some(Key::Escape) => return self.finish(1),
                Some(Key::Enter) => return self.finish(2),
                _ if self.editing => {
                    self.hex.key(&mut self.fonts, &input);
                    self.error = false;
                }
                _ => return WindowAction::None,
            },
            WindowEvent::PointerMove { position } => {
                if let Some(element) = self.dragging {
                    self.adjust(element, position);
                } else {
                    return WindowAction::None;
                }
            }
            WindowEvent::PointerButton {
                position,
                button: window::LEFT_BUTTON,
                pressed,
            } => {
                if !pressed {
                    self.dragging = None;
                    if let Some(index) = self
                        .pressed
                        .take()
                        .filter(|index| contains(button(*index), position))
                    {
                        return self.finish(index);
                    }
                } else if position.1 < 28.0 {
                    return WindowAction::Move;
                } else {
                    self.pressed = (0..3).find(|index| contains(button(*index), position));
                    if self.pressed.is_none()
                        && let Some(element) = self.picker.element_at(position)
                    {
                        if matches!(element, ColorPopoverElement::Field(_)) {
                            self.editing = true;
                            self.hex.select_all(&mut self.fonts);
                        } else if self.adjust(element, position) {
                            self.dragging = Some(element);
                        }
                    }
                }
            }
            WindowEvent::Focused(false) => {
                self.dragging = None;
                self.pressed = None;
            }
            _ => return WindowAction::None,
        }
        WindowAction::Redraw
    }
    /// 【颜色控件】【确认状态】无参数，返回是否交付了明确选择，用于原生对话框生命周期
    fn dialog_accepted(&self) -> bool {
        self.completion.lock().unwrap().is_some()
    }
}

/// 【颜色控件】【对话框构造】按id、color和chinese创建可复用取色窗口，返回对话框与延迟结果
pub fn dialog(id: u64, color: Rgba, chinese: bool) -> (window::WindowDialog, Completion) {
    let completion = Arc::new(Mutex::new(None));
    (
        window::WindowDialog {
            id,
            spec: WindowSpec {
                title: "Mark Shot — Color".into(),
                app_id: "mark-shot-color".into(),
                size: (224, 326),
                minimum: (224, 326),
            },
            content: Box::new(Picker::new(color, chinese, completion.clone())),
        },
        completion,
    )
}
