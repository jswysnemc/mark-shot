//! 【形状测试】【输入链路】覆盖弹层、持久标注、属性历史与缓存画笔取消

use super::{editor, key, pointer, widget_center};
use crate::app::input::{
    compute_cursor, handle_modifiers, handle_pointer_button, handle_scroll, select_tool,
};
use crate::app::panels;
use crate::editor::EditorState;
use crate::tools::Tool;
use crate::types::{AnnotationShape, CursorIcon, MarkerShape, MouseButton, StrokeGroup};
use crate::ui::panel::AnimatedPanel;
use crate::ui::settings_panel::{SettingsAction, SettingsWidget, ToggleField};

/// 【形状测试】【工具构造】返回已选择标记工具的编辑器，并将 dirty 设为初始化重绘范围
fn marker_editor(dirty: &mut u32) -> EditorState {
    let mut state = editor();
    select_tool(&mut state, Tool::Marker, dirty);
    state.toolbar.opacity = 1.0;
    state.toolbar.render_pos = state.toolbar.position;
    panels::settings::update_settings_panel(&mut state, dirty);
    state
}

/// 【形状测试】【完成绘图】在 state 中绘制一个标记；dirty 返回重绘显示器，无返回值
fn draw(state: &mut EditorState, dirty: &mut u32) {
    pointer(state, 140.0, 110.0, dirty);
    handle_pointer_button(state, MouseButton::Left, true, dirty);
    pointer(state, 240.0, 210.0, dirty);
    handle_pointer_button(state, MouseButton::Left, false, dirty);
}

/// 【形状测试】【弹层打开】打开 state 的选择面板并完成透明度动画；dirty 返回重绘显示器
fn open(state: &mut EditorState, dirty: &mut u32) {
    panels::marker::toggle(state, dirty);
    state.marker_popover.animate_step(1.0);
    panels::marker::update(state, dirty);
}

/// 【形状测试】【键盘确认】Enter 只应用预选形状，不触发导出，也不把工具默认值写入撤销
#[test]
fn enter_confirms_a_shape_without_export_or_empty_history() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    open(&mut state, &mut dirty);
    key(&mut state, "End", &mut dirty);
    assert!(state.marker_popover.open);
    key(&mut state, "Enter", &mut dirty);
    assert_eq!(state.tool_settings.marker_shape, MarkerShape::SpeechBubble);
    assert!(!state.marker_popover.open);
    assert!(state.finish.is_none());
    assert!(state.undo_stack.is_empty());
    assert!(state.annotations.is_empty());
    assert_eq!(state.toolbar.marker_shape, MarkerShape::SpeechBubble);
}

/// 【形状测试】【全部选择】面板索引使用显示目录转换，所有形状都可经键盘选择，不误用旧编号
#[test]
fn every_grid_option_uses_its_display_order_not_legacy_id() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    for (index, expected) in MarkerShape::ALL.into_iter().enumerate() {
        open(&mut state, &mut dirty);
        key(&mut state, "Home", &mut dirty);
        for _ in 0..index {
            key(&mut state, "Right", &mut dirty);
        }
        key(&mut state, "Enter", &mut dirty);
        assert_eq!(state.tool_settings.marker_shape, expected);
    }
    assert_eq!(MarkerShape::ALL[6], MarkerShape::Spade);
    assert_eq!(MarkerShape::from_legacy_id(6), Some(MarkerShape::Hexagon));
    assert!(state.undo_stack.is_empty());
}

/// 【形状测试】【已有标记】形状修改只记录一次标注事务，撤销与重做保留形状、填充和选中项
#[test]
fn editing_an_existing_marker_records_one_undo_transaction() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    state.tool_settings.marker_shape = MarkerShape::Triangle;
    state.tool_settings.marker_filled = true;
    draw(&mut state, &mut dirty);
    assert_eq!(state.annotations.len(), 1);
    let original = state.annotations[0].clone();
    select_tool(&mut state, Tool::Pick, &mut dirty);
    state.select_annotation_index(Some(0));
    panels::settings::update_settings_panel(&mut state, &mut dirty);
    let history_len = state.undo_stack.len();
    open(&mut state, &mut dirty);
    key(&mut state, "End", &mut dirty);
    key(&mut state, "Enter", &mut dirty);
    let modified = state.annotations[0].clone();
    assert!(matches!(
        modified.shape,
        AnnotationShape::Marker {
            kind: MarkerShape::SpeechBubble,
            filled: true,
            ..
        }
    ));
    assert_eq!(state.undo_stack.len(), history_len + 1);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(state.annotations, [original]);
    assert_eq!(state.settings_panel.marker_shape, MarkerShape::Triangle);
    key(&mut state, "Ctrl+Shift+Z", &mut dirty);
    assert_eq!(state.annotations, [modified]);
    assert_eq!(state.selected_annotation_index(), Some(0));
    assert_eq!(state.settings_panel.marker_shape, MarkerShape::SpeechBubble);
}

