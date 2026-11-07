//! 【线条测试】【绘图与定点】验证骨架创建、双击、正文外控制点、历史和手势取消

use super::line_support::{click, draw, line_editor, pick, skeleton};
use super::{key, pointer};
use crate::app::input::{handle_modifiers, handle_pointer_button, refresh_panels, select_tool};
use crate::editor::{DamageZone, line_edits};
use crate::features::{
    arrow::ArrowStyle,
    line::{LineStyle, drag, geometry},
};
use crate::tools::Tool;
use crate::types::{AnnotationShape, MouseButton, SelectionHandle, StrokeGroup};

/// 【线条测试】【创建组合】四种头部和三种线型通过真实手势提交，直线与旧编号箭头笔宽分组不变
#[test]
fn every_arrow_head_and_pattern_is_committed_by_the_shared_line_tool() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    assert_eq!(state.selected_tool, Tool::Arrow);
    for head in ArrowStyle::ALL {
        for pattern in LineStyle::ALL {
            state.tool_settings.arrow_style = head;
            state.tool_settings.arrow_line_style = pattern;
            draw(&mut state, (140.0, 180.0), (420.0, 220.0), &mut dirty);
            let annotation = state.annotations.last().unwrap();
            let AnnotationShape::Arrow {
                line,
                style,
                line_style,
            } = &annotation.shape
            else {
                panic!("arrow expected")
            };
            assert_eq!((*style, *line_style), (head, pattern));
            assert_eq!(line.start, (140.0, 180.0));
            assert_eq!(line.end, (420.0, 220.0));
            assert!(line.controls.is_empty());
            assert_eq!(annotation.stroke_width, 3.0);
        }
    }
    key(&mut state, "D", &mut dirty);
    assert_eq!(state.selected_tool, Tool::Line);
    for pattern in LineStyle::ALL {
        state.tool_settings.line_style = pattern;
        draw(&mut state, (150.0, 280.0), (410.0, 280.0), &mut dirty);
        assert!(
            matches!(state.annotations.last().unwrap().shape, AnnotationShape::Line { style, .. } if style == pattern)
        );
    }
    assert_eq!(state.annotations.len(), 15);
    assert_eq!(state.undo_stack.len(), 15);
    assert_eq!(state.tool_settings.stroke(StrokeGroup::Numbered), 11.0);
    assert_eq!(StrokeGroup::Arrow as usize, 2);
    assert_eq!(StrokeGroup::Line as usize, 3);
}

/// 【线条测试】【空手势隔离】空点击、右键和原图外起点不产生隐藏标注或空撤销，终点仍约束到原图
#[test]
fn empty_right_button_and_outside_image_gestures_do_not_commit() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Line, &mut dirty);
    state.ui_hidden = true;
    let viewport = tiny_skia::Rect::from_xywh(100.0, 100.0, 320.0, 200.0).unwrap();
    state.file_canvas = Some(crate::editor::file_canvas::FileCanvas {
        original: crate::image_source::ImageSource::memory(
            tiny_skia::Pixmap::new(320, 200).unwrap(),
        ),
        viewport,
    });
    state.selection.zone = Some(viewport);
    refresh_panels(&mut state, &mut dirty);
    draw(&mut state, (140.0, 180.0), (140.0, 180.0), &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, false, &mut dirty);
    assert_eq!(state.selected_tool, Tool::Pick);
    key(&mut state, "D", &mut dirty);
    draw(&mut state, (80.0, 180.0), (200.0, 200.0), &mut dirty);
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
    assert!(state.pending.is_none() && !state.input.mouse_down);
    draw(&mut state, (140.0, 180.0), (600.0, 400.0), &mut dirty);
    assert_eq!(skeleton(&state, 0).end, (420.0, 300.0));
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【线条测试】【修饰键吸附】创建中按下或释放 Shift 立即更新同一草稿，确认只生成一次历史
#[test]
fn shift_changes_the_live_endpoint_without_creating_extra_history() {
    for tool in [Tool::Line, Tool::Arrow] {
        let mut dirty = 0;
        let mut state = line_editor(tool, &mut dirty);
        pointer(&mut state, 140.0, 180.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 300.0, 310.0, &mut dirty);
        handle_modifiers(&mut state, false, true, &mut dirty);
        let line = state.pending.as_ref().unwrap().shape.line().unwrap();
        assert!((line.end.0 - line.start.0 - (line.end.1 - line.start.1)).abs() < 0.001);
        handle_modifiers(&mut state, false, false, &mut dirty);
        assert_eq!(
            state.pending.as_ref().unwrap().shape.line().unwrap().end,
            (300.0, 310.0)
        );
        assert!(state.undo_stack.is_empty());
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert_eq!(state.undo_stack.len(), 1);
    }
}

