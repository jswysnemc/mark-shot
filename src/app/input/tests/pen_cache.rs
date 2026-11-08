//! 【画笔测试】【缓存一致性】验证增量笔迹提交与原始标注绘制使用相同透明度

use super::{editor, key, pointer};
use crate::app::input::{handle_pointer_button, select_tool};
use crate::editor::EditorState;
use crate::tools::Tool;
use crate::types::{Annotation, AnnotationShape, MarkerShape, MouseButton};
use tiny_skia::{Color, Pixmap, Rect};

/// 【画笔测试】【草稿构造】在 state 中生成有缓存片段的笔迹；dirty 返回重绘显示器，无返回值
fn draft(state: &mut EditorState, dirty: &mut u32) {
    select_tool(state, Tool::Pen, dirty);
    pointer(state, 140.0, 130.0, dirty);
    handle_pointer_button(state, MouseButton::Left, true, dirty);
    for index in 1..10 {
        pointer(
            state,
            140.0 + f64::from(index) * 12.0,
            130.0 + f64::from(index) * 5.0,
            dirty,
        );
    }
    assert!(state.pending_pen_baked > 0);
}

/// 【画笔测试】【半透明提交】缓存片段提交后与一次完整绘制逐像素一致，不重复叠加主线或阴影
#[test]
fn committing_a_translucent_pen_matches_a_fresh_annotation_layer() {
    let mut dirty = 0;
    let mut state = editor();
    state.tool_settings.color = Color::from_rgba8(255, 70, 20, 128);
    draft(&mut state, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(state.annotations.len(), 1);
    let mut expected = Pixmap::new(640, 480).unwrap();
    crate::renderer::rebuild_annotations_layer(
        &mut crate::renderer::Painter::new(&mut expected)
            .with_text(&mut state.text.font_system, &mut state.text.swash_cache),
        &state.annotations,
        &mut state.text.editors,
        &state.filters.images,
        None,
        None,
    );
    assert!(
        state.annotations_layer[0].data() == expected.data(),
        "the committed cached pen duplicates transparency or shadow compared with a fresh render"
    );
    assert!(state.pen_backup.is_empty());
}

/// 【画笔测试】【撤销草稿】撤销未完成画笔仅恢复原始像素，不产生空历史，并结束旧手势
#[test]
fn undoing_a_baked_draft_restores_the_pristine_layer_without_history() {
    let mut dirty = 0;
    let mut state = editor();
    draft(&mut state, &mut dirty);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert!(
        state.annotations_layer[0]
            .pixels()
            .iter()
            .all(|pixel| pixel.alpha() == 0)
    );
    assert!(state.pending.is_none());
    assert!(state.pen_backup.is_empty());
    assert_eq!(state.pending_pen_baked, 0);
    assert!(!state.annotations_dirty);
    assert!(!state.input.mouse_down);
    assert!(!state.tool_active);
    assert!(state.undo_stack.is_empty());
}

/// 【画笔测试】【分数缩放恢复】缓存取消保留已有局部光栅结果，不把其他标注重新绘制到不同的像素相位
#[test]
fn cancellation_preserves_existing_partial_rasters_at_fractional_scale() {
    let mut dirty = 0;
    let mut state = editor();
    state.placements[0].scale = 1.2;
    state.annotations_layer[0] = Pixmap::new(768, 576).unwrap();
    let color = Color::from_rgba8(40, 220, 180, 128);
    let mut annotation = Annotation {
        style_scale: 1.0,
        rotation_degrees: 0.0,
        frozen_bounds: None,
        id: 0,
        shape: AnnotationShape::Marker {
            radius: 0.0,
            start: (120.3, 128.7),
            end: (245.9, 238.4),
            kind: MarkerShape::Circle,
            filled: true,
        },
        color,
        shadow_color: crate::renderer::shadow_color_for(color),
        stroke_width: 3.0,
        bbox: Rect::from_xywh(120.3, 128.7, 125.6, 109.7).unwrap(),
    };
    annotation.update_bbox();
    state.bake_annotation(&annotation);
    state.annotations.push(annotation);
    crate::renderer::rebuild_annotations_layer(
        &mut crate::renderer::Painter::new(&mut state.annotations_layer[0]).with_scale(1.2),
        &state.annotations,
        &mut state.text.editors,
        &state.filters.images,
        None,
        Some(Rect::from_xywh(110.0, 100.0, 200.0, 180.0).unwrap()),
    );
    let before = state.annotations_layer[0].clone();
    let annotations = state.annotations.clone();
    draft(&mut state, &mut dirty);
    key(&mut state, "K", &mut dirty);
    assert!(
        state.annotations_layer[0].data() == before.data(),
        "draft cancellation changed an existing fractional-scale raster"
    );
    assert_eq!(state.annotations, annotations);
    assert!(state.pen_backup.is_empty());
    assert!(!state.annotations_dirty);
}

/// 【画笔测试】【右键取消】右键撤去当前草稿并恢复像素备份，旧左键释放不能提交已经取消的笔迹
#[test]
fn right_button_cancels_a_left_button_draft_without_committing_it() {
    let mut dirty = 0;
    let mut state = editor();
    draft(&mut state, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, false, &mut dirty);
    assert!(state.pending.is_none());
    assert!(state.pen_backup.is_empty());
    assert_eq!(state.selected_tool, Tool::Pick);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(state.annotations.is_empty());
    assert!(state.pen_backup.is_empty());
}
