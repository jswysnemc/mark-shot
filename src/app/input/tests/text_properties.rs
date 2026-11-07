//! 【文字属性测试】【真实路由】验证左键释放、右键保护、滑条事务、搜索与字号输入，不连接实际桌面

use super::*;
use crate::{
    fonts::font_catalog::{Family, FontCatalog},
    keys::{Chord, Key, Mods},
    tools::Tool,
    ui::{
        font_panel::state::{Field, Hit},
        text_properties::layout::{Control, TextLayout},
    },
};

/// 【文字属性测试】【编辑器准备】返回全图文字编辑器与文字对象，目录采用内置样本以隔离系统安装差异
fn prepared() -> EditorState {
    let mut state = editor();
    state.tool_settings.font_size = 20.0;
    super::text::add_text(&mut state);
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.selected_tool = Tool::Pick;
    state.font_panel.catalog = Some(FontCatalog {
        families: ["Inter", "Inter Bold", "Sample"]
            .map(|name| Family {
                name: name.into(),
                faces: Vec::new(),
            })
            .to_vec(),
    });
    super::super::refresh_panels(&mut state, &mut 0);
    state
}

/// 【文字属性测试】【控件指针】将 state 指针移到 control 中心，dirty 返回损坏区域，无返回值
fn control(state: &mut EditorState, control: Control, dirty: &mut u32) {
    let rect = state.settings_panel.widget_rect(control.index()).unwrap();
    pointer(
        state,
        (rect.left() + rect.width() / 2.0) as f64,
        (rect.top() + rect.height() / 2.0) as f64,
        dirty,
    );
}

/// 【文字属性测试】【左键点击】对 state 当前位置执行完整左键点击，dirty 返回损坏区域
fn click(state: &mut EditorState, dirty: &mut u32) {
    super::super::handle_pointer_button(state, crate::types::MouseButton::Left, true, dirty);
    super::super::handle_pointer_button(state, crate::types::MouseButton::Left, false, dirty);
}

/// 【文字属性测试】【字体打开】通过 state 真实按钮路由打开字体弹层，dirty 返回损坏区域
fn open(state: &mut EditorState, dirty: &mut u32) {
    control(state, Control::Font, dirty);
    click(state, dirty);
    assert!(state.font_panel.open);
}

/// 【文字属性测试】【按键路由】将 key、ctrl、text 传入 state，dirty 返回损坏区域，无返回值
fn key(state: &mut EditorState, key: Key, ctrl: bool, text: Option<&str>, dirty: &mut u32) {
    super::super::handle_key(
        state,
        Some(Chord {
            key,
            mods: Mods {
                ctrl,
                ..Default::default()
            },
        }),
        text.map(str::to_owned),
        dirty,
    );
}

#[test]
fn font_button_uses_left_release_and_right_never_activates() {
    let mut state = prepared();
    let mut dirty = 0;
    control(&mut state, Control::Font, &mut dirty);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        true,
        &mut dirty,
    );
    assert!(!state.font_panel.open);
    pointer(&mut state, 610.0, 350.0, &mut dirty);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        false,
        &mut dirty,
    );
    assert!(!state.font_panel.open);
    control(&mut state, Control::Font, &mut dirty);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Right,
        true,
        &mut dirty,
    );
    assert!(!state.font_panel.open);
    assert_eq!(state.annotations.len(), 1);
    open(&mut state, &mut dirty);
    control(&mut state, Control::Font, &mut dirty);
    click(&mut state, &mut dirty);
    assert!(!state.font_panel.open);
}

#[test]
fn slider_drag_records_changes_and_syncs_default_points() {
    let mut state = prepared();
    let mut dirty = 0;
    let original = state.annotations[0].clone();
    let cell = state
        .settings_panel
        .widget_rect(Control::Size.index())
        .unwrap();
    let handle = TextLayout::slider_handle(cell, 1, (-11, 281));
    pointer(
        &mut state,
        (handle.left() + 7.0) as f64,
        (handle.top() + 6.0) as f64,
        &mut dirty,
    );
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        true,
        &mut dirty,
    );
    for x in [cell.left() + 30.0, cell.left() + 50.0, cell.right() + 30.0] {
        pointer(&mut state, x as f64, (cell.top() + 7.0) as f64, &mut dirty);
    }
    assert_eq!(state.undo_stack.len(), 3);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        false,
        &mut dirty,
    );
    assert_eq!(state.undo_stack.len(), 3);
    assert_eq!(crate::editor::text_edits::current_points(&state), 300.0);
    assert_eq!(state.tool_settings.font_size, 300.0);
    for _ in 0..3 {
        state.undo(&mut dirty);
    }
    assert_eq!(state.annotations[0], original);
    assert_eq!(state.tool_settings.font_size, 300.0);
}

#[test]
fn opacity_slider_does_not_change_background_or_default_color() {
    let mut state = prepared();
    let mut dirty = 0;
    let defaults = state.tool_settings.clone();
    let before = state.annotations[0].clone();
    let cell = state
        .settings_panel
        .widget_rect(Control::Opacity.index())
        .unwrap();
    pointer(
        &mut state,
        (cell.left() + 1.0) as f64,
        (cell.top() + 7.0) as f64,
        &mut dirty,
    );
    click(&mut state, &mut dirty);
    assert!(state.annotations[0].color.alpha() < before.color.alpha());
    assert_eq!(state.tool_settings, defaults);
    assert_eq!(state.annotations[0].shape, before.shape);
    state.undo(&mut dirty);
    assert_eq!(state.annotations[0], before);
}

