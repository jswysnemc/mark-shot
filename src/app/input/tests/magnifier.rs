//! 【放大镜测试】【真实输入分派】验证创建、选择、属性、取消与原图导出，不连接显示服务器

use tiny_skia::{Color, Pixmap, Rect};

use super::{editor, key, pointer, widget_center};
use crate::app::input::{handle_modifiers, handle_pointer_button, select_tool};
use crate::app::panels;
use crate::editor::EditorState;
use crate::editor::file_canvas::FileCanvas;
use crate::features::magnifier::{Magnifier, MagnifierShape, drag};
use crate::tools::Tool;
use crate::types::{AnnotationShape, MouseButton, SelectionHandle, StrokeGroup};
use crate::ui::settings_panel::{SettingsWidget, StepperRange, ToggleField};

/// 【放大镜测试】【编辑器构造】返回 shape 指定的放大镜会话；dirty 返回初始化重绘显示器
fn magnifier_editor(shape: MagnifierShape, dirty: &mut u32) -> EditorState {
    let mut state = editor();
    state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.tool_settings.magnifier_shape = shape;
    state.tool_settings.magnifier_zoom = 2.75;
    state.tool_settings.set_stroke(StrokeGroup::Magnifier, 3.0);
    select_tool(&mut state, Tool::Magnifier, dirty);
    settle(&mut state, dirty);
    state
}

/// 【放大镜测试】【面板稳定】把 state 的属性和工具栏调整到动画终态；dirty 返回重绘显示器，无返回值
fn settle(state: &mut EditorState, dirty: &mut u32) {
    panels::settings::update_settings_panel(state, dirty);
    state.toolbar.opacity = if state.toolbar.interferes { 0.0 } else { 1.0 };
    state.toolbar.render_pos = state.toolbar.position;
    state.settings_panel.opacity = if state.settings_panel.visible {
        1.0
    } else {
        0.0
    };
    state.settings_panel.render_pos = state.settings_panel.position;
}

/// 【放大镜测试】【完整手势】在 state 中从 start 拖动到 end；dirty 返回重绘显示器，无返回值
fn gesture(state: &mut EditorState, start: (f64, f64), end: (f64, f64), dirty: &mut u32) {
    pointer(state, start.0, start.1, dirty);
    handle_pointer_button(state, MouseButton::Left, true, dirty);
    pointer(state, end.0, end.1, dirty);
    handle_pointer_button(state, MouseButton::Left, false, dirty);
}

/// 【放大镜测试】【持久数据】返回 state 首个放大镜，类型或索引错误时使测试失败
fn value(state: &EditorState) -> Magnifier {
    let AnnotationShape::Magnifier { magnifier } = state.annotations[0].shape else {
        panic!("the magnifier gesture committed a different type")
    };
    magnifier
}

/// 【放大镜测试】【控件索引】用 predicate 在 state 属性面板中查找控件；返回唯一匹配索引，缺失时使测试失败
fn widget(state: &EditorState, predicate: impl Fn(&SettingsWidget) -> bool) -> usize {
    state
        .settings_panel
        .widgets
        .iter()
        .position(predicate)
        .unwrap()
}

/// 【放大镜测试】【选择操作】在 state 中单击 point 并等待面板稳定；dirty 返回重绘显示器，无返回值
fn select(state: &mut EditorState, point: (f64, f64), dirty: &mut u32) {
    gesture(state, point, point, dirty);
    settle(state, dirty);
}

/// 【放大镜测试】【两位精度】倍率字段显示原始 2.75 倍，不把步进值四舍五入到一位小数
#[test]
fn the_zoom_field_preserves_two_decimal_places() {
    let mut dirty = 0;
    let state = magnifier_editor(MagnifierShape::Circle, &mut dirty);
    let index = widget(&state, |widget| {
        matches!(
            widget,
            SettingsWidget::Stepper {
                range: StepperRange::Magnification,
                ..
            }
        )
    });
    assert_eq!(
        state.settings_panel.fields.value(index).map(String::as_str),
        Some("2.75")
    );
}