/// 【形状测试】【独立属性】标记笔宽与填充不覆盖矩形、激光或画笔默认值
#[test]
fn marker_fill_and_width_do_not_change_other_tool_defaults() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    state.tool_settings.fill = true;
    state.tool_settings.marker_filled = true;
    state.tool_settings.set_stroke(StrokeGroup::Freehand, 7.0);
    state.tool_settings.set_stroke(StrokeGroup::Laser, 13.0);
    state.tool_settings.set_stroke(StrokeGroup::Rectangle, 2.0);
    panels::settings::apply_toggle_field(&mut state, ToggleField::MarkerFill, false, &mut dirty);
    state.tool_settings.set_stroke(StrokeGroup::Marker, 9.0);
    assert!(state.tool_settings.fill);
    assert!(!state.tool_settings.marker_filled);
    assert_eq!(state.tool_settings.stroke(StrokeGroup::Rectangle), 2.0);
    assert_eq!(state.tool_settings.stroke(StrokeGroup::Laser), 13.0);
    assert_eq!(state.tool_settings.stroke(StrokeGroup::Freehand), 7.0);
    draw(&mut state, &mut dirty);
    assert_eq!(state.annotations[0].stroke_width, 9.0);
    assert!(matches!(
        state.annotations[0].shape,
        AnnotationShape::Marker { filled: false, .. }
    ));
}

/// 【形状测试】【点击开关】再次点击形状按钮只关闭弹层，不经过属性动作重新打开
#[test]
fn clicking_the_open_toggle_closes_it_exactly_once() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    let index = state
        .settings_panel
        .widgets
        .iter()
        .position(|widget| {
            matches!(
                widget,
                SettingsWidget::Action {
                    action: SettingsAction::MarkerShapes,
                    ..
                }
            )
        })
        .unwrap();
    let position = widget_center(&state, index);
    pointer(&mut state, position.0, position.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(state.marker_popover.open);
    state.marker_popover.animate_step(1.0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert!(!state.marker_popover.open);
    assert!(state.finish.is_none());
}

/// 【形状测试】【鼠标选择】单元中心显示指针光标并选择正确形状；点击事件不创建额外标注
#[test]
fn pointer_selection_uses_the_same_grid_index_as_keyboard() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    open(&mut state, &mut dirty);
    let panel = &state.marker_popover;
    let cell = panel.layout.item_rect(6, panel.first_row).unwrap();
    let position = (
        panel.render_pos.0 + cell.left() + cell.width() / 2.0,
        panel.render_pos.1 + cell.top() + cell.height() / 2.0,
    );
    pointer(&mut state, position.0 as f64, position.1 as f64, &mut dirty);
    assert_eq!(compute_cursor(&state), CursorIcon::Pointer);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(state.tool_settings.marker_shape, MarkerShape::Spade);
    assert!(state.annotations.is_empty());
}

/// 【形状测试】【滚轮隔离】打开网格时空白处滚轮不修改笔宽，Escape 关闭弹层而不退出编辑器
#[test]
fn an_open_grid_consumes_scroll_and_escape_before_global_actions() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    let width = state.tool_settings.stroke(StrokeGroup::Marker);
    open(&mut state, &mut dirty);
    pointer(&mut state, 1.0, 1.0, &mut dirty);
    handle_scroll(&mut state, 0.0, 9.0, &mut dirty);
    assert_eq!(state.tool_settings.stroke(StrokeGroup::Marker), width);
    key(&mut state, "Escape", &mut dirty);
    assert!(!state.marker_popover.open);
    assert!(!state.cancel);
    key(&mut state, "Escape", &mut dirty);
    assert!(state.cancel);
}

