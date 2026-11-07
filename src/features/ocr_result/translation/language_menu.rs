//! 【OCR 翻译】【语言弹层】独立原生列表显示八行视区，键盘和滚轮可到达第九项，不把菜单输入交给正文

use super::super::{control_input::FocusTarget, window::ResultWindow};
use crate::{
    backend::window::{WindowAction, WindowPopupAction},
    keys::Key,
    renderer::Painter,
    types::{
        MouseButton,
        popup::{PopupEvent, PopupFrame},
    },
    ui::controls,
};
use tiny_skia::{Pixmap, Rect};

#[cfg(test)]
mod tests;
mod text;

pub(crate) struct Menu {
    pub id: u64,
    pub control: Rect,
    position: (f32, f32),
    active: Option<usize>,
    first: usize,
    pressed: Option<usize>,
    dirty: bool,
    drawn_theme: Option<crate::theme::Mode>,
    pixels: Option<Pixmap>,
    labels: Vec<[cosmic_text::Buffer; 2]>,
    scrollbar_press: Option<(f64, usize)>,
}

impl Menu {
    /// 【OCR 翻译】【列表尺寸】返回与语言框等宽的八行原版视区，无参数，不扩大到九行
    fn size(&self) -> (f32, f32) {
        (self.control.width(), 250.0)
    }

    /// 【OCR 翻译】【列表视区】返回排除原版五像素边框和六像素滚动条的文字视区，无参数
    fn viewport(&self) -> Rect {
        Rect::from_xywh(5.0, 5.0, self.size().0 - 16.0, 240.0).unwrap()
    }

    /// 【OCR 翻译】【滚动区域】返回原版右侧六像素滚动区域，无参数，文字命中不得覆盖此区域
    fn scrollbar(&self) -> Rect {
        Rect::from_xywh(self.size().0 - 11.0, 5.0, 6.0, 240.0).unwrap()
    }

    /// 【OCR 翻译】【列表命中】按 x、y 返回可见行的实际身份，边框和空白不接收动作
    fn item(&self, x: f64, y: f64) -> Option<usize> {
        let size = self.size();
        if x < 5.0 || x >= f64::from(size.0 - 11.0) || !(5.0..245.0).contains(&y) {
            return None;
        }
        let index = ((y as f32 - 5.0) / 30.0).floor() as usize + self.first;
        (index < 9).then_some(index)
    }

    /// 【OCR 翻译】【当前行可见】按 index 更新活动项并滚动视区，返回是否变化，末项仍处于八行范围内
    fn select(&mut self, index: Option<usize>) -> bool {
        let previous = (self.active, self.first);
        self.active = index;
        if let Some(index) = index {
            if index < self.first {
                self.first = index;
            }
            if index >= self.first + 8 {
                self.first = index - 7;
            }
        }
        let changed = previous != (self.active, self.first);
        self.dirty |= changed;
        changed
    }
}

/// 【OCR 翻译】【列表打开】打开 window 的原生语言弹层，返回重绘；忙碌时不创建弹层，候选在菜单期间停用
pub(crate) fn open(window: &mut ResultWindow) -> WindowAction {
    if window.translation.state.busy() {
        return WindowAction::None;
    }
    let Some(layout) = &window.layout.translation else {
        return WindowAction::None;
    };
    let control = layout.language;
    let target = window.translation.target();
    let active = crate::translation::language::OPTIONS
        .iter()
        .position(|value| *value == target);
    window.focus_control(FocusTarget::Language);
    super::super::input_method::cancel(window);
    window.source_menu = None;
    window.more_menu.clear();
    window.dragging = false;
    window.control_press = None;
    window.translation.language_menu = Some(Menu {
        id: crate::features::text::input_method::focus::identity(),
        control,
        position: (control.x(), control.bottom()),
        active,
        first: active.map_or(0, |index| index.saturating_sub(7)),
        pressed: None,
        dirty: true,
        drawn_theme: None,
        pixels: None,
        labels: text::rows(&mut window.fonts, control.width(), window.language),
        scrollbar_press: None,
    });
    WindowAction::Redraw
}

