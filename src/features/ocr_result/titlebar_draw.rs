//! 【文字识别】【标题绘制】使用共享按钮组件绘制单行页签、矢量图标和原文复制反馈

use tiny_skia::Rect;

use super::{control_input::FocusTarget, labels, layout::Control, window::ResultWindow};
use crate::{
    renderer::{Icons, Painter, text::TextStyle},
    theme::Rgba,
    ui::{controls, icons},
};

pub(super) const UNDO: &str = include_str!("../../../assets/icons/mark-shot/undo.svg");
const MORE: &str = include_str!("../../../assets/icons/mark-shot/more.svg");
const TRANSLATE: &str = include_str!("../../../assets/icons/mark-shot/translate.svg");
const PIN: &str = include_str!("../../../assets/icons/mark-shot/pin.svg");

/// 【文字识别】【图标缓存】返回 OCR 窗口实际使用的七个矢量图标，不解析其他工具栏资源，无参数
pub(super) fn icons() -> Icons {
    [
        icons::OCR,
        icons::COPY,
        icons::CLOSE,
        UNDO,
        MORE,
        TRANSLATE,
        PIN,
    ]
    .into_iter()
    .map(|svg| {
        (
            svg,
            usvg::Tree::from_str(svg, &usvg::Options::default())
                .expect("embedded OCR icon must parse"),
        )
    })
    .collect()
}

/// 【文字识别】【控件外观】用 painter、state、control、bounds、label、icon 和 size 绘制原版 quiet 按钮，无返回值
pub(super) fn button(
    painter: &mut Painter<'_>,
    state: &DrawState,
    control: Control,
    bounds: Rect,
    label: &str,
    icon: Option<&'static str>,
    size: f32,
) {
    let label = state.language.text(label);
    let enabled = match control {
        Control::Copy => state.copy_enabled,
        Control::Undo => state.can_undo,
        Control::TranslationToggle => state.translation_enabled,
        Control::TranslationCopy => state.translation_copy,
        Control::TranslationUndo => state.translation_undo,
        Control::AlwaysOnTop => state.stacking_supported,
        _ => true,
    };
    let hovered = state.hovered == Some(control);
    let focused = state.focused && state.focus_target == FocusTarget::Button(control);
    let down = match control {
        Control::Copy => state.down[0],
        Control::Close => state.down[1],
        Control::Undo => state.down[2],
        Control::TableCopy => state.down[3],
        Control::More => state.down[4],
        Control::TranslationToggle => state.down[5],
        Control::TranslationCopy => state.down[6],
        Control::TranslationUndo => state.down[7],
        Control::AlwaysOnTop => state.down[10],
        _ => false,
    };
    let transparent = Rgba(0, 0, 0, 0);
    let colors = state.colors;
    let icon_button = matches!(
        control,
        Control::Close | Control::More | Control::TranslationToggle | Control::AlwaysOnTop
    );
    let checked = control == Control::TranslationToggle && state.translation_visible
        || control == Control::AlwaysOnTop && state.always_on_top;
    let fill = if checked && enabled {
        colors.pressed
    } else if !enabled || (hovered || focused && icon_button) && !down {
        colors.alternate
    } else if down {
        colors.pressed
    } else {
        transparent
    };
    let border = if !enabled {
        colors.border
    } else if focused && !icon_button {
        colors.accent
    } else if hovered && enabled && !icon_button {
        colors.border
    } else {
        transparent
    };
    controls::icon_button(
        painter,
        bounds,
        label,
        icon,
        controls::ButtonStyle {
            fill,
            border,
            foreground: if !enabled {
                colors.muted
            } else if control == Control::More {
                colors.more_icon
            } else if checked {
                colors.accent
            } else {
                colors.text
            },
            font_size: size,
            weight: cosmic_text::Weight::NORMAL,
        },
    );
}

pub(super) struct DrawState {
    language: crate::ui::language::Language,
    colors: super::palette::Palette,
    focused: bool,
    focus_target: FocusTarget,
    hovered: Option<Control>,
    down: [bool; 11],
    copy_enabled: bool,
    can_undo: bool,
    translation_enabled: bool,
    translation_copy: bool,
    translation_undo: bool,
    translation_visible: bool,
    stacking_supported: bool,
    always_on_top: bool,
    view: super::window::View,
}

impl DrawState {
    /// 【文字识别】【按钮快照】从 state 借用少量按钮标志并返回独立快照，不复制正文或替换字库
    pub fn new(state: &ResultWindow) -> Self {
        Self {
            language: state.language,
            colors: super::palette::Palette::current(),
            focused: state.focused,
            focus_target: state.focus_target,
            hovered: state.hovered,
            down: [
                Control::Copy,
                Control::Close,
                Control::Undo,
                Control::TableCopy,
                Control::More,
                Control::TranslationToggle,
                Control::TranslationCopy,
                Control::TranslationUndo,
                Control::Translate,
                Control::LanguageMenu,
                Control::AlwaysOnTop,
            ]
            .map(|control| state.control_down(control)),
            copy_enabled: state.control_enabled(Control::Copy),
            can_undo: state.can_undo(),
            translation_enabled: state.control_enabled(Control::TranslationToggle),
            translation_copy: state.control_enabled(Control::TranslationCopy),
            translation_undo: state.control_enabled(Control::TranslationUndo),
            translation_visible: state.translation.state.visible,
            stacking_supported: state.stacking.supported,
            always_on_top: state.stacking.active,
            view: state.view,
        }
    }
}