/// 【形状测试】【滚轮方向】短屏网格将 Wayland 向上滚动映射到较早条目，向下滚动映射到较晚条目
#[test]
fn short_grid_scrolling_follows_the_pointer_axis_direction() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    state.placements[0].size = (180, 190);
    open(&mut state, &mut dirty);
    state.marker_popover.first_row = 1;
    let panel = &state.marker_popover;
    let cell = panel.layout.item_rect(panel.layout.columns, 1).unwrap();
    state.input.pointer.local = (
        (panel.render_pos.0 + cell.left() + cell.width() / 2.0) as f64,
        (panel.render_pos.1 + cell.top() + cell.height() / 2.0) as f64,
    );
    let delta = crate::config::get().input.scroll_sensitivity.recip();
    handle_scroll(&mut state, 0.0, delta, &mut dirty);
    assert_eq!(state.marker_popover.first_row, 0);
    state.marker_popover.cancel_scroll();
    handle_scroll(&mut state, 0.0, -delta, &mut dirty);
    assert_eq!(state.marker_popover.first_row, 1);
}

/// 【形状测试】【缓存取消】工具切换重建草稿覆盖的图层，已有标记和导出像素保持不变
#[test]
fn switching_from_a_baked_pen_draft_preserves_committed_markers() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    draw(&mut state, &mut dirty);
    let committed = state.annotations.clone();
    let layer = state.annotations_layer[0].data().to_vec();
    let export = crate::app::render_final(&mut state).unwrap().0;
    let history_len = state.undo_stack.len();
    select_tool(&mut state, Tool::Pen, &mut dirty);
    pointer(&mut state, 145.0, 130.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    for index in 1..10 {
        pointer(
            &mut state,
            145.0 + f64::from(index) * 12.0,
            130.0 + f64::from(index) * 5.0,
            &mut dirty,
        );
    }
    assert!(state.pending_pen_baked > 0);
    assert!(
        state.annotations_layer[0].data() != layer,
        "the pen draft did not enter its drawing cache"
    );
    key(&mut state, "K", &mut dirty);
    assert_eq!(state.selected_tool, Tool::Marker);
    assert!(state.pending.is_none());
    assert_eq!(state.pending_pen_baked, 0);
    assert!(!state.input.mouse_down);
    assert!(!state.tool_active);
    let frame = crate::app::frame_render::Frame {
        active_text_id: None,
        scan_badge: None,
        current_color: state.tool_settings.color,
        outline_color: state.tool_settings.outline_color,
        annotations_hidden: false,
    };
    crate::app::frame_render::rebuild_layer(&mut state, 0, &frame);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    let after = state.annotations_layer[0].data();
    let mut differences = layer
        .as_chunks::<4>()
        .0
        .iter()
        .zip(after.as_chunks::<4>().0.iter())
        .enumerate()
        .filter(|(_, (before, after))| before != after)
        .map(|(index, (before, after))| (index % 640, index / 640, before, after));
    let first = differences.next();
    let count = usize::from(first.is_some()) + differences.count();
    assert!(
        count == 0,
        "{} pixels differ after draft cancellation, first {:?}",
        count,
        first
    );
    assert_eq!(state.annotations, committed);
    assert_eq!(state.undo_stack.len(), history_len);
    assert!(
        crate::app::render_final(&mut state).unwrap().0 == export,
        "canceling a draft changed the exported image"
    );
}

/// 【形状测试】【隐藏界面】隐藏工具界面会关闭标记弹层，恢复界面不重新打开
#[test]
fn hiding_ui_closes_the_marker_popover() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    open(&mut state, &mut dirty);
    crate::app::actions::run(&mut state, crate::keys::Action::ToggleUi, &mut dirty);
    assert!(state.ui_hidden);
    assert!(!state.marker_popover.open);
    crate::app::actions::run(&mut state, crate::keys::Action::ToggleUi, &mut dirty);
    assert!(!state.ui_hidden);
    assert!(!state.marker_popover.open);
}