/// 【放大镜测试】【创建历史】空手势不提交，有效双框只有一次历史，撤销与重做保留倍率及独立笔宽分组
#[test]
fn creation_skips_empty_gestures_and_has_a_single_history_entry() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Circle, &mut dirty);
    let pen_width = state.tool_settings.stroke(StrokeGroup::Freehand);
    gesture(&mut state, (140.0, 180.0), (140.0, 180.0), &mut dirty);
    gesture(&mut state, (140.0, 180.0), (170.0, 190.0), &mut dirty);
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    assert_eq!(state.annotations.len(), 1);
    assert_eq!(state.undo_stack.len(), 1);
    let original = value(&state);
    assert_eq!(original.zoom, 2.75);
    assert_eq!(state.annotations[0].stroke_width, 3.0);
    assert_eq!(state.tool_settings.stroke(StrokeGroup::Freehand), pen_width);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert!(state.annotations.is_empty());
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(value(&state), original);
}

/// 【放大镜测试】【草稿取消】Escape 和切换工具只取消未完成双框，保留已有标注与导出像素
#[test]
fn canceling_or_switching_a_draft_preserves_committed_annotations() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Rectangle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    let original = state.annotations.clone();
    let output = crate::app::render_final(&mut state).unwrap().0;
    for cancel in ["Escape", "P"] {
        select_tool(&mut state, Tool::Magnifier, &mut dirty);
        pointer(&mut state, 520.0, 150.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 520.0, 400.0, &mut dirty);
        assert!(state.pending.is_some());
        key(&mut state, cancel, &mut dirty);
        assert!(state.pending.is_none(), "{cancel} did not cancel the draft");
        assert!(!state.input.mouse_down && !state.tool_active && !state.cancel);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert_eq!(state.annotations, original);
        assert_eq!(state.undo_stack.len(), 1);
        assert_eq!(crate::app::render_final(&mut state).unwrap().0, output);
    }
}

/// 【放大镜测试】【源框拖动】放大镜工具可直接选择源框并独立移动，静止单击不增加历史
#[test]
fn the_magnifier_tool_moves_the_source_frame_independently() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Rectangle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    let before = value(&state);
    gesture(&mut state, (140.0, 180.0), (170.0, 200.0), &mut dirty);
    let after = value(&state);
    assert_eq!(after.center, before.center);
    assert_eq!(after.size, before.size);
    assert_ne!(after.source, before.source);
    assert_eq!(state.undo_stack.len(), 2);
    select(&mut state, (170.0, 200.0), &mut dirty);
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(value(&state), before);
}

/// 【放大镜测试】【选择源框】选择工具首次点击源框时不移动镜框，也不使用尚未显示的缩放控制点
#[test]
fn the_pick_tool_keeps_source_and_lens_movement_separate() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Circle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    let before = value(&state);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (170.0, 200.0), &mut dirty);
    assert_eq!(value(&state).center, before.center);
    assert_eq!(value(&state).size, before.size);
    assert_ne!(value(&state).source, before.source);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(value(&state), before);
    gesture(&mut state, (350.0, 280.0), (380.0, 300.0), &mut dirty);
    assert_eq!(value(&state).source, before.source);
    assert_ne!(value(&state).center, before.center);
}

/// 【放大镜测试】【选择框旋转】选择工具优先命中正文之外的旋转把手，一次拖动仅产生一次历史
#[test]
fn the_pick_tool_hits_the_rotation_handle_outside_the_frame() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Rectangle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    let before = value(&state);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    select(&mut state, (350.0, 280.0), &mut dirty);
    let handle = drag::rotation_handle(before).unwrap();
    gesture(
        &mut state,
        (handle.0 as f64, handle.1 as f64),
        ((before.pivot.0 + 120.0) as f64, before.pivot.1 as f64),
        &mut dirty,
    );
    assert_ne!(value(&state).rotation, 0.0);
    assert_eq!(state.annotations.len(), 1);
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(value(&state), before);
}

