//! 【框选翻译】【组件绘制】语言列表和浮条沿用工具栏主题与矢量图标

use super::{
    language_popup::Popup,
    result_bar::{Action, Bar},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::toolbar::Palette,
    ui::{controls, icons},
};
use tiny_skia::Rect;

/// 【框选翻译】【范围命中】按 rect 和 point 返回左闭右开命中结果，无边缘重复点击
pub fn contains(rect: Rect, point: (f64, f64)) -> bool {
    point.0 >= f64::from(rect.left())
        && point.0 < f64::from(rect.right())
        && point.1 >= f64::from(rect.top())
        && point.1 < f64::from(rect.bottom())
}

/// 【框选翻译】【列表绘制】在 p 绘制 popup 本地内容，不修改选择或保存配置
pub fn popup(p: &mut Painter, popup: &Popup) {
    let palette = Palette::current();
    let language = crate::ui::language::Language::current();
    let bounds = Rect::from_xywh(0.0, 0.0, popup.rect.width(), popup.rect.height()).unwrap();
    controls::frame(p, bounds, palette.popup_fill, palette.popup_border, 10.0);
    p.text(
        Rect::from_xywh(12.0, 4.0, bounds.width() - 24.0, 20.0).unwrap(),
        language.text("Translate to"),
        TextStyle::new(13.0, palette.muted_text),
    );
    for (index, option) in popup.options.iter().enumerate() {
        let row = popup.row(index);
        if index == popup.active {
            controls::frame(
                p,
                row,
                palette.soft_hover_fill,
                palette.soft_hover_border,
                7.0,
            );
        }
        let label = controls::fit_label(
            p,
            crate::translation::language::display_for(option, language),
            13.0,
            row.width() - 35.0,
        );
        p.text(
            Rect::from_xywh(
                row.left() + 8.0,
                row.top(),
                row.width() - 35.0,
                row.height(),
            )
            .unwrap(),
            &label,
            TextStyle::new(13.0, palette.text),
        );
        if index == popup.current {
            p.icon(
                icons::MARK_SHOT_CHECK,
                13.0,
                row.right() - 22.0,
                row.top() + 6.5,
                palette.hover_fill,
            );
        }
    }
}

/// 【框选翻译】【浮条绘制】按 bar 在 p 绘制原版按钮状态，不缓存与场景背景合成后的像素
pub fn bar(p: &mut Painter, bar: &Bar) {
    let palette = Palette::current();
    controls::frame(p, bar.rect, palette.panel_fill, palette.panel_border, 8.0);
    p.icon(
        crate::ui::toolbar::icons::TRANSLATE,
        13.0,
        bar.rect.left() + 8.0,
        bar.rect.top() + 8.5,
        palette.muted_text,
    );
    let language = crate::ui::language::Language::current();
    for (action, cell) in &bar.cells {
        let rect = Rect::from_xywh(
            bar.rect.left() + cell.left(),
            bar.rect.top() + cell.top(),
            cell.width(),
            cell.height(),
        )
        .unwrap();
        let active = (*action == Action::Translated && bar.translated)
            || (*action == Action::Original && !bar.translated);
        let hovered = bar.hovered == Some(*action);
        let ink = if active {
            palette.strong_text
        } else {
            palette.text
        };
        if active || hovered {
            controls::frame(
                p,
                rect,
                if active {
                    palette.active_fill
                } else {
                    palette.soft_hover_fill
                },
                if active {
                    palette.active_border
                } else {
                    palette.soft_hover_border
                },
                6.0,
            );
        }
        match action {
            Action::Language => {
                p.text(
                    Rect::from_xywh(
                        rect.left() + 7.0,
                        rect.top(),
                        rect.width() - 31.0,
                        rect.height(),
                    )
                    .unwrap(),
                    &bar.label,
                    TextStyle::new(13.0, ink),
                );
                p.icon(
                    icons::CHEVRON_DOWN,
                    13.0,
                    rect.right() - 20.0,
                    rect.top() + 5.5,
                    palette.muted_text,
                );
            }
            Action::Original | Action::Translated => {
                p.text(
                    rect,
                    language.text(if *action == Action::Original {
                        "Original"
                    } else {
                        "Translated"
                    }),
                    TextStyle::new(13.0, ink).center(),
                );
            }
            Action::Copy | Action::OpenWindow | Action::Exit => p.icon(
                match action {
                    Action::Copy => icons::COPY,
                    Action::OpenWindow => crate::ui::toolbar::icons::OPEN,
                    _ => icons::CLOSE,
                },
                13.0,
                rect.left() + (rect.width() - 13.0) / 2.0,
                rect.top() + (rect.height() - 13.0) / 2.0,
                ink,
            ),
        }
    }
}
