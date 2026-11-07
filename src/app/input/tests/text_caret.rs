//! 【文字光标测试】【生产调度】通过真实输入和帧绘制验证焦点、周期、菜单与最小动态损坏

use super::*;
use std::time::{Duration, Instant};

use crate::{
    app::{
        event_loop,
        frame_render::{self, Pacing},
    },
    types::OverlayEvent,
};

/// 【文字光标测试】【会话准备】返回已开始正文编辑且实际获得主表面焦点的 state；now 固定计时起点，不连接显示服务器
fn prepared(now: Instant) -> EditorState {
    let mut state = editor();
    state.selected_tool = crate::tools::Tool::Text;
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.selection.prev_zone = state.selection.zone;
    crate::editor::text_session::begin_new(&mut state, (250.0, 200.0), 1.0, &mut 0);
    super::super::handle_key(&mut state, None, Some("alpha  ".into()), &mut 0);
    event_loop::handle_event(&mut state, OverlayEvent::Focus { monitor_idx: 0 }, &mut 0);
    crate::app::text_caret::tick(&mut state, now, &mut 0);
    state.annotations_dirty = false;
    state.damage_rects.clear();
    state.layer_damage_rects.clear();
    state
}

/// 【文字光标测试】【动态帧】将 state 的 dirty 渲染到画布并清理帧状态，返回完整像素；不提交真实 Wayland 缓冲
fn frame(state: &mut EditorState, dirty: u32) -> Vec<u8> {
    let info = frame_render::Frame {
        active_text_id: Some(crate::editor::text_session::PREVIEW_ID),
        scan_badge: None,
        current_color: state.tool_settings.color,
        outline_color: state.tool_settings.outline_color,
        annotations_hidden: false,
    };
    let mut pacing = Pacing::new(1);
    let area = state.monitor_dirty_rect(0);
    frame_render::render_monitor(state, 0, &info, area, &mut pacing.dimmed_for[0]);
    assert_ne!(dirty, 0);
    frame_render::clear_frame_state(state, &mut 0, &pacing);
    state.canvas[0].data().to_vec()
}

/// 【文字光标测试】【局部更新】半周期仅损坏光标附近，局部帧与整帧逐字节一致，持久图层不失效，无参数、无返回值
#[test]
fn blink_damage_is_local_and_matches_a_full_redraw() {
    let now = Instant::now();
    let mut state = prepared(now);
    let shown = frame(&mut state, 1);
    let mut dirty = 0;
    crate::app::text_caret::tick(&mut state, now + Duration::from_millis(500), &mut dirty);
    let area = state.monitor_dirty_rect(0).unwrap();
    assert!(area.width() < 20.0 && area.height() < 60.0, "{area:?}");
    assert!(state.layer_damage_rects.is_empty());
    assert!(!state.annotations_dirty);
    let hidden = frame(&mut state, dirty);
    assert!(hidden != shown, "the caret must change the frame");
    let complete = frame(&mut state, 1);
    let differences = hidden
        .iter()
        .zip(&complete)
        .filter(|(left, right)| left != right)
        .count();
    assert_eq!(
        differences, 0,
        "local frame channels differ from full rendering"
    );
    crate::app::text_caret::tick(&mut state, now + Duration::from_millis(1000), &mut dirty);
    let restored = frame(&mut state, dirty);
    let differences = restored
        .iter()
        .zip(&shown)
        .filter(|(left, right)| left != right)
        .count();
    assert_eq!(differences, 0, "visible phase did not restore exact pixels");
}

/// 【文字光标测试】【焦点暂停】真实失去主表面焦点隐藏光标并停止定时，重新获得焦点建立新周期，无参数、无返回值
#[test]
fn keyboard_focus_loss_stops_the_caret_without_committing_text() {
    let now = Instant::now();
    let mut state = prepared(now);
    event_loop::handle_event(
        &mut state,
        OverlayEvent::FocusLost { monitor_idx: 0 },
        &mut 0,
    );
    crate::app::text_caret::tick(&mut state, now, &mut 0);
    assert!(!crate::app::text_caret::visible(&state));
    assert_eq!(event_loop::poll_timeout(&state), None);
    assert!(state.text_session.is_some());
    assert!(state.annotations.is_empty());
    event_loop::handle_event(&mut state, OverlayEvent::Focus { monitor_idx: 0 }, &mut 0);
    crate::app::text_caret::tick(&mut state, now + Duration::from_secs(4), &mut 0);
    assert!(crate::app::text_caret::visible(&state));
    assert_eq!(
        state.text_session.as_ref().unwrap().caret.deadline(),
        Some(now + Duration::from_millis(4500))
    );
}

/// 【文字光标测试】【选区保留】选中全部字符仍有光标周期，失活不清除正文与选区，无参数、无返回值
#[test]
fn selection_keeps_blinking_and_survives_focus_changes() {
    let now = Instant::now();
    let mut state = prepared(now);
    key(&mut state, "Ctrl+A", &mut 0);
    assert!(crate::tools::text::session_input::has_selection(&state));
    crate::app::text_caret::tick(&mut state, now + Duration::from_millis(500), &mut 0);
    assert!(!crate::app::text_caret::visible(&state));
    assert!(
        state
            .text_session
            .as_ref()
            .unwrap()
            .caret
            .deadline()
            .is_some()
    );
    event_loop::handle_event(
        &mut state,
        OverlayEvent::FocusLost { monitor_idx: 0 },
        &mut 0,
    );
    crate::app::text_caret::tick(&mut state, now + Duration::from_millis(700), &mut 0);
    assert!(crate::tools::text::session_input::has_selection(&state));
    assert_eq!(state.text_session.as_ref().unwrap().caret.deadline(), None);
}