/// 【形状测试】【动画轮询】弹层只在渐入或渐出时唤醒事件循环，稳定显示与关闭后均可阻塞等待
#[test]
fn marker_animation_stops_polling_at_both_opacity_targets() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    assert!(crate::app::event_loop::poll_timeout(&state).is_none());
    panels::marker::toggle(&mut state, &mut dirty);
    assert!(crate::app::event_loop::poll_timeout(&state).is_some());
    state.marker_popover.animate_step(1.0);
    assert!(crate::app::event_loop::poll_timeout(&state).is_none());
    panels::marker::close(&mut state, &mut dirty);
    assert!(crate::app::event_loop::poll_timeout(&state).is_some());
    state.marker_popover.animate_step(1.0);
    assert!(crate::app::event_loop::poll_timeout(&state).is_none());
}

/// 【形状测试】【微小草稿】单击与小于两个源图像素的矩形不提交，标识和撤销记录保持不变
#[test]
fn single_clicks_and_subpixel_markers_do_not_create_annotations() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    for end in [(140.0, 140.0), (141.0, 148.0), (150.0, 140.0)] {
        pointer(&mut state, 140.0, 140.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, end.0, end.1, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert!(state.pending.is_none());
        assert!(state.prev_pending.is_none());
        assert!(state.annotations.is_empty());
        assert!(state.undo_stack.is_empty());
        assert_eq!(state.next_id, 0);
        assert!(!state.tool_active);
    }
}

/// 【形状测试】【等比例手势】Ctrl 按下立即变为正方形，释放恢复自由比例，提交使用当前约束
#[test]
fn ctrl_updates_the_marker_aspect_ratio_without_extra_pointer_motion() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    pointer(&mut state, 140.0, 120.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 200.0, 150.0, &mut dirty);
    handle_modifiers(&mut state, true, false, &mut dirty);
    assert!(matches!(
        state.pending.as_ref().unwrap().shape,
        AnnotationShape::Marker {
            end: (200.0, 180.0),
            ..
        }
    ));
    handle_modifiers(&mut state, false, false, &mut dirty);
    assert!(matches!(
        state.pending.as_ref().unwrap().shape,
        AnnotationShape::Marker {
            end: (200.0, 150.0),
            ..
        }
    ));
    handle_modifiers(&mut state, true, false, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(matches!(
        state.annotations[0].shape,
        AnnotationShape::Marker {
            end: (200.0, 180.0),
            ..
        }
    ));
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【形状测试】【源图坐标】源图外不能开始标记，结束点限定到源图；最小尺寸使用原始像素比例
#[test]
fn image_markers_require_an_inside_start_and_use_native_pixel_sizes() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    let file = crate::editor::file_canvas::FileCanvas::new(
        tiny_skia::Pixmap::new(1600, 900).unwrap(),
        tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
    )
    .unwrap();
    let viewport = file.viewport;
    state.file_canvas = Some(file);
    state.selection.zone = Some(viewport);
    pointer(
        &mut state,
        f64::from(viewport.width()) / 2.0 + f64::from(viewport.left()),
        f64::from(viewport.top()) - 12.0,
        &mut dirty,
    );
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert!(state.pending.is_none());
    assert!(!state.input.mouse_down);
    pointer(&mut state, 140.0, 160.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 141.0, 161.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(state.annotations.len(), 1);
    pointer(&mut state, 150.0, 180.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 1000.0, 1000.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    let AnnotationShape::Marker { end, .. } = state.annotations[1].shape else {
        panic!("expected marker")
    };
    assert_eq!(end, (viewport.right(), viewport.bottom()));
}

/// 【形状测试】【未缓存取消】取消标记草稿只清除屏幕动态层，不重画或改变已有持久标记
#[test]
fn canceling_an_unbaked_marker_does_not_dirty_the_persistent_layer() {
    let mut dirty = 0;
    let mut state = marker_editor(&mut dirty);
    draw(&mut state, &mut dirty);
    let committed = state.annotations.clone();
    let layer = state.annotations_layer[0].data().to_vec();
    pointer(&mut state, 160.0, 130.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 250.0, 190.0, &mut dirty);
    key(&mut state, "P", &mut dirty);
    assert!(state.pending.is_none());
    assert!(!state.annotations_dirty);
    assert!(state.layer_damage_rects.is_empty());
    assert!(state.annotations_layer[0].data() == layer);
    assert_eq!(state.annotations, committed);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert_eq!(state.undo_stack.len(), 1);
}
