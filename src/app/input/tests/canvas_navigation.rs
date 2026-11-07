//! 【画布导航测试】【实际流程】验证屏幕输入、原始像素、持久几何和输出损坏使用同一视图

use super::{editor, pointer};
use crate::{
    app::{
        frame_render::{Frame, render_monitor},
        input::{handle_axis_scroll, handle_modifiers, handle_pointer_button, select_tool},
    },
    editor::EditorState,
    tools::Tool,
    types::{AnnotationShape, MouseButton, ScrollEvent},
};
use tiny_skia::{Color, Pixmap, Rect};

mod rotation;

/// 【画布导航测试】【图片构造】按scale返回原始棋盘图片编辑器，故意保留不同色的预览以识别错误采样来源
fn image_editor(scale: f32) -> EditorState {
    let mut state = editor();
    let mut original = Pixmap::new(320, 240).unwrap();
    for (index, pixel) in original
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let (x, y) = (index % 320, index / 320);
        *pixel = if (x / 4 + y / 4) % 2 == 0 {
            [220, 40, 60, 255]
        } else {
            [30, 150, 190, 255]
        };
    }
    let file = crate::editor::file_canvas::FileCanvas::new(
        original,
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
    )
    .unwrap();
    state.selection.zone = Some(file.viewport);
    state.file_canvas = Some(file);
    state.placements[0].scale = scale;
    state.canvas[0] = Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32).unwrap();
    state.dimmed[0] = state.canvas[0].clone();
    state.annotations_layer[0] = state.canvas[0].clone();
    state
}

/// 【画布导航测试】【场景绘制】按state和dirty绘制当前真实帧，临时隐藏屏幕面板以检查画布像素，返回无值
fn draw(state: &mut EditorState, dirty: Option<Rect>) {
    state.toolbar.opacity = 0.0;
    state.action_toolbar.opacity = 0.0;
    state.settings_panel.visible = false;
    state.selection_size.panel = None;
    let frame = Frame {
        active_text_id: state
            .text_session
            .as_ref()
            .map(|session| session.presented().id),
        scan_badge: None,
        current_color: Color::BLACK,
        outline_color: Color::TRANSPARENT,
        annotations_hidden: false,
    };
    let mut previous = state.selection.zone;
    render_monitor(state, 0, &frame, dirty, &mut previous);
}

/// 【画布导航测试】【锚点缩放】真实滚轮保持图内锚点，原图、选区和历史均不进入视图事务
#[test]
fn wheel_anchor_preserves_source_and_history() {
    let mut state = image_editor(1.0);
    let mut dirty = 0;
    pointer(&mut state, 230.0, 170.0, &mut dirty);
    let original = state.file_canvas.as_ref().unwrap().original.data().to_vec();
    let zone = state.selection.zone;
    let before = state.canvas_point();
    handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut dirty);
    assert!((state.navigation.view.unwrap().zoom() - 1.12).abs() < 1e-12);
    let after = state.canvas_point();
    assert!((after.0 - before.0).abs() < 0.0001 && (after.1 - before.1).abs() < 0.0001);
    assert_eq!(state.selection.zone, zone);
    assert!(state.undo_stack.is_empty());
    assert_eq!(
        state.file_canvas.as_ref().unwrap().original.data(),
        original
    );
    assert_eq!(state.input.pointer.global, (230.0, 170.0));
}

