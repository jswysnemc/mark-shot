//! 【属性面板】【动作图标】绘制当前样式预览及展开标记，保持属性栏的统一尺寸

use tiny_skia::{PathBuilder, Rect, Stroke};

use super::{SettingsAction, SettingsPanel};
use crate::renderer::{Painter, text::TextStyle};
use crate::theme::{Rgba, font};
use crate::ui::line_properties::{self, LineChoice};

/// 【属性面板】【动作图标绘制】按 panel 当前属性将 action、svg 绘入 painter 的 rect，tint 指定颜色；无返回值
pub(super) fn draw(
    painter: &mut Painter<'_>,
    panel: &SettingsPanel,
    action: SettingsAction,
    svg: &'static str,
    rect: Rect,
    tint: Rgba,
) {
    let preview = Rect::from_xywh(
        rect.left() + 4.0,
        rect.top(),
        (rect.width() - 16.0).max(1.0),
        rect.height(),
    );
    let icon_size = action.icon_size();
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
                    (LineChoice::Heads, panel.arrow_style as usize)
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
                    panel.ruler_mode.label(),
                    TextStyle::new(font::label(), tint.color()).center(),
                );
            }
        }
        SettingsAction::MarkerShapes => {
            if let Some(icon) = Rect::from_xywh(
                rect.left() + (rect.width() - icon_size - 8.0) / 2.0,
                rect.top() + (rect.height() - icon_size) / 2.0,
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
                rect.left() + (rect.width() - icon_size) / 2.0,
                rect.top() + (rect.height() - icon_size) / 2.0,
                tint,
            );
            return;
        }
    }
    draw_caret(painter, rect, action == SettingsAction::MarkerShapes, tint);
}

/// 【属性面板】【展开标记绘制】在 painter 的 rect 右侧绘制 tint 色折线，marker 保留标记按钮原有位置；无返回值
fn draw_caret(painter: &mut Painter<'_>, rect: Rect, marker: bool, tint: Rgba) {
    let x = rect.right() - if marker { 7.0 } else { 10.0 };
    let y = (rect.top() + rect.bottom()) * 0.5;
    let mut caret = PathBuilder::new();
    caret.move_to(x, y - 1.5);
    caret.line_to(x + 3.0, y + 1.5);
    caret.line_to(x + 6.0, y - 1.5);
    if let Some(path) = caret.finish() {
        painter.stroke_path(
            &path,
            tint.color(),
            &Stroke {
                width: 1.2,
                ..Stroke::default()
            },
        );
    }
}
