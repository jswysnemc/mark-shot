//! 【尺子工具条测试】【详情配置】验证工具栏操作、预览、导出和撤销使用同一组开关

use super::*;
use crate::{
    features::ruler::{Readout, Readouts},
    ui::settings_panel::ToggleField,
};

/// 【尺子工具条测试】【开关定位】按 state 和 field 返回实际读数开关索引，折叠或其他尺子形态时返回空值
fn index(state: &EditorState, field: Readout) -> Option<usize> {
    state.settings_panel.widgets.iter().position(|widget| matches!(widget, SettingsWidget::Toggle { field: ToggleField::RulerReadout(current), .. } if *current == field))
}

/// 【尺子工具条测试】【单击开关】在 state 中点击 field，dirty 接收重绘显示器，无返回值
fn toggle(state: &mut EditorState, field: Readout, dirty: &mut u32) {
    let center = widget_center(state, index(state, field).unwrap());
    click(state, center, dirty);
}

/// 【尺子工具条测试】【折叠与回显】无参数，验证默认隐藏、按项修改、导出恢复以及撤销，无返回值
#[test]
fn ruler_readouts_edit_selected_annotation_and_clear_the_last_card() {
    let mut dirty = 0;
    let mut state = ruler_editor(RulerMode::Length, &mut dirty);
    draw(&mut state, (140.0, 130.0), (280.0, 240.0), &mut dirty);
    state.select_annotation_index(Some(0));
    panels::settings::update_settings_panel(&mut state, &mut dirty);
    let baseline = crate::app::render_final(&mut state).unwrap().0;
    crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
    assert!(state.ruler.labels.is_empty() && index(&state, Readout::Width).is_none());
    let opener = state
        .settings_panel
        .widgets
        .iter()
        .position(|widget| {
            matches!(
                widget,
                SettingsWidget::Action {
                    action: SettingsAction::RulerReadouts,
                    ..
                }
            )
        })
        .unwrap();
    let center = widget_center(&state, opener);
    click(&mut state, center, &mut dirty);
    assert_eq!(
        state.settings_panel.ruler_metrics,
        measurement(&state, 0).length_metrics()
    );
    assert_eq!(state.undo_stack.len(), 1);

    // 1. 【尺子工具条测试】【局部读数】只启用宽度和面积，预览与实际导出同步改变
    toggle(&mut state, Readout::Width, &mut dirty);
    toggle(&mut state, Readout::Area, &mut dirty);
    crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
    assert_eq!(
        state.ruler.labels[0].lines,
        ["width 140 px", "area 15400 px"]
    );
    assert_ne!(crate::app::render_final(&mut state).unwrap().0, baseline);
    toggle(&mut state, Readout::Width, &mut dirty);
    toggle(&mut state, Readout::Area, &mut dirty);
    crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
    assert!(state.ruler.labels.is_empty());
    assert_eq!(crate::app::render_final(&mut state).unwrap().0, baseline);
    state.undo(&mut dirty);
    panels::settings::update_settings_panel(&mut state, &mut dirty);
    assert!(
        state
            .settings_panel
            .is_toggled(index(&state, Readout::Area).unwrap())
    );

    // 2. 【尺子工具条测试】【默认设置】撤销已有标注不回拨工具偏好；后续尺子按最新默认值创建
    draw(&mut state, (120.0, 300.0), (320.0, 350.0), &mut dirty);
    assert_eq!(measurement(&state, 1).readouts, Readouts::default());
    assert!(measurement(&state, 0).readouts.area);
    toggle(&mut state, Readout::Height, &mut dirty);
    assert!(state.tool_settings.ruler_readouts.height);
    let center = widget_center(&state, opener);
    click(&mut state, center, &mut dirty);
    assert!(index(&state, Readout::Height).is_none());
    draw(&mut state, (360.0, 290.0), (420.0, 350.0), &mut dirty);
    assert_eq!(
        measurement(&state, 2).info_text().as_deref(),
        Some("height 60 px")
    );
    assert_eq!(measurement(&state, 1).readouts, Readouts::default());
}

/// 【尺子工具条测试】【模式隔离】无参数，角度尺和箭头尺不显示长度读数入口，无返回值
#[test]
fn ruler_readouts_are_only_offered_for_standard_length() {
    for (mode, style) in [
        (
            RulerMode::Angle,
            crate::features::ruler::RulerStyle::Standard,
        ),
        (
            RulerMode::Length,
            crate::features::ruler::RulerStyle::Simple,
        ),
    ] {
        let mut state = ruler_editor(mode, &mut 0);
        state.tool_settings.ruler_style = style;
        panels::settings::update_settings_panel(&mut state, &mut 0);
        assert!(!state.settings_panel.widgets.iter().any(|widget| matches!(
            widget,
            SettingsWidget::Action {
                action: SettingsAction::RulerReadouts,
                ..
            }
        )));
    }
}