/// 【画布导航测试】【直接平移】中键立即进入手势，无阈值等待，完整位移保留已完成标注与历史
#[test]
fn middle_pan_is_immediate_and_release_ends_it() {
    let mut state = image_editor(1.0);
    let mut dirty = 0;
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(1200), &mut dirty);
    let start = state.canvas_transform();
    handle_pointer_button(&mut state, MouseButton::Middle, true, &mut dirty);
    assert!(state.navigation.pan.is_some());
    pointer(&mut state, 321.0, 241.0, &mut dirty);
    let current = state.canvas_transform();
    assert!((current.translation.0 - start.translation.0 - 1.0).abs() < 0.0001);
    assert!((current.translation.1 - start.translation.1 - 1.0).abs() < 0.0001);
    assert_eq!(
        crate::app::input::compute_cursor(&state),
        crate::types::CursorIcon::Grabbing
    );
    handle_pointer_button(&mut state, MouseButton::Middle, false, &mut dirty);
    assert!(state.navigation.pan.is_none());
    assert!(state.undo_stack.is_empty());
}

/// 【画布导航测试】【修饰键复位】重复按下状态不构成第二次Ctrl，释放后再次按下恢复恒等视图
#[test]
fn two_control_press_edges_reset_without_touching_history() {
    let mut state = image_editor(1.0);
    let mut dirty = 0;
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(240), &mut dirty);
    handle_modifiers(&mut state, true, false, &mut dirty);
    handle_modifiers(&mut state, true, false, &mut dirty);
    assert!(state.navigation.view.unwrap().zoom() > 1.0);
    handle_modifiers(&mut state, false, false, &mut dirty);
    handle_modifiers(&mut state, true, false, &mut dirty);
    assert!(state.canvas_transform().is_identity());
    assert!(state.undo_stack.is_empty());
}

/// 【画布导航测试】【源图正文】缩放后从原始棋盘采样，重复分数比例条带绘制与整帧逐字节一致
#[test]
fn transformed_original_and_partial_redraws_match_at_fractional_scales() {
    for scale in [1.0, 1.5, 2.0] {
        let mut state = image_editor(scale);
        let mut dirty = 0;
        pointer(&mut state, 307.5, 217.25, &mut dirty);
        handle_axis_scroll(&mut state, ScrollEvent::wheel(720), &mut dirty);
        draw(&mut state, None);
        let whole = state.canvas[0].clone();
        let pixel = whole
            .pixel((307.5 * scale) as u32, (217.25 * scale) as u32)
            .unwrap();
        assert!(pixel.red() > 80 || pixel.blue() > 80);
        for left in [101.25, 209.5, 307.375, 421.625] {
            for _ in 0..3 {
                draw(&mut state, Rect::from_xywh(left, 100.25, 13.625, 279.375));
            }
        }
        let different = whole
            .data()
            .iter()
            .zip(state.canvas[0].data())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(different, 0, "scale={scale}");
    }
}

