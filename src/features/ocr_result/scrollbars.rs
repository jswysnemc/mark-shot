//! 【文字识别】【正文滚动条】原文和译文独立滚动，共用文字排版范围与窄滑块

use super::{
    layout::contains,
    translation::input::{self, Field},
    window::{ResultWindow, View},
};
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowEvent},
    renderer::Painter,
    ui::scrollbar::{Drag, Geometry},
};
use tiny_skia::{Pixmap, Rect};

#[derive(Default)]
pub(super) struct State {
    drag: Option<(Field, Drag)>,
}

/// 【文字识别】【栏位滚动条】按 window 和 field 返回实际滚动几何，隐藏栏位和短正文返回空值
fn geometry(window: &mut ResultWindow, field: Field) -> Option<Geometry> {
    if window.view != View::Text {
        return None;
    }
    let text = match field {
        Field::Source => window.layout.text_rect(),
        Field::Translation => window.layout.translation.as_ref()?.editor,
        _ => return None,
    };
    let viewport = Rect::from_xywh(text.x(), text.y(), text.width() + 10.0, text.height())?;
    input::with_editor(window, field, |editor, fonts| {
        editor.resize(fonts, text.width(), text.height());
        editor.scrollbar(fonts, viewport)
    })
    .flatten()
}

/// 【文字识别】【滚动条绘制】把 window 两栏滑块按 scale 绘到 canvas，返回值为空，短正文不显示轨道
pub(super) fn draw(window: &mut ResultWindow, canvas: &mut Pixmap, scale: f32) {
    let colors = super::palette::Palette::current();
    for field in [Field::Source, Field::Translation] {
        if let Some(bar) = geometry(window, field) {
            let mut painter = Painter::new(canvas).with_scale(scale);
            painter.fill_rounded(bar.track, 4.0, colors.alternate);
            painter.fill_rounded(bar.handle, 3.0, colors.muted);
        }
    }
}

/// 【文字识别】【滚动条输入】按 event 处理 window 的拖动、轨道翻页与结束，返回消费动作，右键和正文事件继续分派
pub(super) fn event(window: &mut ResultWindow, event: &WindowEvent) -> Option<WindowAction> {
    match *event {
        WindowEvent::Focused(false) | WindowEvent::Resize { .. } => {
            window.scrollbars.drag = None;
            None
        }
        WindowEvent::PointerButton {
            button: LEFT_BUTTON,
            pressed: false,
            ..
        } if window.scrollbars.drag.take().is_some() => Some(WindowAction::Redraw),
        WindowEvent::PointerMove { position } => {
            let (field, drag) = window.scrollbars.drag?;
            if let Some(bar) = geometry(window, field) {
                input::with_editor(window, field, |editor, fonts| {
                    editor.scroll_to(fonts, drag.value(position, &bar))
                });
            }
            Some(WindowAction::Redraw)
        }
        WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed: true,
        } => {
            for field in [Field::Source, Field::Translation] {
                let Some(bar) = geometry(window, field).filter(|bar| contains(bar.track, position))
                else {
                    continue;
                };
                window.dragging = false;
                window.control_press = None;
                if contains(bar.handle, position) {
                    window.scrollbars.drag = Some((field, Drag::new(position, &bar)));
                } else {
                    // 1. 【文字识别】【轨道翻页】按点击方向移动一屏，滑块与正文共享绝对像素位置
                    let current =
                        Drag::new((f64::from(bar.handle.x()), f64::from(bar.handle.y())), &bar)
                            .value((f64::from(bar.handle.x()), f64::from(bar.handle.y())), &bar);
                    let next = if position.1 < f64::from(bar.handle.y()) {
                        current.saturating_sub(bar.page)
                    } else {
                        (current + bar.page).min(bar.maximum)
                    };
                    input::with_editor(window, field, |editor, fonts| {
                        editor.scroll_to(fonts, next)
                    });
                }
                return Some(WindowAction::Redraw);
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::text_editor::TextEditor;

    /// 【滚动条测试】【独立滚动】两栏长文分别显示滑块，拖到底再回顶不改变正文与选择，短文隐藏滑块，无参数和返回值
    #[test]
    fn translation_ui_regression_text_scrollbars_drag_each_pane() {
        let (_root, mut window) = super::super::translation::controller::tests::window("exit 8");
        let text = (0..100)
            .map(|i| format!("line {i}: long source and translation text\n"))
            .collect::<String>();
        window.source = TextEditor::new(&text, &mut window.fonts);
        window.translation.editor = TextEditor::new(&text, &mut window.fonts);
        window.translation.state.visible = true;
        window.refresh_layout();
        for width in [760.0, 460.0] {
            window.layout.size.0 = width;
            window.translation.layout_dirty = true;
            window.refresh_layout();
            for field in [Field::Source, Field::Translation] {
                input::with_editor(&mut window, field, |editor, _| editor.reset_scroll());
                let bar = geometry(&mut window, field).unwrap();
                let start = (
                    f64::from(bar.handle.x() + 2.0),
                    f64::from(bar.handle.y() + 5.0),
                );
                assert!(
                    event(
                        &mut window,
                        &WindowEvent::PointerButton {
                            position: start,
                            button: LEFT_BUTTON,
                            pressed: true
                        }
                    )
                    .is_some()
                );
                assert!(
                    event(
                        &mut window,
                        &WindowEvent::PointerMove {
                            position: (start.0, 2000.0)
                        }
                    )
                    .is_some()
                );
                let bottom = geometry(&mut window, field).unwrap();
                assert!((bottom.handle.bottom() - bottom.track.bottom()).abs() < 1.0);
                assert!(
                    event(&mut window, &WindowEvent::PointerMove { position: start }).is_some()
                );
                let top = geometry(&mut window, field).unwrap();
                assert!((top.handle.y() - top.track.y()).abs() < 1.0);
                event(
                    &mut window,
                    &WindowEvent::PointerButton {
                        position: start,
                        button: LEFT_BUTTON,
                        pressed: false,
                    },
                );
                assert_eq!(input::editor(&window, field).unwrap().text(), text);
                assert!(
                    input::editor(&window, field)
                        .unwrap()
                        .selected_text()
                        .is_none()
                );
            }
        }
        window.source = TextEditor::new("short", &mut window.fonts);
        assert!(geometry(&mut window, Field::Source).is_none());
        assert!(geometry(&mut window, Field::Translation).is_some());
    }
}