/// 【线条测试】【草稿取消】Escape、撤销和工具切换均丢弃草稿，不关闭编辑器、不撤销其他标注
#[test]
fn canceling_line_drafts_preserves_the_existing_history_and_session() {
    for tool in [Tool::Line, Tool::Arrow] {
        for cancel in ["Escape", "Ctrl+Z", "P"] {
            let mut dirty = 0;
            let mut state = line_editor(tool, &mut dirty);
            draw(&mut state, (140.0, 180.0), (420.0, 180.0), &mut dirty);
            let before = state.annotations.clone();
            pointer(&mut state, 140.0, 280.0, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
            pointer(&mut state, 420.0, 300.0, &mut dirty);
            key(&mut state, cancel, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
            assert_eq!(state.annotations, before);
            assert_eq!(state.undo_stack.len(), 1);
            assert!(state.pending.is_none() && state.ann_drag.is_none());
            assert!(!state.cancel && state.finish.is_none());
        }
    }
}

/// 【线条测试】【双击插入历史】首次双击路径插入原版索引控制点，删除只移除控制点，撤销清除临时删除目标
#[test]
fn double_click_and_delete_control_points_are_individual_history_transactions() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Arrow, &mut dirty);
    draw(&mut state, (140.0, 180.0), (420.0, 180.0), &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    state.ui_hidden = true;
    refresh_panels(&mut state, &mut dirty);
    click(&mut state, (240.0, 180.0), &mut dirty);
    assert!(skeleton(&state, 0).controls.is_empty());
    click(&mut state, (240.0, 180.0), &mut dirty);
    let id = state.annotations[0].id;
    assert_eq!(skeleton(&state, 0).controls, [(240.0, 180.0)]);
    assert_eq!(state.line_session.selected_point, Some((id, 2)));
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Delete", &mut dirty);
    assert_eq!(state.annotations.len(), 1);
    assert!(skeleton(&state, 0).controls.is_empty());
    assert_eq!(state.undo_stack.len(), 3);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(skeleton(&state, 0).controls, [(240.0, 180.0)]);
    assert_eq!(state.line_session.selected_point, None);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert!(skeleton(&state, 0).controls.is_empty());
    key(&mut state, "Ctrl+Z", &mut dirty);
    key(&mut state, "Delete", &mut dirty);
    assert!(state.annotations.is_empty());
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.annotations.len(), 1);
    assert_eq!(skeleton(&state, 0).controls.len(), 1);
}

/// 【线条测试】【曲线路由插入】控制点移出正文后仍可拖动，第二次双击真实曲线插入新控制点而非折线顶点
#[test]
fn external_controls_and_multiple_curve_insertions_follow_the_real_route() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Line, &mut dirty);
    draw(&mut state, (140.0, 180.0), (420.0, 180.0), &mut dirty);
    pick(&mut state, 0, &mut dirty);
    state.ui_hidden = true;
    refresh_panels(&mut state, &mut dirty);
    click(&mut state, (240.0, 180.0), &mut dirty);
    click(&mut state, (240.0, 180.0), &mut dirty);
    draw(&mut state, (240.0, 180.0), (220.0, 90.0), &mut dirty);
    assert_eq!(skeleton(&state, 0).controls, [(220.0, 90.0)]);
    assert!(!state.annotations[0].initial_hit_test((220.0, 90.0)));
    draw(&mut state, (220.0, 90.0), (230.0, 100.0), &mut dirty);
    assert_eq!(skeleton(&state, 0).controls, [(230.0, 100.0)]);
    assert_eq!(skeleton(&state, 0).start, (140.0, 180.0));
    assert_eq!(skeleton(&state, 0).end, (420.0, 180.0));
    let sampled = geometry::sampled(skeleton(&state, 0)).unwrap();
    let point = sampled.at(sampled.length * 0.75);
    click(&mut state, (point.0 as f64, point.1 as f64), &mut dirty);
    click(&mut state, (point.0 as f64, point.1 as f64), &mut dirty);
    assert_eq!(skeleton(&state, 0).controls.len(), 2);
    assert!(skeleton(&state, 0).controls.contains(&(230.0, 100.0)));
    let two_controls = state.annotations.clone();
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(skeleton(&state, 0).controls, [(230.0, 100.0)]);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(state.annotations, two_controls);
}