/// 【放大镜测试】【原地吸附】保持指针不动时按下或释放 Shift 应更新正在旋转的双框，不依赖下一次移动事件
#[test]
fn shift_changes_an_active_rotation_without_another_pointer_move() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Rectangle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    select(&mut state, (350.0, 280.0), &mut dirty);
    let before = value(&state);
    let handle = drag::rotation_handle(before).unwrap();
    pointer(&mut state, handle.0 as f64, handle.1 as f64, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    let vector = (handle.0 - before.pivot.0, handle.1 - before.pivot.1);
    let (sin, cos) = 34.0_f32.to_radians().sin_cos();
    pointer(
        &mut state,
        (before.pivot.0 + vector.0 * cos - vector.1 * sin) as f64,
        (before.pivot.1 + vector.0 * sin + vector.1 * cos) as f64,
        &mut dirty,
    );
    handle_modifiers(&mut state, false, true, &mut dirty);
    let AnnotationShape::Magnifier { magnifier } = state.pending.as_ref().unwrap().shape else {
        unreachable!()
    };
    assert!((magnifier.rotation - std::f32::consts::FRAC_PI_4).abs() < 0.001);
    handle_modifiers(&mut state, false, false, &mut dirty);
    let AnnotationShape::Magnifier { magnifier } = state.pending.as_ref().unwrap().shape else {
        unreachable!()
    };
    assert!((magnifier.rotation - 34.0_f32.to_radians()).abs() < 0.001);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(state.undo_stack.len(), 2);
}

/// 【放大镜测试】【联动控制点】源框缩放改变镜框尺寸但保留镜框中心，完成后倍率与单次撤销仍正确
#[test]
fn source_handles_resize_the_lens_in_a_single_transaction() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Rectangle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    select(&mut state, (140.0, 180.0), &mut dirty);
    let before = value(&state);
    let (handle, point) = drag::controls(before)
        .into_iter()
        .find(|(handle, _)| *handle == SelectionHandle::MagnifierSourceBottomRight)
        .unwrap();
    assert_eq!(drag::hit_test(before, point, true), handle);
    gesture(
        &mut state,
        (point.0 as f64, point.1 as f64),
        (point.0 as f64 + 15.0, point.1 as f64 + 20.0),
        &mut dirty,
    );
    let after = value(&state);
    assert_eq!(after.center, before.center);
    assert_ne!(after.size, before.size);
    assert!((after.size.0 / after.source_rect().unwrap().width() - before.zoom).abs() < 0.001);
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(value(&state), before);
}

/// 【放大镜测试】【字段编辑】倍率通过真实键盘编辑确认后仅产生一次历史，Enter 不触发截图导出，笔宽不变
#[test]
fn zoom_field_editing_commits_once_and_enter_does_not_export() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Rectangle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    select(&mut state, (350.0, 280.0), &mut dirty);
    let index = widget(&state, |widget| {
        matches!(
            widget,
            SettingsWidget::Stepper {
                range: StepperRange::Magnification,
                ..
            }
        )
    });
    let point = widget_center(&state, index);
    select(&mut state, (point.0 - 12.0, point.1), &mut dirty);
    assert!(state.settings_panel.is_editing());
    handle_modifiers(&mut state, true, false, &mut dirty);
    key(&mut state, "Ctrl+A", &mut dirty);
    handle_modifiers(&mut state, false, false, &mut dirty);
    for ch in "3.25".chars() {
        crate::app::input::handle_key(&mut state, None, Some(ch.to_string()), &mut dirty);
    }
    key(&mut state, "Return", &mut dirty);
    assert_eq!(value(&state).zoom, 3.25);
    assert_eq!(state.annotations[0].stroke_width, 3.0);
    assert_eq!(state.undo_stack.len(), 2);
    assert!(state.finish.is_none() && !state.cancel);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(value(&state).zoom, 2.75);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(value(&state).zoom, 3.25);
}

/// 【放大镜测试】【框形属性】统一开关修改已选双框框形，撤销后返回原样且不修改普通矩形填充设置
#[test]
fn the_shape_toggle_updates_the_selected_annotation_and_undo() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Circle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    select(&mut state, (350.0, 280.0), &mut dirty);
    let fill = state.tool_settings.fill;
    let index = widget(&state, |widget| {
        matches!(
            widget,
            SettingsWidget::Toggle {
                field: ToggleField::MagnifierRectangle,
                ..
            }
        )
    });
    let point = widget_center(&state, index);
    select(&mut state, point, &mut dirty);
    assert_eq!(value(&state).shape, MagnifierShape::Rectangle);
    assert_eq!(state.undo_stack.len(), 2);
    assert_eq!(state.tool_settings.fill, fill);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(value(&state).shape, MagnifierShape::Circle);
}