#[test]
fn font_search_and_shortcuts_do_not_edit_or_export_the_annotation() {
    let mut state = prepared();
    let mut dirty = 0;
    let before = state.annotations.clone();
    open(&mut state, &mut dirty);
    key(&mut state, Key::Char('s'), false, Some("sam"), &mut dirty);
    assert_eq!(state.font_panel.filtered, vec![2]);
    key(&mut state, Key::Enter, false, None, &mut dirty);
    key(&mut state, Key::Char('s'), true, None, &mut dirty);
    assert!(state.font_panel.open);
    assert_eq!(state.annotations, before);
    assert!(state.finish.is_none());
    key(&mut state, Key::Escape, false, None, &mut dirty);
    assert!(!state.font_panel.open);
    assert!(!state.cancel);
    open(&mut state, &mut dirty);
    assert_eq!(state.font_panel.query(), "");
    assert_eq!(state.font_panel.filtered.len(), 3);
}

#[test]
fn size_input_applies_valid_values_immediately_and_keeps_intermediate_text() {
    let mut state = prepared();
    let mut dirty = 0;
    open(&mut state, &mut dirty);
    let mut defaults = state.tool_settings.clone();
    defaults.font_size = 42.0;
    key(&mut state, Key::Tab, false, None, &mut dirty);
    assert!(state.font_panel.fields.is_editing_key(Field::Size));
    key(&mut state, Key::Char('4'), false, Some("4"), &mut dirty);
    assert_eq!(crate::editor::text_edits::current_points(&state), 20.0);
    key(&mut state, Key::Char('2'), false, Some("2"), &mut dirty);
    assert_eq!(crate::editor::text_edits::current_points(&state), 42.0);
    key(&mut state, Key::Char('0'), false, Some("0"), &mut dirty);
    assert_eq!(
        state.font_panel.fields.editing.as_ref().unwrap().field.text,
        "42 pt"
    );
    key(&mut state, Key::Char('a'), true, None, &mut dirty);
    key(&mut state, Key::Backspace, false, None, &mut dirty);
    assert_eq!(crate::editor::text_edits::current_points(&state), 42.0);
    assert_eq!(
        state.font_panel.fields.editing.as_ref().unwrap().field.text,
        " pt"
    );
    assert_eq!(state.tool_settings, defaults);
}

#[test]
fn selected_family_requires_release_on_same_row_and_closes_panel() {
    let mut state = prepared();
    let mut dirty = 0;
    open(&mut state, &mut dirty);
    let defaults = state.tool_settings.clone();
    let rect = state.font_panel.layout.row(2, 0.0, false);
    let position = state.font_panel.render_pos;
    pointer(
        &mut state,
        (position.0 + rect.left() + 45.0) as f64,
        (position.1 + rect.top() + 14.0) as f64,
        &mut dirty,
    );
    assert_eq!(
        state.font_panel.hit(state.input.pointer.local),
        Some(Hit::Family(2))
    );
    click(&mut state, &mut dirty);
    assert!(!state.font_panel.open);
    assert_eq!(state.tool_settings, defaults);
    assert!(
        matches!(&state.annotations[0].shape,crate::types::AnnotationShape::Text { appearance,.. } if appearance.family=="Sample")
    );
    assert_eq!(state.undo_stack.len(), 1);
}

#[test]
fn text_size_shortcuts_do_not_depend_on_removed_stepper_widgets() {
    let mut state = prepared();
    let mut dirty = 0;
    let original = state.annotations[0].clone();
    key(&mut state, Key::Char(']'), false, None, &mut dirty);
    assert_eq!(crate::editor::text_edits::current_points(&state), 21.0);
    state.undo(&mut dirty);
    assert_eq!(state.annotations[0], original);
    state.select_annotation_index(None);
    state.selected_tool = Tool::Text;
    super::super::refresh_panels(&mut state, &mut dirty);
    let points = state.tool_settings.font_size;
    key(&mut state, Key::Char(']'), false, None, &mut dirty);
    assert_eq!(state.tool_settings.font_size, points + 1.0);
}

#[test]
fn canvas_scroll_still_adjusts_text_after_stepper_removal() {
    let mut state = prepared();
    let mut dirty = 0;
    let original = state.annotations[0].clone();
    pointer(&mut state, 300.0, 300.0, &mut dirty);
    assert!(matches!(
        super::super::hit_test_ui(&state, state.input.pointer.local),
        super::super::UiHit::None
    ));
    super::super::handle_axis_scroll(
        &mut state,
        crate::types::ScrollEvent::wheel(120),
        &mut dirty,
    );
    assert_eq!(crate::editor::text_edits::current_points(&state), 21.0);
    assert!(state.text_size_history.pending.is_some());
    state.undo(&mut dirty);
    assert_eq!(state.annotations[0], original);
}