/// 【线条测试】【定点高亮重绘】同一标注仅切换定点时仍通知动态损坏区域，不修改持久图层或产生历史
#[test]
fn selecting_another_point_damages_selection_chrome_without_a_geometry_edit() {
    let mut dirty = 0;
    let mut state = line_editor(Tool::Line, &mut dirty);
    draw(&mut state, (140.0, 180.0), (420.0, 180.0), &mut dirty);
    pick(&mut state, 0, &mut dirty);
    state.ui_hidden = true;
    refresh_panels(&mut state, &mut dirty);
    assert!(line_edits::insert_control(
        &mut state,
        0,
        (240.0, 180.0),
        &mut dirty
    ));
    let before = state.annotations.clone();
    let id = state.annotations[0].id;
    pointer(&mut state, 140.0, 180.0, &mut dirty);
    state.damage_rects.clear();
    state.layer_damage_rects.clear();
    dirty = 0;
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert_eq!(state.line_session.selected_point, Some((id, 0)));
    assert_ne!(dirty, 0);
    assert!(
        state
            .damage_rects
            .iter()
            .any(|zone| matches!(zone, DamageZone::Global(_)))
    );
    assert!(state.layer_damage_rects.is_empty());
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(state.annotations, before);
    assert_eq!(state.undo_stack.len(), 2);
    assert!(!line_edits::remove_selected_control(&mut state, &mut dirty));
}

/// 【线条测试】【正文外外框】已选箭头和直线的缩放、旋转把手在正文之外仍开始正确手势，Escape 恢复快照
#[test]
fn external_frame_handles_and_rotation_cancel_without_deleting_the_annotation() {
    for tool in [Tool::Line, Tool::Arrow] {
        for expected in [SelectionHandle::TopRight, SelectionHandle::Rotate] {
            let mut dirty = 0;
            let mut state = line_editor(tool, &mut dirty);
            draw(&mut state, (140.0, 180.0), (420.0, 220.0), &mut dirty);
            pick(&mut state, 0, &mut dirty);
            state.ui_hidden = true;
            refresh_panels(&mut state, &mut dirty);
            let before = state.annotations.clone();
            let bbox = state.annotations[0].bbox;
            let position = if expected == SelectionHandle::Rotate {
                drag::rotation_point(skeleton(&state, 0), bbox)
            } else {
                let frame = drag::frame(bbox).unwrap();
                (frame.right(), frame.top())
            };
            assert!(!state.annotations[0].initial_hit_test((position.0 as f64, position.1 as f64)));
            pointer(&mut state, position.0 as f64, position.1 as f64, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
            assert_eq!(
                state.ann_drag.as_ref().map(|drag| drag.handle),
                Some(expected)
            );
            handle_modifiers(
                &mut state,
                expected != SelectionHandle::Rotate,
                expected == SelectionHandle::Rotate,
                &mut dirty,
            );
            pointer(
                &mut state,
                position.0 as f64 + 65.0,
                position.1 as f64 - 25.0,
                &mut dirty,
            );
            assert!(state.pending.is_some());
            key(&mut state, "Escape", &mut dirty);
            assert_eq!(state.annotations, before);
            assert!(state.pending.is_none() && state.ann_drag.is_none());
            assert_eq!(state.line_session.selected_point, None);
            assert_eq!(state.undo_stack.len(), 1);
            assert!(!state.cancel && state.finish.is_none());
        }
    }
}