/// 【画布导航测试】【稳定笔迹】缩放后绘制完整笔迹，提交前后正文一致，历史及导出只使用稳定源坐标
#[test]
fn pen_draft_commit_and_export_keep_stable_geometry() {
    let mut state = image_editor(1.5);
    let mut dirty = 0;
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(720), &mut dirty);
    select_tool(&mut state, Tool::Pen, &mut dirty);
    state.tool_settings.color = Color::from_rgba8(240, 210, 10, 255);
    pointer(&mut state, 210.0, 160.0, &mut dirty);
    let start = state.canvas_point();
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    for (x, y) in [(225.0, 172.0), (255.0, 186.0), (280.0, 204.0)] {
        pointer(&mut state, x, y, &mut dirty);
    }
    draw(&mut state, None);
    let draft = state.canvas[0].clone();
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    draw(&mut state, None);
    assert_eq!(draft.data(), state.canvas[0].data());
    let AnnotationShape::Pen { points } = &state.annotations[0].shape else {
        panic!("pen expected")
    };
    assert!((f64::from(points[0].0) - start.0).abs() < 0.0001);
    let annotation = state.annotations[0].clone();
    let before = crate::app::image_export::render_precise(&mut state)
        .unwrap()
        .0;
    crate::app::canvas_navigation::input::ctrl_pressed(
        &mut state,
        std::time::Instant::now(),
        &mut dirty,
    );
    crate::app::canvas_navigation::input::ctrl_pressed(
        &mut state,
        std::time::Instant::now(),
        &mut dirty,
    );
    let after = crate::app::image_export::render_precise(&mut state)
        .unwrap()
        .0;
    assert_eq!(before, after);
    assert_eq!(state.annotations[0], annotation);
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【画布导航测试】【滚动条流程】边缘揭示、滑块拖动与离开隐藏只改变视图，箭头光标不进入下方画布
#[test]
fn scrollbar_drag_uses_full_window_ranges_and_hides_on_exit() {
    let mut state = image_editor(1.0);
    let mut dirty = 0;
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(720), &mut dirty);
    pointer(&mut state, 637.0, 210.0, &mut dirty);
    let axis = state.navigation.bars.vertical;
    assert!(axis.visible && axis.maximum > 0);
    assert_eq!(axis.page_step, 480);
    assert_eq!(
        crate::app::input::compute_cursor(&state),
        crate::types::CursorIcon::Default
    );
    let (_, thumb) = crate::ui::canvas_navigation::geometry::parts(axis, false, true).unwrap();
    pointer(&mut state, 637.0, thumb.y + thumb.height / 2.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert!(state.navigation.slider.is_some());
    pointer(
        &mut state,
        637.0,
        thumb.y + thumb.height / 2.0 + 20.0,
        &mut dirty,
    );
    assert!(state.navigation.bars.vertical.value > axis.value);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    pointer(&mut state, 300.0, 200.0, &mut dirty);
    assert!(!state.navigation.bars.vertical.visible);
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
}

/// 【画布导航测试】【拖动保护】左键绘制期间滚轮不改变视图，也不能修改当前笔宽
#[test]
fn drawing_blocks_navigation_and_width_changes() {
    let mut state = image_editor(1.0);
    let mut dirty = 0;
    select_tool(&mut state, Tool::Pen, &mut dirty);
    pointer(&mut state, 240.0, 180.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    let width = state
        .tool_settings
        .stroke(crate::types::StrokeGroup::Freehand);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut dirty);
    assert_eq!(state.navigation.view.unwrap().zoom(), 1.0);
    assert_eq!(
        state
            .tool_settings
            .stroke(crate::types::StrokeGroup::Freehand),
        width
    );
    assert!(state.pending.is_some());
}

/// 【画布导航测试】【控制点及删除】放大后普通方块命中保持屏幕容差，删除按钮通过真实输入删除稳定标注
#[test]
fn selected_controls_and_delete_button_follow_the_screen_view() {
    let mut state = image_editor(1.5);
    let mut dirty = 0;
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(1440), &mut dirty);
    let mut annotation = crate::types::Annotation {
        style_scale: 1.0,
        id: 42,
        shape: AnnotationShape::Rectangle {
            start: (300.0, 220.0),
            end: (340.0, 260.0),
            filled: false,
        },
        rotation_degrees: 0.0,
        frozen_bounds: state.selection.zone,
        color: Color::BLACK,
        shadow_color: Color::TRANSPARENT,
        stroke_width: 3.0,
        bbox: Rect::from_xywh(300.0, 220.0, 40.0, 40.0).unwrap(),
    };
    annotation.update_bbox();
    state.annotations.push(annotation.clone());
    select_tool(&mut state, Tool::Pick, &mut dirty);
    state.select_annotation_index(Some(0));
    let view = state.canvas_transform();
    let corner = view.to_screen((300.0, 220.0));
    let point = view.to_canvas((corner.0 + 25.0, corner.1 + 25.0));
    assert_eq!(
        state.canvas_handle(&annotation, point),
        crate::types::SelectionHandle::Move
    );
    assert_eq!(
        state.canvas_handle(
            &annotation,
            view.to_canvas((corner.0 + 4.0, corner.1 + 4.0))
        ),
        crate::types::SelectionHandle::TopLeft
    );
    crate::app::annotation_selection::sync(&mut state, &mut dirty);
    let (_, button) = state.annotation_selection.frame.unwrap();
    assert_eq!((button.width(), button.height()), (20.0, 20.0));
    pointer(
        &mut state,
        f64::from(button.x() + 10.0),
        f64::from(button.y() + 10.0),
        &mut dirty,
    );
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(state.annotations.is_empty());
    assert_eq!(state.undo_stack.len(), 1);
    assert_eq!(state.canvas_transform(), view);
}

