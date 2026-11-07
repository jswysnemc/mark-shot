//! 【文字存储测试】【真实手势】连续操作必须保留原版存储矩形，即使可见选框没有变化

use super::*;
use crate::types::{AnnotationShape, MouseButton, SelectionHandle};

/// 【文字存储测试】【场景准备】按degrees返回固定Noto Sans二十点标注，尺寸与生产Qt输入相同，不连接桌面
fn prepared(degrees: f64) -> EditorState {
    let mut state = editor();
    super::text::add_text(&mut state);
    if let AnnotationShape::Text {
        start,
        content,
        appearance,
        ..
    } = &mut state.annotations[0].shape
    {
        *start = (80.0, 90.0);
        *content = "Rotation".into();
        appearance.family = "Noto Sans".into();
    }
    state.annotations[0].rotation_degrees = degrees;
    state.selected_tool = crate::tools::Tool::Pick;
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    crate::editor::annotation_drag::rebuild_annotation(&mut state, 0);
    state
}

/// 【文字存储测试】【生产读取】按name返回完整生产函数的连续手势记录，不由当前模型生成预期
fn sequence(name: &str) -> serde_json::Value {
    let values: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/qt-text-storage/reference.json"
    )))
    .unwrap();
    values["sequences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|sample| sample["name"] == name && sample["text"] == "Rotation")
        .unwrap()
        .clone()
}

/// 【文字存储测试】【边中点拖动】在state当前选框的handle上拖动delta，不直接修改宽高，返回值为空
fn resize(state: &mut EditorState, handle: SelectionHandle, delta: f32) {
    let annotation = &state.annotations[0];
    let frame = annotation.bbox;
    let center = annotation.rotation_center();
    let (start, end) = match handle {
        SelectionHandle::Right => ((frame.right(), center.1), (frame.right() + delta, center.1)),
        SelectionHandle::Bottom => (
            (center.0, frame.bottom()),
            (center.0, frame.bottom() + delta),
        ),
        _ => unreachable!(),
    };
    let start = annotation.to_visual(start);
    let end = annotation.to_visual(end);
    pointer(state, f64::from(start.0), f64::from(start.1), &mut 0);
    super::super::handle_pointer_button(state, MouseButton::Left, true, &mut 0);
    assert_eq!(state.ann_drag.as_ref().unwrap().handle, handle);
    pointer(state, f64::from(end.0), f64::from(end.1), &mut 0);
    super::super::handle_pointer_button(state, MouseButton::Left, false, &mut 0);
}

/// 【文字存储测试】【连续扩宽】第二次扩宽映射已有150像素矩形，不能仍保存150，原图选框继续保持126×45，无参数、无返回值
#[test]
fn repeated_widening_maps_the_stored_rectangle_like_production_qt() {
    let reference = sequence("widen-twice");
    for degrees in [0.0, 90.0] {
        let mut state = prepared(degrees);
        let before = state.annotations[0].clone();
        for step in 1..=2 {
            resize(&mut state, SelectionHandle::Right, 24.0);
            let AnnotationShape::Text { frame, .. } = &state.annotations[0].shape else {
                unreachable!()
            };
            let actual = frame.width.unwrap();
            let expected = reference["steps"][step]["after"]["stored"][2]
                .as_f64()
                .unwrap();
            assert!(
                (f64::from(actual) - expected).abs() < 0.0001,
                "step={step} actual={actual} expected={expected}"
            );
            assert_eq!(state.annotations[0].bbox, before.bbox);
        }
        assert_eq!(state.undo_stack.len(), 2);
        key(&mut state, "Ctrl+Z", &mut 0);
        key(&mut state, "Ctrl+Z", &mut 0);
        assert_eq!(state.annotations[0], before);
    }
}

/// 【文字存储测试】【隐藏高度历史】连续底边缩放不改变自然选框，但两次存储高度变化都必须记录，无参数、无返回值
#[test]
fn repeated_height_changes_keep_both_history_entries_when_the_frame_is_unchanged() {
    let mut state = prepared(90.0);
    let before = state.annotations[0].clone();
    for _ in 0..2 {
        resize(&mut state, SelectionHandle::Bottom, 24.0);
        assert_eq!(state.annotations[0].bbox, before.bbox);
    }
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Ctrl+Z", &mut 0);
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.annotations[0], before);
}

