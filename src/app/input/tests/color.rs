//! 【颜色输入测试】【原版职责】验证按下释放、颜色默认隔离与一次打开的历史事务

use super::{editor, pointer, widget_center};
use crate::{
    app::{
        input::{compute_cursor, handle_pointer_button, refresh_panels, select_tool},
        panels::color,
    },
    editor::EditorState,
    tools::Tool,
    types::{CursorIcon, MouseButton},
    ui::{
        color_popover::{ColorField, ColorTarget, parse_hex_color},
        panel::UiPanel,
        radial_palette::{COLORS, RadialPalette},
        settings_panel::SettingsWidget,
    },
};

/// 【颜色输入测试】【点击辅助】在 state 的 point 处完整单击 button；dirty 返回输入损坏区域
fn click(state: &mut EditorState, point: (f64, f64), button: MouseButton, dirty: &mut u32) {
    pointer(state, point.0, point.1, dirty);
    handle_pointer_button(state, button, true, dirty);
    handle_pointer_button(state, button, false, dirty);
}

/// 【颜色输入测试】【按钮定位】返回 state 主颜色按钮的实际中心坐标
fn button(state: &EditorState) -> (f64, f64) {
    let index = state
        .settings_panel
        .widgets
        .iter()
        .position(|item| matches!(item, SettingsWidget::ColorSwatch(ColorTarget::Main)))
        .unwrap();
    widget_center(state, index)
}

/// 【颜色输入测试】【标注环境】返回含一个已选矩形的编辑器，dirty 接收初始化损坏区域
fn selected(dirty: &mut u32) -> EditorState {
    let mut state = editor();
    select_tool(&mut state, Tool::Rectangle, dirty);
    pointer(&mut state, 300.0, 280.0, dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, dirty);
    pointer(&mut state, 440.0, 380.0, dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, dirty);
    assert_eq!(state.annotations.len(), 1);
    select_tool(&mut state, Tool::Pick, dirty);
    state.select_annotation_index(Some(0));
    refresh_panels(&mut state, dirty);
    state.undo_stack.clear();
    state
}

/// 【颜色输入测试】【按钮确认】左键按下不打开，移出释放不打开，同按钮点击只切换一次
#[test]
fn property_button_requires_release_and_closes_without_reopening() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    let point = button(&state);
    pointer(&mut state, point.0, point.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert!(!state.color_popover.open);
    pointer(&mut state, 5.0, 460.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(!state.color_popover.open);
    click(&mut state, point, MouseButton::Left, &mut dirty);
    assert!(state.color_popover.open && state.color_popover.radial.is_none());
    click(&mut state, point, MouseButton::Left, &mut dirty);
    assert!(!state.color_popover.open);
}

/// 【颜色输入测试】【非左键取消】右键打断尚未释放的属性按钮，后续左键释放不能打开弹层
#[test]
fn right_button_interrupts_an_armed_property_button() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    let point = button(&state);
    pointer(&mut state, point.0, point.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(!state.color_popover.open && state.input.color_button_press.is_none());
}

/// 【颜色输入测试】【双击分组】连续四次右键构成两组双击，第三次不重复切换，色板位置固定于第二次指针
#[test]
fn four_right_clicks_form_two_non_overlapping_double_clicks() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    for (index, expected) in [false, true, true, false].into_iter().enumerate() {
        click(&mut state, (310.0, 250.0), MouseButton::Right, &mut dirty);
        assert_eq!(state.color_popover.open, expected, "click {}", index + 1);
        if expected {
            assert!(state.color_popover.radial.is_some());
            assert_eq!(state.color_popover.render_pos, (221.0, 161.0));
        }
    }
}

/// 【颜色输入测试】【快捷确认】快捷按钮按下不改色，同位置释放修改默认色与选中标注并关闭，只记录一次历史
#[test]
fn radial_button_applies_to_default_and_selection_only_on_release() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    let before = state.annotations.clone();
    crate::app::panels::radial_palette::toggle(&mut state, &mut dirty);
    let rect = state.color_popover.rect().unwrap();
    let cell = RadialPalette::button(4).unwrap();
    let point = (
        (rect.left() + cell.left() + 14.0) as f64,
        (rect.top() + cell.top() + 14.0) as f64,
    );
    pointer(&mut state, point.0, point.1, &mut dirty);
    assert_eq!(compute_cursor(&state), CursorIcon::Default);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert_eq!(state.annotations, before);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(state.annotations[0].color, COLORS[4].color());
    assert_eq!(state.tool_settings.color, COLORS[4].color());
    assert!(!state.color_popover.open);
    assert_eq!(state.undo_stack.len(), 1);
    state.undo(&mut dirty);
    assert_eq!(state.annotations, before);
    assert_eq!(state.tool_settings.color, COLORS[4].color());
}

/// 【颜色输入测试】【取消快捷】快捷按钮移出后释放不改色、不关闭色板、也不生成历史
#[test]
fn radial_button_release_outside_cancels_the_choice() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    let before = state.annotations.clone();
    crate::app::panels::radial_palette::toggle(&mut state, &mut dirty);
    let rect = state.color_popover.rect().unwrap();
    pointer(
        &mut state,
        rect.left() as f64 + 88.0,
        rect.top() as f64 + 20.0,
        &mut dirty,
    );
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 5.0, 460.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(state.annotations, before);
    assert!(state.color_popover.open && state.undo_stack.is_empty());
}

