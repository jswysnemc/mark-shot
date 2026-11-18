//! 【副工具栏】【动作图标】绘制下拉选择器芯片、当前样式预览与幽灵动作图标按钮

use super::{SettingsAction, SettingsPanel, draw_controls::draw_chip_frame};
use crate::{
    features::ruler::RulerMode,
    renderer::{Painter, text::TextStyle},
    theme::{Rgba, toolbar::Palette},
    ui::{
        language::Language,
        line_properties::{self, LineChoice},
    },
};
use cosmic_text::Weight;
use tiny_skia::{PathBuilder, Rect, Stroke};

const CHIP_RADIUS: f32 = 6.5;

/// 【副工具栏】【选择器判定】判断 action 是否属于弹出子面板的下拉选择器控件，返回布尔值
fn is_selector_action(action: SettingsAction) -> bool {
    matches!(
        action,
        SettingsAction::FilterModes
            | SettingsAction::ArrowStyles
            | SettingsAction::LineStyles
            | SettingsAction::NumberStyles
            | SettingsAction::RulerModes
            | SettingsAction::MarkerShapes
            | SettingsAction::ColorFormats
    )
}

/// 【副工具栏】【尺子文案】按当前语言返回尺子测量模式显示名称
fn ruler_label(mode: RulerMode) -> &'static str {
    if Language::current() == Language::Chinese {
        match mode {
            RulerMode::Length => "长度",
            RulerMode::Angle => "角度",
        }
    } else {
        mode.label()
    }
}

/// 【副工具栏】【动作绘制】按 panel 当前属性将 action 与 svg 绘入 painter 的 rect，区分选择器芯片与普通图标按钮，无返回值
pub(super) fn draw(
    painter: &mut Painter<'_>,
    panel: &SettingsPanel,
    action: SettingsAction,
    svg: &'static str,
    rect: Rect,
    is_hovered: bool,
    is_selected: bool,
) {
    let palette = Palette::current();
    let cfg = &crate::config::get().settings_panel;
    let selector = is_selector_action(action);

    // 1. 选择器芯片默认带柔和底框以便识别可展开，纯动作按钮采用与主工具栏一致的无框幽灵态
    let tint = if selector {
        let (fill, border, ink) = if is_selected {
            (
                palette.active_fill,
                palette.active_border,
                palette.strong_text,
            )
        } else if is_hovered {
            (
                palette.soft_hover_fill,
                palette.soft_hover_border,
                palette.strong_text,
            )
        } else {
            (palette.control_fill, palette.control_border, palette.text)
        };
        draw_chip_frame(painter, rect, CHIP_RADIUS, fill, border, 1.0);
        ink
    } else {
        if is_hovered || is_selected {
            let (fill, border) = if is_hovered {
                (palette.hover_fill, palette.hover_border)
            } else {
                (palette.active_fill, palette.active_border)
            };
            draw_chip_frame(painter, rect, CHIP_RADIUS, fill, border, 1.0);
        }
        if is_hovered {
            palette.hover_icon
        } else if is_selected {
            palette.strong_text
        } else {
            cfg.icon.get()
        }
    };

    let preview = Rect::from_xywh(
        rect.left() + 5.0,
        rect.top(),
        (rect.width() - 18.0).max(1.0),
        rect.height(),
    );
    let icon_size = (action.icon_size() * crate::ui::toolbar::metrics::properties_scale())
        .min(rect.width() - 8.0)
        .min(rect.height() - 6.0);

    // 2. 按具体工具绘制实时样式缩略或动作图标
    match action {
        SettingsAction::FilterModes => {
            if let Some(preview) = preview {
                crate::ui::filter_properties::preview(
                    painter,
                    panel.filter_mode,
                    preview,
                    tint.color(),
                );
            }
        }
        SettingsAction::ArrowStyles | SettingsAction::LineStyles => {
            if let Some(preview) = preview {
                let (kind, index) = if action == SettingsAction::ArrowStyles {
                    (LineChoice::Heads, panel.arrow_style.head_index())
                } else {
                    (LineChoice::Pattern, panel.line_style as usize)
                };
                line_properties::preview(painter, kind, index, preview, tint.color());
            }
        }
        SettingsAction::NumberStyles => {
            if let Some(preview) = preview {
                crate::renderer::number_text::draw_preview(
                    painter,
                    preview,
                    panel.number_style,
                    tint.color(),
                );
            }
        }
        SettingsAction::RulerModes => {
            if let Some(preview) = preview {
                painter.text(
                    preview,
                    ruler_label(panel.ruler_mode),
                    TextStyle::new(crate::ui::toolbar::metrics::font(), tint.color())
                        .weight(Weight::MEDIUM)
                        .center(),
                );
            }
        }
        SettingsAction::MarkerShapes => {
            if let Some(icon) = Rect::from_xywh(
                rect.left() + (rect.width() - icon_size - 9.0) * 0.5,
                rect.top() + (rect.height() - icon_size) * 0.5,
                icon_size,
                icon_size,
            ) {
                crate::features::marker::draw_icon(
                    painter,
                    panel.marker_shape,
                    icon,
                    tint.color(),
                    panel.marker_filled,
                );
            }
        }
        _ => {
            painter.icon(
                svg,
                icon_size,
                rect.left() + (rect.width() - icon_size) * 0.5,
                rect.top() + (rect.height() - icon_size) * 0.5,
                tint,
            );
            return;
        }
    }

    draw_caret(
        painter,
        rect,
        action == SettingsAction::MarkerShapes,
        palette.muted_text,
    );
}

/// 【副工具栏】【展开标记】在选择器芯片右侧绘制圆角下折线，无返回值
fn draw_caret(painter: &mut Painter<'_>, rect: Rect, compact: bool, tint: Rgba) {
    let cx = rect.right() - if compact { 6.5 } else { 8.5 };
    let cy = (rect.top() + rect.bottom()) * 0.5 + 0.5;
    let mut caret = PathBuilder::new();
    caret.move_to(cx - 2.6, cy - 1.3);
    caret.line_to(cx, cy + 1.4);
    caret.line_to(cx + 2.6, cy - 1.3);
    if let Some(path) = caret.finish() {
        painter.stroke_path(
            &path,
            tint.color(),
            &Stroke {
                width: 1.3,
                line_cap: tiny_skia::LineCap::Round,
                line_join: tiny_skia::LineJoin::Round,
                ..Stroke::default()
            },
        );
    }
}