/// 【OCR 翻译】【独立列表帧】按 window 和 scale 返回变化像素，静止列表不分配或重画，不混入父窗口画面
pub(crate) fn frame(window: &mut ResultWindow, scale: f32) -> Option<PopupFrame<'_>> {
    let menu = window.translation.language_menu.as_mut()?;
    let size = menu.size();
    let pixels = (
        (size.0 * scale).ceil() as u32,
        (size.1 * scale).ceil() as u32,
    );
    if !menu.dirty
        && menu.drawn_theme == Some(crate::theme::Mode::current())
        && menu
            .pixels
            .as_ref()
            .is_some_and(|p| (p.width(), p.height()) == pixels)
    {
        return None;
    }
    let mut canvas = Pixmap::new(pixels.0, pixels.1)?;
    canvas.fill(tiny_skia::Color::TRANSPARENT);
    let colors = super::super::palette::Palette::current();
    let mut painter = Painter::new(&mut canvas)
        .with_scale(scale)
        .with_text(&mut window.fonts, &mut window.cache);
    controls::frame(
        &mut painter,
        Rect::from_xywh(0.0, 0.0, size.0, size.1).unwrap(),
        colors.base,
        colors.border,
        0.0,
    );
    let viewport = menu.viewport();
    painter.clipped(Some(viewport), |painter| {
        for index in menu.first..(menu.first + 8).min(9) {
            let row = Rect::from_xywh(
                9.0,
                6.0 + (index - menu.first) as f32 * 30.0,
                size.0 - 24.0,
                28.0,
            )
            .unwrap();
            let selected = menu.active == Some(index);
            if selected {
                painter.fill_rounded(row, 6.0, colors.accent);
            }
            let foreground = if selected {
                colors.on_accent
            } else {
                colors.text
            };
            let text_rect = Rect::from_xywh(
                row.x() + 10.0,
                row.y(),
                (row.width() - if selected { 38.0 } else { 20.0 }).max(1.0),
                row.height(),
            )
            .unwrap();
            painter.clipped(Some(text_rect), |painter| {
                painter.text_buffer(
                    &menu.labels[index][usize::from(selected)],
                    (
                        text_rect.x(),
                        text_rect.y() + (text_rect.height() - 19.0) * 0.5,
                    ),
                    foreground.color(),
                )
            });
            if selected {
                let (x, y) = (row.right() - 17.0, row.y() + row.height() * 0.5 - 1.0);
                let mut path = tiny_skia::PathBuilder::new();
                path.move_to(x - 5.0, y);
                path.line_to(x - 1.5, y + 3.5);
                path.line_to(x + 4.5, y - 3.5);
                if let Some(path) = path.finish() {
                    painter.stroke_path(
                        &path,
                        foreground,
                        &tiny_skia::Stroke {
                            width: 1.7,
                            line_cap: tiny_skia::LineCap::Round,
                            line_join: tiny_skia::LineJoin::Round,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    });
    painter.fill_rounded(
        Rect::from_xywh(
            size.0 - 9.0,
            7.0 + menu.first as f32 * (236.0 / 9.0),
            2.0,
            236.0 * 8.0 / 9.0,
        )
        .unwrap(),
        2.0,
        colors.border,
    );
    menu.pixels = Some(canvas);
    menu.dirty = false;
    menu.drawn_theme = Some(crate::theme::Mode::current());
    let canvas = menu.pixels.as_ref()?;
    Some(PopupFrame {
        id: menu.id,
        monitor_idx: 0,
        serial: None,
        anchor: (menu.control.x(), menu.control.bottom()),
        size: (size.0 as u32, size.1 as u32),
        scale,
        pixels: canvas.data(),
        pixel_size: pixels,
    })
}

/// 【OCR 翻译】【列表事件】按 id 和 event 处理 window 当前弹层，返回导航、关闭或延后选择，右键不执行语言选择
pub(crate) fn event(window: &mut ResultWindow, id: u64, event: PopupEvent) -> WindowPopupAction {
    let Some(menu) = window
        .translation
        .language_menu
        .as_mut()
        .filter(|menu| menu.id == id)
    else {
        return WindowPopupAction::None;
    };
    let changed = match event {
        PopupEvent::Configured { position } => {
            menu.position = (position.0 as f32, position.1 as f32);
            false
        }
        PopupEvent::Pointer { y, .. } if menu.scrollbar_press.is_some() => {
            let (start, first) = menu.scrollbar_press.unwrap();
            let next = if y > start + 2.0 {
                1
            } else if y < start - 2.0 {
                0
            } else {
                first
            };
            let changed = menu.first != next;
            menu.first = next;
            menu.dirty |= changed;
            changed
        }
        PopupEvent::Pointer { x, y } => {
            let index = menu.item(x, y);
            menu.select(index)
        }
        PopupEvent::Leave => false,
        PopupEvent::Scroll { delta } => {
            let next = if delta.1 > 0.0 {
                1
            } else if delta.1 < 0.0 {
                0
            } else {
                menu.first
            };
            let changed = next != menu.first;
            menu.first = next;
            menu.dirty |= changed;
            changed
        }
        PopupEvent::Button {
            x,
            y,
            button: MouseButton::Left,
            pressed,
        } => {
            if !pressed && menu.scrollbar_press.take().is_some() {
                return WindowPopupAction::Redraw;
            }
            if pressed && super::super::layout::contains(menu.scrollbar(), (x, y)) {
                menu.scrollbar_press = Some((y, menu.first));
                menu.pressed = None;
                if y > 220.0 {
                    menu.first = 1;
                    menu.dirty = true;
                }
                return WindowPopupAction::Redraw;
            }
            let index = menu.item(x, y);
            if pressed {
                menu.pressed = index;
                false
            } else if let Some(index) = index.filter(|index| menu.pressed.take() == Some(*index)) {
                return WindowPopupAction::Activate(index);
            } else {
                menu.pressed = None;
                false
            }
        }
        PopupEvent::Button { .. } => false,
        PopupEvent::Closed => return WindowPopupAction::Close,
        PopupEvent::Key { chord, text } => match chord.map(|chord| chord.key) {
            Some(Key::Escape) => return WindowPopupAction::Close,
            Some(Key::Enter | Key::Space) => {
                return menu
                    .active
                    .map_or(WindowPopupAction::None, WindowPopupAction::Activate);
            }
            Some(Key::Down) => menu.select(Some(menu.active.map_or(0, |index| (index + 1).min(8)))),
            Some(Key::Up) => {
                menu.select(Some(menu.active.map_or(8, |index| index.saturating_sub(1))))
            }
            Some(Key::Home | Key::PageUp) => menu.select(Some(0)),
            Some(Key::End | Key::PageDown) => menu.select(Some(8)),
            _ => {
                let letter = text
                    .as_deref()
                    .and_then(|text| text.chars().next())
                    .filter(|letter| letter.is_ascii_alphabetic());
                if let Some(letter) = letter {
                    let start = menu.active.map_or(0, |index| index + 1);
                    let index = (start..start + 9).map(|index| index % 9).find(|index| {
                        crate::translation::language::OPTIONS[*index]
                            .starts_with([letter.to_ascii_uppercase(), letter.to_ascii_lowercase()])
                    });
                    menu.select(index.or(menu.active))
                } else {
                    false
                }
            }
        },
    };
    if changed {
        WindowPopupAction::Redraw
    } else {
        WindowPopupAction::None
    }
}

/// 【OCR 翻译】【列表交付】销毁 window 的 id 对应弹层后确认 index，返回父窗口动作，关闭或旧身份不保存语言
pub(crate) fn closed(window: &mut ResultWindow, id: u64, index: Option<usize>) -> WindowAction {
    if !window
        .translation
        .language_menu
        .as_ref()
        .is_some_and(|menu| menu.id == id)
    {
        return WindowAction::None;
    }
    window.translation.language_menu = None;
    if let Some(index) = index {
        super::controller::choose(window, index)
    } else {
        WindowAction::Redraw
    }
}