/// 【画布导航测试】【多输出映射】负坐标工作区共享倍率和锚点，损坏按变换后的输出分派，不按标注原始位置分派
#[test]
fn multiple_outputs_share_navigation_and_map_canvas_damage() {
    let mut state = editor();
    state.placements[0].position = (-640, 0);
    state.placements.push(crate::types::Placement {
        position: (0, 0),
        size: (640, 480),
        scale: 1.5,
    });
    state.captures.push(crate::types::Capture {
        pixmap: Pixmap::new(960, 720).unwrap().into(),
        scale: 1.5,
    });
    state.canvas.push(Pixmap::new(960, 720).unwrap());
    state.dimmed.push(Pixmap::new(960, 720).unwrap());
    state.annotations_layer.push(Pixmap::new(960, 720).unwrap());
    state.selection.zone = Rect::from_xywh(-640.0, 0.0, 1280.0, 480.0);
    state.selected_tool = Tool::Pick;
    let mut dirty = 0;
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    let anchor = state.canvas_point();
    handle_axis_scroll(&mut state, ScrollEvent::wheel(720), &mut dirty);
    assert!((state.canvas_point().0 - anchor.0).abs() < 0.0001);
    let view = state.canvas_transform();
    crate::app::input::handle_pointer_move(&mut state, 1, 100.0, 240.0, &mut dirty);
    assert_eq!(state.canvas_point(), view.to_canvas((100.0, 240.0)));
    let center = view.to_canvas((100.0, 240.0));
    state.damage_rects.clear();
    state.layer_damage_rects.clear();
    state.damage_rects.push(crate::editor::DamageZone::Global(
        Rect::from_xywh(center.0 as f32 - 1.0, center.1 as f32 - 1.0, 2.0, 2.0).unwrap(),
    ));
    dirty = 0;
    crate::editor::dirty::apply_damage_rects(&mut state, &mut dirty);
    assert_eq!(dirty, 2);
    let local = state.monitor_dirty_rect(1).unwrap();
    assert!(local.left() < 100.0 && local.right() > 100.0);
}

