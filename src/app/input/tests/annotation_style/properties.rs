//! 【标注样式测试】【属性事务】原图数值与默认值独立于画布比例，撤销重做保存标注自身单位

use super::*;
use crate::{app::panels, ui::settings_panel::SettingsWidget};

/// 【标注样式测试】【单项属性】放大的小图属性显示六个原图像素，真实输入改为九后正确换算且历史可恢复
#[test]
fn single_property_input_reads_image_width_and_restores_units_in_history() {
    let mut state = image_editor((320, 240));
    let mut dirty = 0;
    draw(&mut state, Tool::Rectangle, 6.0, &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    state.select_annotation_index(Some(0));
    panels::settings::update_settings_panel(&mut state, &mut dirty);
    let index = state
        .settings_panel
        .widgets
        .iter()
        .position(|widget| matches!(widget, SettingsWidget::Stepper { .. }))
        .unwrap();
    assert_eq!(state.settings_panel.fields.values.get(&index).unwrap(), "6");
    let center = widget_center(&state, index);
    pointer(&mut state, center.0 - 12.0, center.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    crate::app::input::handle_modifiers(&mut state, true, false, &mut dirty);
    key(&mut state, "Ctrl+A", &mut dirty);
    crate::app::input::handle_key(&mut state, None, Some("9".into()), &mut dirty);
    crate::app::input::handle_modifiers(&mut state, false, false, &mut dirty);
    key(&mut state, "Enter", &mut dirty);
    close(state.annotations[0].stroke_width, 18.0, "canvas width");
    close(
        state.annotations[0].image_stroke_width(),
        9.0,
        "image width",
    );
    close(
        state.tool_settings.stroke(StrokeGroup::Rectangle),
        9.0,
        "default width",
    );
    key(&mut state, "Ctrl+Z", &mut dirty);
    close(state.annotations[0].stroke_width, 12.0, "undo");
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    close(state.annotations[0].stroke_width, 18.0, "redo");
    close(state.annotations[0].style_scale, 2.0, "history scale");
}

/// 【标注样式测试】【群组属性】群组成员按各自比例换算同一个原图宽度，首项属性和工具默认值保持原图数值
#[test]
fn collection_properties_use_each_members_own_image_units() {
    let mut state = image_editor((1280, 960));
    let mut dirty = 0;
    draw(&mut state, Tool::Rectangle, 6.0, &mut dirty);
    let mut second = state.annotations[0].scaled(2.0);
    second.id = state.next_id;
    second.set_image_stroke_width(4.0);
    state.annotations.push(second);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    let ids = state
        .annotations
        .iter()
        .map(|annotation| annotation.id)
        .collect::<Vec<_>>();
    state.select_annotation_ids(ids);
    panels::settings::update_settings_panel(&mut state, &mut dirty);
    close(
        state.settings_panel.text_points,
        25.0,
        "first image width plus nineteen",
    );
    assert!(crate::editor::annotation_collection::properties::set_width(
        &mut state, 9, true, &mut dirty
    ));
    close(state.annotations[0].stroke_width, 4.5, "first canvas width");
    close(
        state.annotations[1].stroke_width,
        9.0,
        "second canvas width",
    );
    for annotation in &state.annotations {
        close(annotation.image_stroke_width(), 9.0, annotation.id);
    }
    close(
        state.tool_settings.stroke(StrokeGroup::Rectangle),
        9.0,
        "collection default",
    );
    key(&mut state, "Ctrl+Z", &mut dirty);
    close(state.annotations[0].image_stroke_width(), 6.0, "first undo");
    close(
        state.annotations[1].image_stroke_width(),
        4.0,
        "second undo",
    );
}