/// 【文字识别】【单行绘制】按 state 快照、layout 和 copy_label 绘制 painter 标题及撤销行，页签严格裁切，无返回值
pub(super) fn draw(
    state: &DrawState,
    layout: &super::layout::Layout,
    copy_label: &str,
    painter: &mut Painter<'_>,
) {
    let titlebar = &layout.titlebar;
    if let Some(rect) = titlebar.icon {
        painter.icon(icons::OCR, 18.0, rect.x(), rect.y(), state.colors.accent);
    }
    painter.clipped(Some(titlebar.viewport), |painter| {
        painter.fill_rounded(titlebar.viewport, 10.0, state.colors.segment);
        for &(view, rect) in &titlebar.tabs {
            if view == state.view {
                painter.fill_rounded(
                    Rect::from_xywh(
                        rect.x() + 1.0,
                        rect.y() + 1.0,
                        rect.width() - 2.0,
                        rect.height() - 2.0,
                    )
                    .unwrap(),
                    8.0,
                    state.colors.selected_segment,
                );
            }
            let ink =
                if state.focused && state.focus_target == FocusTarget::Tabs && view == state.view {
                    state.colors.accent
                } else if view == state.view || state.hovered == Some(Control::View(view)) {
                    state.colors.text
                } else {
                    state.colors.muted
                };
            painter.text(
                rect,
                state.language.text(labels::view(view)),
                TextStyle::new(labels::TITLE_FONT, ink).center(),
            );
        }
    });
    button(
        painter,
        state,
        Control::Copy,
        layout.copy,
        copy_label,
        Some(icons::COPY),
        labels::TITLE_FONT,
    );
    if let Some(rect) = layout.titlebar.slots[0] {
        button(
            painter,
            state,
            Control::TranslationToggle,
            rect,
            "",
            Some(TRANSLATE),
            labels::TITLE_FONT,
        );
    }
    if let Some(rect) = layout.titlebar.slots[2] {
        button(
            painter,
            state,
            Control::AlwaysOnTop,
            rect,
            "",
            Some(PIN),
            labels::TITLE_FONT,
        );
    }
    button(
        painter,
        state,
        Control::Close,
        layout.close,
        "",
        Some(icons::CLOSE),
        labels::TITLE_FONT,
    );
    button(
        painter,
        state,
        Control::More,
        layout.more,
        "",
        Some(MORE),
        labels::TITLE_FONT,
    );
    if let Some(rect) = layout.undo {
        button(
            painter,
            state,
            Control::Undo,
            rect,
            "",
            Some(UNDO),
            labels::TITLE_FONT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【标题绘制测试】【静默焦点】普通复制按钮的焦点只改变边框，不填充正文背景；关闭图标仍显示焦点底色，无参数、无返回值
    #[test]
    fn quiet_copy_focus_does_not_change_the_caption_background() {
        let colors = super::super::palette::Palette::current();
        let mut state = DrawState {
            language: crate::ui::language::Language::English,
            colors,
            focused: true,
            focus_target: FocusTarget::Button(Control::Copy),
            hovered: None,
            down: [false; 11],
            copy_enabled: true,
            can_undo: false,
            translation_enabled: true,
            translation_copy: false,
            translation_undo: false,
            translation_visible: false,
            stacking_supported: true,
            always_on_top: false,
            view: super::super::window::View::Text,
        };
        let mut canvas = tiny_skia::Pixmap::new(80, 40).unwrap();
        canvas.fill(colors.window.color());
        let original = canvas.pixel(10, 17).unwrap();
        let rect = Rect::from_xywh(4.0, 4.0, 68.0, 26.0).unwrap();
        button(
            &mut Painter::new(&mut canvas),
            &state,
            Control::Copy,
            rect,
            "Copy",
            None,
            12.0,
        );
        assert_eq!(canvas.pixel(10, 17).unwrap(), original);
        state.focus_target = FocusTarget::Button(Control::Close);
        button(
            &mut Painter::new(&mut canvas),
            &state,
            Control::Close,
            rect,
            "",
            None,
            12.0,
        );
        assert_ne!(canvas.pixel(10, 17).unwrap(), original);
        state.copy_enabled = false;
        state.focus_target = FocusTarget::Button(Control::Copy);
        button(
            &mut Painter::new(&mut canvas),
            &state,
            Control::Copy,
            rect,
            "Copy",
            None,
            12.0,
        );
        assert_eq!(
            canvas.pixel(4, 17).unwrap().demultiply(),
            colors.border.color().to_color_u8()
        );
    }
}
