//! 【录屏副工具栏】【属性绘制】使用统一胶囊下拉芯片与主强调色开始按钮展示录屏参数

use super::{SettingsAction, SettingsPanel, SettingsWidget, draw_controls::draw_chip_frame};
use crate::{
    features::recording::tool_options::Parameter,
    renderer::{Painter, paths::rounded_rect_path, text::TextStyle},
    theme::toolbar::Palette,
    ui::language::Language,
};
use cosmic_text::Weight;
use tiny_skia::{Rect, Stroke};

const CHIP_RADIUS: f32 = 6.5;

pub(crate) const WIDGETS: &[SettingsWidget] = &[
    control(Parameter::Format),
    control(Parameter::Fps),
    control(Parameter::Countdown),
    control(Parameter::Audio),
    control(Parameter::Quality),
    control(Parameter::Encoder),
    control(Parameter::Cursor),
    SettingsWidget::Separator,
    SettingsWidget::Action {
        action: SettingsAction::StartRecording,
        svg: crate::ui::icons::VIDEO,
    },
];

/// 【录屏副工具栏】【参数控件】按 parameter 返回使用统一动作类型的属性控件
const fn control(parameter: Parameter) -> SettingsWidget {
    SettingsWidget::Action {
        action: SettingsAction::Recording(parameter),
        svg: crate::ui::icons::CHEVRON_DOWN,
    }
}

/// 【录屏副工具栏】【绘制分派】按 panel 参数在 rect 内绘制录屏芯片或开始按钮，返回是否已处理该动作
pub(super) fn draw(
    painter: &mut Painter<'_>,
    panel: &SettingsPanel,
    action: SettingsAction,
    rect: Rect,
    is_hovered: bool,
    is_selected: bool,
) -> bool {
    let cn = Language::current() == Language::Chinese;
    let palette = Palette::current();
    let font = crate::ui::toolbar::metrics::font();

    match action {
        SettingsAction::Recording(parameter) => {
            let config = &panel.recording;
            let enabled = parameter.enabled(config);
            let value = if enabled {
                parameter
                    .choices(config, cn)
                    .get(parameter.selected(config))
                    .cloned()
                    .unwrap_or_default()
            } else if cn {
                "关闭".into()
            } else {
                "Off".into()
            };

            // 1. 绘制参数下拉胶囊底座，禁用项自动降低文字与图标不透明度
            let (fill, border, mut ink) = if !enabled {
                (
                    palette.control_fill,
                    palette.control_border,
                    palette.muted_text.with_alpha(115),
                )
            } else if is_selected {
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
            if !enabled {
                ink = ink.with_alpha(110);
            }

            let text = format!("{} {value}", parameter.label(cn));
            if let Some(area) = Rect::from_xywh(
                rect.left() + 7.0,
                rect.top(),
                (rect.width() - 22.0).max(1.0),
                rect.height(),
            ) {
                let fitted = crate::ui::controls::fit_label(painter, &text, font, area.width());
                painter.text(
                    area,
                    &fitted,
                    TextStyle::new(font, ink.color())
                        .weight(Weight::MEDIUM)
                        .center(),
                );
            }
            painter.icon(
                crate::ui::icons::CHEVRON_DOWN,
                10.0,
                rect.right() - 14.0,
                rect.top() + (rect.height() - 10.0) * 0.5,
                if enabled {
                    palette.muted_text
                } else {
                    palette.muted_text.with_alpha(95)
                },
            );
            true
        }
        SettingsAction::StartRecording => {
            // 2. 开始录制按钮采用与主工具栏保存按钮一致的主强调色渐变胶囊与摄像机图标
            let ready = panel.recording_ready;
            if ready {
                let (top, bottom, border) = if is_hovered {
                    (
                        palette.primary_hover_top,
                        palette.primary_hover_bottom,
                        palette.primary_hover_border,
                    )
                } else {
                    (
                        palette.primary_top,
                        palette.primary_bottom,
                        palette.primary_border,
                    )
                };
                painter.fill_rounded_gradient(rect, CHIP_RADIUS, top.color(), bottom.color());
                if let Some(path) = rounded_rect_path(&rect, CHIP_RADIUS, true, true, true, true) {
                    painter.stroke_path(
                        &path,
                        border,
                        &Stroke {
                            width: 1.0,
                            ..Default::default()
                        },
                    );
                }
            } else {
                draw_chip_frame(
                    painter,
                    rect,
                    CHIP_RADIUS,
                    palette.control_fill,
                    palette.control_border,
                    1.0,
                );
            }

            let ink = if ready {
                palette.primary_text
            } else {
                palette.muted_text.with_alpha(120)
            };
            let label = if cn { "开始录制" } else { "Start" };
            let icon_size = 14.0;
            painter.icon(
                crate::ui::icons::VIDEO,
                icon_size,
                rect.left() + 10.0,
                rect.top() + (rect.height() - icon_size) * 0.5,
                ink,
            );
            if let Some(area) = Rect::from_xywh(
                rect.left() + 26.0,
                rect.top(),
                (rect.width() - 32.0).max(1.0),
                rect.height(),
            ) {
                let fitted = crate::ui::controls::fit_label(painter, label, font, area.width());
                painter.text(
                    area,
                    &fitted,
                    TextStyle::new(font, ink.color())
                        .weight(Weight::SEMIBOLD)
                        .center(),
                );
            }
            true
        }
        _ => false,
    }
}