/// 【画布导航测试】【文字提交】中键平移先提交未完成文字，焦点离开结束手势，视图保留且文字只记录一次历史
#[test]
fn middle_pan_commits_text_and_focus_loss_ends_the_gesture() {
    let mut state = image_editor(1.0);
    let mut dirty = 0;
    select_tool(&mut state, Tool::Text, &mut dirty);
    pointer(&mut state, 280.0, 210.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    crate::app::input::handle_key(&mut state, None, Some("导航文字".into()), &mut dirty);
    assert!(state.text_session.is_some());
    handle_pointer_button(&mut state, MouseButton::Middle, true, &mut dirty);
    assert!(state.text_session.is_none() && state.navigation.pan.is_some());
    assert_eq!(state.annotations.len(), 1);
    assert_eq!(state.undo_stack.len(), 1);
    crate::app::canvas_navigation::input::focus_lost(&mut state, &mut dirty);
    assert!(state.navigation.pan.is_none());
    assert_eq!(state.annotations.len(), 1);
}

/// 【画布导航测试】【单次期限】气泡到期时独立清理像素，九百毫秒之后不保留定时唤醒
#[test]
fn zoom_preview_expires_without_another_pointer_event() {
    let mut state = image_editor(1.0);
    state.ensure_navigation();
    state.input.pointer = crate::types::PointerState::new(0, (320.0, 240.0), (320.0, 240.0));
    let now = std::time::Instant::now();
    let mut dirty = 0;
    crate::app::canvas_navigation::preview::show(&mut state, now, &mut dirty);
    assert_eq!(state.navigation.preview.as_ref().unwrap().text, "100%");
    assert_eq!(
        state.navigation.preview.as_ref().unwrap().rect.height(),
        30.0
    );
    assert!(crate::app::event_loop::poll_timeout(&state).is_some());
    crate::app::canvas_navigation::preview::tick(
        &mut state,
        now + std::time::Duration::from_millis(899),
        &mut dirty,
    );
    assert!(state.navigation.preview.is_some());
    dirty = 0;
    crate::app::canvas_navigation::preview::tick(
        &mut state,
        now + std::time::Duration::from_millis(900),
        &mut dirty,
    );
    assert!(state.navigation.preview.is_none());
    assert_eq!(dirty, 1);
    assert!(crate::app::event_loop::poll_timeout(&state).is_none());
}

/// 【画布导航测试】【群组局部绘制】非恒等视图下共同拖动保留顺序，反复条带重绘与提交前后像素一致
#[test]
fn navigated_collection_redraw_and_commit_keep_the_same_pixels() {
    let mut state = super::annotation_collection::group();
    let ids = state.selected_annotation_ids().to_vec();
    state.select_annotation_ids([]);
    state.placements[0].scale = 1.5;
    state.canvas[0] = Pixmap::new(960, 720).unwrap();
    state.dimmed[0] = state.canvas[0].clone();
    state.annotations_layer[0] = state.canvas[0].clone();
    let mut dirty = 0;
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(240), &mut dirty);
    assert!(!state.canvas_transform().is_identity());
    state.select_annotation_ids(ids);
    state.begin_collection_drag(crate::types::SelectionHandle::Move);
    let start = state.canvas_point();
    state.update_collection_drag((start.0 + 10.25, start.1 - 5.75));
    crate::app::canvas_navigation::preview::clear(&mut state, &mut dirty);
    draw(&mut state, None);
    let whole = state.canvas[0].clone();
    for left in [105.375, 163.625, 259.25, 397.125] {
        for _ in 0..3 {
            draw(&mut state, Rect::from_xywh(left, 50.25, 17.625, 369.375));
        }
    }
    let different = whole
        .data()
        .iter()
        .zip(state.canvas[0].data())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(different, 0);
    state.finish_collection_drag();
    draw(&mut state, None);
    assert_eq!(whole.data(), state.canvas[0].data());
}

/// 【画布导航测试】【已有文字原件】缩放中重新编辑文字只绘制独立预览，持久原件保留在历史中但不重复合成
#[test]
fn editing_existing_text_does_not_draw_its_persistent_original() {
    let mut state = image_editor(1.5);
    state.tool_settings.font_size = 8.0;
    let mut dirty = 0;
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(240), &mut dirty);
    select_tool(&mut state, Tool::Text, &mut dirty);
    pointer(&mut state, 280.0, 210.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    crate::app::input::handle_key(&mut state, None, Some("WWWWWWW".into()), &mut dirty);
    crate::editor::text_session::commit(&mut state, &mut dirty);
    let original = state.annotations[0].clone();
    assert!(crate::editor::text_session::begin_existing(
        &mut state,
        original.id,
        &mut dirty
    ));
    super::key(&mut state, "ctrl+a", &mut dirty);
    crate::app::input::handle_key(&mut state, None, Some("i".into()), &mut dirty);
    assert_eq!(state.annotations[0], original);
    draw(&mut state, None);
    let actual = state.canvas[0].clone();
    state.annotations.clear();
    draw(&mut state, None);
    let different = actual
        .data()
        .iter()
        .zip(state.canvas[0].data())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        different, 0,
        "persistent text leaked behind its working preview"
    );
}