/// 【放大镜测试】【按住撤销】已有框拖动期间撤销会恢复原图标注并结束输入，后续移动与松开不重新提交
#[test]
fn undoing_a_held_drag_ends_the_input_gesture() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Circle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    let before = value(&state);
    pointer(&mut state, 350.0, 280.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 380.0, 300.0, &mut dirty);
    assert!(state.pending.is_some() && state.ann_drag.is_some());
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(value(&state), before);
    assert!(!state.input.mouse_down && !state.tool_active);
    pointer(&mut state, 420.0, 340.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(value(&state), before);
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【放大镜测试】【拖动取消】Escape 取消已有镜框拖动，不关闭会话、不增加撤销记录
#[test]
fn escape_aborts_an_existing_frame_drag_without_closing_the_editor() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Rectangle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    let before = value(&state);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    pointer(&mut state, 350.0, 280.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 390.0, 320.0, &mut dirty);
    key(&mut state, "Escape", &mut dirty);
    assert_eq!(value(&state), before);
    assert!(state.pending.is_none() && state.ann_drag.is_none() && !state.cancel);
    assert!(!state.input.mouse_down && !state.tool_active);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(value(&state), before);
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【放大镜测试】【键盘删除】放大镜工具选中已有双框后可以删除，撤销恢复完整双框
#[test]
fn the_magnifier_tool_can_delete_and_restore_its_selected_annotation() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Circle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    let before = value(&state);
    select(&mut state, (350.0, 280.0), &mut dirty);
    key(&mut state, "Delete", &mut dirty);
    assert!(state.annotations.is_empty());
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(value(&state), before);
}

/// 【放大镜测试】【原始像素导出】本地图像镜框使用原图绿色区域而不是灰色屏幕预览，保留透明度和原始分辨率
#[test]
fn native_image_export_samples_original_pixels_not_the_preview_capture() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Rectangle, &mut dirty);
    let mut pixels = Pixmap::new(1200, 800).unwrap();
    pixels.fill(Color::from_rgba8(0, 0, 200, 128));
    for y in 0..500usize {
        for x in 0..400usize {
            let index = (y * 1200 + x) * 4;
            pixels.data_mut()[index..index + 4].copy_from_slice(&[0, 100, 0, 128]);
        }
    }
    let file = FileCanvas::new(pixels, Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap()).unwrap();
    let viewport = file.viewport;
    let scale = file.original.width() as f32 / viewport.width();
    state.selection.zone = Some(viewport);
    state.file_canvas = Some(file);
    gesture(
        &mut state,
        (viewport.left() as f64 - 2.0, 170.0),
        (200.0, 250.0),
        &mut dirty,
    );
    assert!(state.annotations.is_empty());
    gesture(&mut state, (100.0, 170.0), (380.0, 320.0), &mut dirty);
    assert_eq!(value(&state).pixel_scale, scale);
    let bytes = crate::app::render_final(&mut state).unwrap().0;
    let output = image::load_from_memory(&bytes).unwrap().into_rgba8();
    assert_eq!(output.dimensions(), (1200, 800));
    assert_eq!(output.get_pixel(1199, 799).0[3], 128);
    let x = ((380.0 - viewport.left()) * scale).round() as u32;
    let y = ((320.0 - viewport.top()) * scale).round() as u32;
    let pixel = output.get_pixel(x, y).0;
    assert!(
        pixel[1] > pixel[2],
        "the lens did not read the frozen green source: {pixel:?}"
    );
    assert_eq!(pixel[3], 192);
}

/// 【放大镜测试】【空闲阻塞】完成创建和选择后没有定时任务，光标取色放大镜的开关不改变持久双框
#[test]
fn idle_magnifiers_do_not_poll_or_depend_on_the_cursor_loupe() {
    let mut dirty = 0;
    let mut state = magnifier_editor(MagnifierShape::Circle, &mut dirty);
    gesture(&mut state, (140.0, 180.0), (350.0, 280.0), &mut dirty);
    select(&mut state, (350.0, 280.0), &mut dirty);
    let before = value(&state);
    assert_eq!(crate::app::event_loop::poll_timeout(&state), None);
    crate::app::actions::run(&mut state, crate::keys::Action::ToggleMagnifier, &mut dirty);
    assert_eq!(value(&state), before);
    assert_eq!(crate::app::event_loop::poll_timeout(&state), None);
}
