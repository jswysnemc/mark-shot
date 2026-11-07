//! 【文字输入测试】【属性事务】覆盖点字号、背景、默认隔离和缓存恢复，不加载识别模型

use super::*;
use crate::{
    editor::{
        annotation_drag::rebuild_annotation,
        text_edits::{self, Property},
    },
    features::text::TextAppearance,
    tools::Tool,
    types::{Annotation, AnnotationShape},
    ui::color_popover::ColorTarget,
};
use cosmic_text::Edit;
use tiny_skia::Rect;

/// 【文字输入测试】【文字创建】在 state 创建二十点文字，并选择属性对象；返回索引，使用内置字体
pub(super) fn add_text(state: &mut EditorState) -> usize {
    let index = state.annotations.len();
    state.annotations.push(Annotation {
        style_scale: 1.0,
        rotation_degrees: 0.0,
        frozen_bounds: None,
        id: 7,
        shape: AnnotationShape::Text {
            frame: Default::default(),
            start: (80.0, 100.0),
            content: "original".into(),
            font_size: 20.0,
            bold: true,
            italic: false,
            outline: None,
            appearance: TextAppearance {
                family: "Inter".into(),
                ..Default::default()
            },
        },
        color: Color::from_rgba8(200, 50, 60, 128),
        shadow_color: Color::TRANSPARENT,
        stroke_width: 0.0,
        bbox: Rect::from_xywh(80.0, 100.0, 1.0, 1.0).unwrap(),
    });
    state.select_annotation_index(Some(index));
    state.selected_tool = Tool::Text;
    rebuild_annotation(state, index);
    index
}

/// 【文字输入测试】【默认隔离】选中文字的字体家族与字形只修改自身，各操作可撤销重做，工具默认样式保持不变
#[test]
fn selected_text_properties_do_not_overwrite_tool_defaults() {
    let mut state = editor();
    let index = add_text(&mut state);
    let defaults = state.tool_settings.clone();
    let mut dirty = 0;
    for property in [
        Property::Family("Sans Serif".into()),
        Property::Bold(false),
        Property::Italic(true),
    ] {
        let before = state.annotations[index].clone();
        text_edits::set(&mut state, property, true, &mut dirty);
        let after = state.annotations[index].clone();
        assert_ne!(before, after);
        assert_eq!(state.tool_settings, defaults);
        state.undo(&mut dirty);
        assert_eq!(state.annotations[index], before);
        state.redo(&mut dirty);
        assert_eq!(state.annotations[index], after);
        assert_eq!(state.tool_settings, defaults);
    }
}

/// 【文字输入测试】【空操作】同值、空字体不创建历史，不清除已有重做分支；无对象时修改工具默认值
#[test]
fn no_op_properties_preserve_redo_and_unselected_edits_only_change_defaults() {
    let mut state = editor();
    add_text(&mut state);
    let mut dirty = 0;
    text_edits::set(&mut state, Property::Points(30.0), true, &mut dirty);
    state.undo(&mut dirty);
    assert_eq!(state.redo_stack.len(), 1);
    text_edits::set(&mut state, Property::Points(20.0), true, &mut dirty);
    text_edits::set(&mut state, Property::Family(" ".into()), true, &mut dirty);
    assert!(state.undo_stack.is_empty());
    assert_eq!(state.redo_stack.len(), 1);
    state.select_annotation_index(None);
    let before = state.annotations.clone();
    text_edits::set(&mut state, Property::Points(301.0), true, &mut dirty);
    text_edits::set(
        &mut state,
        Property::Family("Inter".into()),
        true,
        &mut dirty,
    );
    text_edits::set(&mut state, Property::Bold(false), true, &mut dirty);
    text_edits::set(&mut state, Property::Italic(true), true, &mut dirty);
    assert_eq!(state.tool_settings.font_size, 300.0);
    assert_eq!(state.tool_settings.text_family, "Inter");
    assert!(!state.tool_settings.bold);
    assert!(state.tool_settings.italic);
    assert_eq!(state.annotations, before);
    assert!(state.undo_stack.is_empty());
}

/// 【文字输入测试】【背景事务】一次打开内多次改背景共用一次撤销，正文颜色与工具默认值不变，撤销重做恢复边界和缓存
#[test]
fn background_picker_has_one_transaction_and_independent_defaults() {
    let mut state = editor();
    let index = add_text(&mut state);
    let original = state.annotations[index].clone();
    let defaults = state.tool_settings.clone();
    state.color_popover.open = true;
    state.color_popover.target = ColorTarget::TextBackground;
    let mut dirty = 0;
    for alpha in [64, 128, 192] {
        crate::editor::color_selection::apply_color_selection(
            &mut state,
            Color::from_rgba8(10, 20, 30, alpha),
            true,
            &mut dirty,
        );
    }
    let edited = state.annotations[index].clone();
    assert_eq!(edited.color, original.color);
    assert_eq!(edited.bbox, original.bbox);
    assert_eq!(state.tool_settings, defaults);
    assert!(state.undo_stack.is_empty());
    state.undo(&mut dirty);
    assert_eq!(state.annotations[index], original);
    assert!(state.undo_stack.is_empty());
    state.redo(&mut dirty);
    assert_eq!(state.annotations[index], edited);
    assert_eq!(state.tool_settings, defaults);
}

/// 【文字输入测试】【缓存恢复】撤销字号时恢复实际像素字号与文字框，不保留修改后的旧排版
#[test]
fn undo_restores_editor_metrics_and_text_bounds() {
    let mut state = editor();
    add_text(&mut state);
    let original = state.annotations[0].bbox;
    let mut dirty = 0;
    text_edits::set(&mut state, Property::Points(50.0), true, &mut dirty);
    assert_eq!(
        state.text.editors[&7].with_buffer(|buffer| buffer.metrics().font_size),
        67.0
    );
    assert!(state.annotations[0].bbox.width() > original.width());
    state.undo(&mut dirty);
    assert_eq!(
        state.text.editors[&7].with_buffer(|buffer| buffer.metrics().font_size),
        27.0
    );
    assert_eq!(state.annotations[0].bbox, original);
}

/// 【文字输入测试】【单行保持】没有换行的框宽不限制新字体尺寸，已经换行的框宽随字号比例缩放
#[test]
fn property_changes_keep_unwrapped_text_on_one_line() {
    let mut state = editor();
    add_text(&mut state);
    if let AnnotationShape::Text { frame, .. } = &mut state.annotations[0].shape {
        frame.width = Some(200.0);
    }
    rebuild_annotation(&mut state, 0);
    let mut dirty = 0;
    text_edits::set(&mut state, Property::Points(100.0), true, &mut dirty);
    assert_eq!(
        state.text.editors[&7].with_buffer(|buffer| buffer.layout_runs().count()),
        1
    );
    assert!(
        matches!(&state.annotations[0].shape, AnnotationShape::Text { frame, .. } if frame.width.is_none())
    );
    state.undo(&mut dirty);
    if let AnnotationShape::Text { frame, .. } = &mut state.annotations[0].shape {
        frame.width = Some(70.0);
    }
    rebuild_annotation(&mut state, 0);
    text_edits::set(&mut state, Property::Points(40.0), true, &mut dirty);
    assert!(
        matches!(&state.annotations[0].shape, AnnotationShape::Text { frame, .. } if frame.width == Some(140.0))
    );
    assert!(state.text.editors[&7].with_buffer(|buffer| buffer.layout_runs().count()) > 1);
}