/// 【文字存储测试】【编辑收框历史】隐藏高度编辑提交收回自然框，保留身份和角度，只追加一次历史，无参数、无返回值
#[test]
fn unchanged_text_commit_fits_hidden_height_and_records_one_change() {
    let mut state = prepared(90.0);
    for _ in 0..2 {
        resize(&mut state, SelectionHandle::Bottom, 24.0);
    }
    let before = state.annotations[0].clone();
    assert!(before.text_stored_bounds().unwrap().height() > 100.0);
    assert!(crate::editor::text_session::begin_existing(
        &mut state, before.id, &mut 0
    ));
    crate::editor::text_session::commit(&mut state, &mut 0);
    assert_eq!(state.undo_stack.len(), 3);
    let after = state.annotations[0].clone();
    assert_eq!(after.id, before.id);
    assert_eq!(after.rotation_degrees, 90.0);
    assert_eq!(after.text_stored_bounds(), Some(after.bbox));
    crate::editor::text_session::begin_existing(&mut state, after.id, &mut 0);
    crate::editor::text_session::commit(&mut state, &mut 0);
    assert_eq!(state.undo_stack.len(), 3);
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.annotations[0], before);
    key(&mut state, "Ctrl+Y", &mut 0);
    assert_eq!(state.annotations[0], after);
}

/// 【文字存储测试】【编辑宽度隔离】连续扩宽保留隐藏矩形，重新编辑使用可见宽度，普通样式刷新不得覆盖它，无参数、无返回值
#[test]
fn editing_uses_visible_width_and_style_refresh_preserves_it() {
    let mut state = prepared(0.0);
    for _ in 0..2 {
        resize(&mut state, SelectionHandle::Right, 24.0);
    }
    let before = state.annotations[0].clone();
    crate::editor::text_session::begin_existing(&mut state, before.id, &mut 0);
    for _ in 0..3 {
        crate::editor::text_session::sync_style(&mut state, &mut 0);
        let working = &state.text_session.as_ref().unwrap().working;
        assert_eq!(
            working.text_stored_bounds().unwrap().width(),
            before.bbox.width()
        );
    }
    crate::editor::text_session::commit(&mut state, &mut 0);
    assert_eq!(
        state.annotations[0].text_stored_bounds().unwrap().width(),
        before.bbox.width()
    );
    assert_eq!(state.undo_stack.len(), 3);
}

/// 【文字存储测试】【显式换行】多个段落不等于段内换行，提交缩短正文后必须收回自然宽度，无参数、无返回值
#[test]
fn explicit_paragraph_breaks_do_not_preserve_an_unused_wrap_width_on_commit() {
    let mut state = prepared(0.0);
    resize(&mut state, SelectionHandle::Right, -50.0);
    let id = state.annotations[0].id;
    crate::editor::text_session::begin_existing(&mut state, id, &mut 0);
    key(&mut state, "Ctrl+A", &mut 0);
    crate::tools::text::session_input::text(&mut state, "a\nb", &mut 0);
    let editor = state
        .text
        .editors
        .get(&crate::editor::text_session::PREVIEW_ID)
        .unwrap();
    assert!(!crate::tools::text::has_wrapped_paragraph(editor));
    crate::editor::text_session::commit(&mut state, &mut 0);
    assert!(state.annotations[0].bbox.width() < 40.0);
    assert_eq!(
        state.annotations[0].text_stored_bounds(),
        Some(state.annotations[0].bbox)
    );
}

/// 【文字存储测试】【字号自然框】原始缺省宽度未改变时，显式字号操作仍同步自然框策略，不能沿用开始编辑的旧宽度造成换行，无参数、无返回值
#[test]
fn explicit_font_change_resets_the_editor_width_even_when_source_defaults_are_equal() {
    let mut state = prepared(0.0);
    let id = state.annotations[0].id;
    crate::editor::text_session::begin_existing(&mut state, id, &mut 0);
    let old = state.text_session.as_ref().unwrap().working.bbox;
    crate::editor::text_edits::set(
        &mut state,
        crate::editor::text_edits::Property::Points(30.0),
        true,
        &mut 0,
    );
    let working = &state.text_session.as_ref().unwrap().working;
    let AnnotationShape::Text { frame, .. } = &working.shape else {
        unreachable!()
    };
    assert_eq!(frame.width, None);
    let editor = state
        .text
        .editors
        .get(&crate::editor::text_session::PREVIEW_ID)
        .unwrap();
    assert!(!crate::tools::text::has_wrapped_paragraph(editor));
    assert!(working.bbox.width() > old.width());
    assert_eq!(working.bbox.height(), 63.0);
    crate::editor::text_session::commit(&mut state, &mut 0);
    assert_eq!(state.annotations[0].bbox.height(), 63.0);
}