/// 【颜色输入测试】【整次事务】HSV、alpha 与文本共用一次打开的快照，属性取色不改默认色，撤销重做精确恢复
#[test]
fn detailed_picker_groups_multiple_gestures_without_changing_defaults() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    let before = state.annotations.clone();
    let default = state.tool_settings.color;
    color::toggle(&mut state, ColorTarget::Main, &mut dirty);
    let rect = state.color_popover.rect().unwrap();
    for offset in [(60.0, 45.0), (110.0, 130.0), (140.0, 150.0)] {
        click(
            &mut state,
            (
                (rect.left() + offset.0) as f64,
                (rect.top() + offset.1) as f64,
            ),
            MouseButton::Left,
            &mut dirty,
        );
        assert!(state.undo_stack.is_empty());
    }
    assert_eq!(state.tool_settings.color, default);
    assert_ne!(state.annotations, before);
    assert!(state.color_popover.pre_edit_snapshot.is_some());
    let changed = state.annotations.clone();
    color::close_color_popover(&mut state, &mut dirty);
    assert_eq!(state.undo_stack.len(), 1);
    state.undo(&mut dirty);
    assert_eq!(state.annotations, before);
    state.redo(&mut dirty);
    assert_eq!(state.annotations, changed);
}

/// 【颜色输入测试】【文本确认】输入不逐字符修改标注，确认接受 RGB、简写与 RGBA；无效文本恢复原值
#[test]
fn hex_input_commits_only_on_confirmation_and_rejects_invalid_values() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    color::toggle(&mut state, ColorTarget::Main, &mut dirty);
    let mut previous = state.annotations[0].color;
    for value in ["#abc", "#123ABC7F", "12!", "#125"] {
        state.color_popover.fields.begin_edit(
            ColorField::Hex,
            String::new(),
            crate::ui::text_field::CursorInit::End,
        );
        for character in value.chars() {
            color::handle_color_field_text_input(&mut state, character, &mut dirty);
        }
        assert_eq!(state.annotations[0].color, previous);
        color::commit_color_field_edit(&mut state, &mut dirty);
        let filtered: String = value
            .chars()
            .filter(|c| c.is_ascii_hexdigit() || *c == '#')
            .collect();
        previous = parse_hex_color(&filtered).unwrap_or(previous);
        assert_eq!(state.annotations[0].color, previous);
    }
    assert!(state.undo_stack.is_empty());
    color::close_color_popover(&mut state, &mut dirty);
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【颜色输入测试】【光标分工】SV、滑条、文本与空白使用原版光标；拖动离开滑条仍保持手形光标并正常释放
#[test]
fn detailed_picker_uses_per_control_cursors_and_releases_outside() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    color::toggle(&mut state, ColorTarget::Main, &mut dirty);
    let rect = state.color_popover.rect().unwrap();
    for (offset, cursor) in [
        ((50.0, 50.0), CursorIcon::Crosshair),
        ((100.0, 130.0), CursorIcon::Pointer),
        ((100.0, 150.0), CursorIcon::Pointer),
        ((100.0, 175.0), CursorIcon::Text),
        ((4.0, 4.0), CursorIcon::Default),
    ] {
        pointer(
            &mut state,
            (rect.left() + offset.0) as f64,
            (rect.top() + offset.1) as f64,
            &mut dirty,
        );
        assert_eq!(compute_cursor(&state), cursor);
    }
    pointer(
        &mut state,
        rect.left() as f64 + 60.0,
        rect.top() as f64 + 150.0,
        &mut dirty,
    );
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 630.0, 460.0, &mut dirty);
    assert_eq!(compute_cursor(&state), CursorIcon::Pointer);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(!state.color_popover.alpha_dragging);
    assert_eq!(state.annotations[0].color.to_color_u8().alpha(), 255);
}

/// 【颜色输入测试】【跨屏轨道】拖动进入右侧显示器仍按原面板坐标取值，不因本地坐标归零把 alpha 变为零
#[test]
fn track_drags_keep_the_picker_coordinate_space_across_outputs() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    color::toggle(&mut state, ColorTarget::Main, &mut dirty);
    state.color_popover.alpha_dragging = true;
    state.input.pointer.local = (10.0, 150.0);
    state.input.pointer.global = (650.0, 150.0);
    state.input.pointer.monitor_idx = 1;
    color::handle_color_popover_drag(&mut state, &mut dirty);
    assert_eq!(state.color_popover.sv_square.alpha, 255);
}

/// 【颜色输入测试】【空编辑隔离】同色确认不产生撤销快照，也不清除现有重做记录
#[test]
fn confirming_the_existing_color_keeps_redo_and_creates_no_history() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    state.redo_stack.push(state.snapshot());
    let color = state.annotations[0].color;
    color::toggle(&mut state, ColorTarget::Main, &mut dirty);
    crate::editor::edits::apply_color_selection(&mut state, color, true, &mut dirty);
    color::close_color_popover(&mut state, &mut dirty);
    assert!(state.undo_stack.is_empty());
    assert_eq!(state.redo_stack.len(), 1);
}

/// 【颜色输入测试】【确认焦点】颜色字段连续两次 Enter 保持输入焦点，不调用全局导出，失去焦点才结束字段编辑
#[test]
fn enter_keeps_hex_focus_and_never_leaks_to_global_export() {
    let mut dirty = 0;
    let mut state = selected(&mut dirty);
    color::toggle(&mut state, ColorTarget::Main, &mut dirty);
    state.color_popover.fields.begin_edit(
        ColorField::Hex,
        "#abcdef".into(),
        crate::ui::text_field::CursorInit::End,
    );
    for _ in 0..2 {
        super::key(&mut state, "Enter", &mut dirty);
        assert!(state.color_popover.fields.is_editing());
        assert!(state.finish.is_none());
        assert_eq!(
            state.annotations[0].color,
            parse_hex_color("#ABCDEF").unwrap()
        );
    }
    color::close_color_popover(&mut state, &mut dirty);
    assert!(!state.color_popover.fields.is_editing());
    assert_eq!(state.undo_stack.len(), 1);
}
