//! 【尺子工具条】【测量信息】折叠读数开关，并在选中刻度尺时显示实际数值

use super::{
    SettingsAction, SettingsPanel, SettingsWidget, StepperRange, ToggleField, ToggleVisual,
};
use crate::{
    features::ruler::{Readout, RulerMode, RulerStyle},
    ui::{color_popover::ColorTarget, language::Language, panel::PanelItem},
};

/// 【尺子工具条】【字段开关】按 field 返回控制该读数是否出现在画布上的按钮
const fn toggle(field: Readout) -> SettingsWidget {
    SettingsWidget::Toggle {
        visual: ToggleVisual::Checkbox {
            label: field.label(),
        },
        field: ToggleField::RulerReadout(field),
    }
}

const CONTROLS: &[SettingsWidget] = &[
    SettingsWidget::ColorSwatch(ColorTarget::Main),
    SettingsWidget::Action {
        action: SettingsAction::RulerModes,
        svg: crate::ui::icons::RULER,
    },
    SettingsWidget::Toggle {
        visual: ToggleVisual::Checkbox {
            label: "Arrow style",
        },
        field: ToggleField::RulerSimple,
    },
    SettingsWidget::Separator,
    SettingsWidget::Stepper {
        label: "",
        range: StepperRange::Stroke,
        unit: "px",
    },
    SettingsWidget::Action {
        action: SettingsAction::RulerReadouts,
        svg: crate::ui::icons::CHEVRON_DOWN,
    },
    toggle(Readout::Width),
    toggle(Readout::Height),
    toggle(Readout::Diagonal),
    toggle(Readout::Area),
];

/// 【尺子工具条】【可见控件】按 mode、style 和展开状态返回控件；非标准长度尺不提供详情开关
pub(crate) fn widgets(
    mode: RulerMode,
    style: RulerStyle,
    expanded: bool,
) -> &'static [SettingsWidget] {
    if (mode, style) != (RulerMode::Length, RulerStyle::Standard) {
        &CONTROLS[..5]
    } else if !expanded {
        &CONTROLS[..6]
    } else {
        CONTROLS
    }
}

impl SettingsPanel {
    /// 【尺子工具条】【读数文案】按 field 返回选中尺子的本地化数值标签；未选中时不附加数值
    pub(super) fn readout_caption(&self, field: ToggleField) -> Option<String> {
        let ToggleField::RulerReadout(field) = field else {
            return None;
        };
        let (value, unit) = field.value(self.ruler_metrics?);
        Some(format!(
            "{} {value} {unit}",
            Language::current().text(field.label())
        ))
    }

    /// 【属性工具条】【数值宽度】按 widget 和选中尺子的读数分配空间，普通控件沿用自身宽度
    pub(super) fn widget_width(&self, widget: &SettingsWidget) -> f32 {
        let width = widget.size();
        let (
            SettingsWidget::Toggle {
                field: ToggleField::RulerReadout(field),
                ..
            },
            Some(metrics),
        ) = (widget, self.ruler_metrics)
        else {
            return width;
        };
        let (value, unit) = field.value(metrics);
        let digits = value.checked_ilog10().unwrap_or(0) + 1;
        width
            + (digits as usize + 2 + unit.chars().count()) as f32
                * crate::config::get().settings_panel.char_width
                * crate::ui::toolbar::metrics::properties_scale()
    }
}
