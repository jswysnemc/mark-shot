//! 【录屏工具】【属性绘制】使用现有属性控件尺寸和主题显示参数与开始按钮

use super::{SettingsAction, SettingsPanel, SettingsWidget};
use crate::{
    features::recording::tool_options::Parameter,
    renderer::{Painter, text::TextStyle},
    theme::Rgba,
};
use tiny_skia::Rect;

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

/// 【录屏工具】【参数控件】按 parameter 返回使用统一动作按钮的属性控件
const fn control(parameter: Parameter) -> SettingsWidget {
    SettingsWidget::Action {
        action: SettingsAction::Recording(parameter),
        svg: crate::ui::icons::CHEVRON_DOWN,
    }
}

/// 【录屏工具】【动作文字】按 panel 参数绘制 action 到 painter 的 rect，tint 指定颜色；返回是否处理
pub(super) fn draw(
    painter: &mut Painter<'_>,
    panel: &SettingsPanel,
    action: SettingsAction,
    rect: Rect,
    tint: Rgba,
) -> bool {
    let cn = crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
    let (text, enabled, caret) = match action {
        SettingsAction::Recording(parameter) => {
            let config = &panel.recording;
            let value = if parameter.enabled(config) {
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
            (
                format!("{} {value}", parameter.label(cn)),
                parameter.enabled(config),
                true,
            )
        }
        SettingsAction::StartRecording => (
            if cn {
                "开始录制"
            } else {
                "Start recording"
            }
            .into(),
            panel.recording_ready,
            false,
        ),
        _ => return false,
    };
    let mut color = tint.color();
    if !enabled {
        color.set_alpha(color.alpha() * 0.45);
    }
    let area = Rect::from_xywh(
        rect.left() + 5.0,
        rect.top(),
        rect.width() - if caret { 20.0 } else { 10.0 },
        rect.height(),
    )
    .unwrap();
    let font = crate::ui::toolbar::metrics::font();
    let text = crate::ui::controls::fit_label(painter, &text, font, area.width());
    painter.text(area, &text, TextStyle::new(font, color).center());
    if caret {
        painter.icon(
            crate::ui::icons::CHEVRON_DOWN,
            10.0,
            rect.right() - 13.0,
            rect.top() + (rect.height() - 10.0) / 2.0,
            tint,
        );
    }
    true
}