/// 【文字光标测试】【菜单与字段】菜单和字段独占输入时隐藏光标并停止唤醒，恢复编辑后光标重新显示，无参数、无返回值
#[test]
fn menus_and_fields_suspend_only_the_annotation_caret() {
    let now = Instant::now();
    let mut state = prepared(now);
    state.font_panel.open = true;
    crate::app::text_caret::tick(&mut state, now, &mut 0);
    assert!(!crate::app::text_caret::visible(&state));
    assert_eq!(event_loop::poll_timeout(&state), None);
    state.font_panel.open = false;
    state.text_context_menu = Some(crate::ui::context_menu::ContextMenu::new(
        Vec::new(),
        &mut state.text.font_system,
        (250.0, 210.0),
        (640.0, 480.0),
        0,
    ));
    crate::app::text_caret::tick(&mut state, now + Duration::from_secs(1), &mut 0);
    assert_eq!(state.text_session.as_ref().unwrap().caret.deadline(), None);
    crate::app::text_context_menu::close(&mut state, &mut 0);
    crate::app::text_caret::tick(&mut state, now + Duration::from_secs(2), &mut 0);
    assert!(crate::app::text_caret::visible(&state));
    crate::editor::text_session::commit(&mut state, &mut 0);
    assert_eq!(event_loop::poll_timeout(&state), None);
}

/// 【文字光标测试】【延后失活】其他显示器的旧失活事件不清除新焦点，无参数、无返回值
#[test]
fn stale_leave_does_not_clear_a_new_monitor_focus() {
    let mut state = editor();
    event_loop::handle_event(&mut state, OverlayEvent::Focus { monitor_idx: 1 }, &mut 0);
    event_loop::handle_event(
        &mut state,
        OverlayEvent::FocusLost { monitor_idx: 0 },
        &mut 0,
    );
    assert_eq!(state.input.keyboard_focus, Some(1));
}

/// 【文字光标测试】【分数与选区】不同输出比例、半透明背景和字符选区下反复局部闪烁不叠加字形，完整帧逐字节一致，无参数、无返回值
#[test]
fn fractional_selected_text_does_not_accumulate_outside_caret_damage() {
    for scale in [0.75_f32, 1.0, 1.2, 2.0] {
        let now = Instant::now();
        let mut state = prepared(now);
        state.placements[0].scale = scale;
        let size = ((640.0 * scale) as u32, (480.0 * scale) as u32);
        let mut source = tiny_skia::Pixmap::new(size.0, size.1).unwrap();
        source.fill(tiny_skia::Color::from_rgba8(30, 34, 38, 255));
        state.captures[0].pixmap = source.clone().into();
        state.captures[0].scale = scale;
        state.canvas[0] = source.clone();
        state.dimmed[0] = source;
        state.annotations_layer[0] = tiny_skia::Pixmap::new(size.0, size.1).unwrap();
        state.tool_settings.color = tiny_skia::Color::from_rgba8(255, 77, 77, 128);
        state.tool_settings.text_background = tiny_skia::Color::from_rgba8(45, 80, 120, 100);
        crate::editor::text_session::sync_style(&mut state, &mut 0);
        key(&mut state, "Ctrl+A", &mut 0);
        state.damage_rects.clear();
        state.layer_damage_rects.clear();
        let _ = frame(&mut state, 1);
        for index in 1..=4 {
            let mut dirty = 0;
            crate::app::text_caret::tick(
                &mut state,
                now + Duration::from_millis(index * 500),
                &mut dirty,
            );
            let partial = frame(&mut state, dirty);
            let complete = frame(&mut state, 1);
            let differences = partial
                .iter()
                .zip(&complete)
                .filter(|(left, right)| left != right)
                .count();
            assert_eq!(
                differences, 0,
                "scale {scale}, phase {index}: local redraw accumulated pixels"
            );
        }
    }
}

/// 【文字光标测试】【已有文字】重新编辑时持久选中身份不扩大每次光标损坏，预览仍按窄区域重绘，无参数、无返回值
#[test]
fn existing_annotation_selection_does_not_expand_blink_damage() {
    let now = Instant::now();
    let mut state = prepared(now);
    let mut annotation = state.text_session.as_ref().unwrap().working.clone();
    annotation.id = 7;
    state.annotations.push(annotation);
    crate::editor::text_session::begin_existing(&mut state, 7, &mut 0);
    crate::app::text_caret::tick(&mut state, now, &mut 0);
    state.annotations_dirty = false;
    state.damage_rects.clear();
    state.layer_damage_rects.clear();
    let mut dirty = 0;
    crate::app::text_caret::tick(&mut state, now + Duration::from_millis(500), &mut dirty);
    let rect = state.monitor_dirty_rect(0).unwrap();
    assert!(
        rect.width() < 20.0,
        "the existing selected object expanded the caret area: {rect:?}"
    );
}
