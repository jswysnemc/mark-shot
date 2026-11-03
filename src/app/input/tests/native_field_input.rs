//! 【字段输入测试】【边界隔离】验证实际字段原生上下文、目标身份与字段局部历史

use super::native_field_support::*;
use crate::{
    app::{panels, text_input},
    types::{
        OverlayEvent,
        text_input::{Batch, ContentType, Focus, Preedit, Update},
    },
    ui::{font_panel::state::Field, text_field::CursorInit},
};

/// 【字段输入测试】【候选批次】按 text 返回有可见末尾光标的候选批次，不确认正文
fn candidate(text: &str) -> Batch {
    Batch {
        preedit: Preedit {
            text: text.into(),
            cursor: Some((text.len(), text.len())),
        },
        ..Default::default()
    }
}

/// 【字段输入测试】【候选快照】实际搜索过滤不读取候选，快照只保留已确认正文；静止同步复用缓存，无参数、无返回值
#[test]
fn candidates_do_not_filter_families_or_allocate_stationary_snapshots() {
    let mut state = prepared("search", "Inter", 5, 5);
    panels::text::keyboard::apply(&mut state, &mut 0);
    let first = sync(&mut state).pop().unwrap().unwrap();
    assert_eq!(first.content, ContentType::Text);
    assert_eq!(state.font_panel.filtered, vec![0, 1]);
    deliver(&mut state, candidate("ni"), true);
    assert_eq!(state.font_panel.query(), "Inter");
    assert_eq!(state.font_panel.filtered, vec![0, 1]);
    let next = sync(&mut state).pop().unwrap().unwrap();
    assert_eq!(next.surrounding, first.surrounding);
    assert_ne!(next.cursor_rect, first.cursor_rect);
    let context = state
        .input
        .text_input
        .snapshot
        .as_ref()
        .unwrap()
        .surrounding
        .as_ref()
        .unwrap()
        .text
        .as_ptr();
    let layout = state.input.text_input.field_layout.unwrap();
    for _ in 0..5 {
        assert!(sync(&mut state).is_empty());
    }
    assert_eq!(
        context,
        state
            .input
            .text_input
            .snapshot
            .as_ref()
            .unwrap()
            .surrounding
            .as_ref()
            .unwrap()
            .text
            .as_ptr()
    );
    assert_eq!(
        layout.cursor_x,
        state.input.text_input.field_layout.unwrap().cursor_x
    );
    assert_eq!(content(&state), "body");
}

/// 【字段输入测试】【字段切换】字号切换与程序替换使旧目标失效，数字用途不带多行属性，无参数、无返回值
#[test]
fn field_switch_and_programmatic_reset_reject_queued_payloads() {
    let mut state = prepared("search", "", 0, 0);
    let old = sync(&mut state)[0].as_ref().unwrap().target_id;
    panels::text::font_popup::focus(&mut state, Field::Size, CursorInit::SelectAll);
    let stale = OverlayEvent::TextInput(Update {
        target_id: old,
        serial: 1,
        sync_allowed: true,
        batch: Batch {
            commit: "wrong".into(),
            ..Default::default()
        },
    });
    text_input::event(&mut state, stale, &mut 0);
    assert_eq!(field(&state).text, "20 pt");
    let size = sync(&mut state).pop().unwrap().unwrap();
    assert_eq!(size.content, ContentType::Digits);
    assert_ne!(size.target_id, old);
    assert_eq!(
        (
            size.surrounding.as_ref().unwrap().cursor,
            size.surrounding.as_ref().unwrap().anchor
        ),
        (2, 0)
    );
    state
        .font_panel
        .fields
        .set_editing_text(Field::Size, "30 pt".into());
    text_input::event(
        &mut state,
        OverlayEvent::TextInput(Update {
            target_id: size.target_id,
            serial: 2,
            sync_allowed: false,
            batch: Batch {
                commit: "40".into(),
                ..Default::default()
            },
        }),
        &mut 0,
    );
    assert_eq!(field(&state).text, "30 pt");
    assert!(state.input.text_input.sync_allowed);
    assert_eq!(content(&state), "body");
}

/// 【字段输入测试】【原生焦点】键盘失焦不清候选，匹配 leave 才取消，其他表面焦点不能启用旧字段，无参数、无返回值
#[test]
fn native_focus_ownership_and_leave_cancel_only_the_current_field() {
    let mut state = prepared("search", "", 0, 0);
    sync(&mut state);
    deliver(&mut state, candidate("ni"), true);
    crate::app::event_loop::handle_event(
        &mut state,
        OverlayEvent::FocusLost { monitor_idx: 0 },
        &mut 0,
    );
    assert!(field(&state).composition.is_some());
    text_input::event(
        &mut state,
        OverlayEvent::TextInputLeave { focus_id: 9099 },
        &mut 0,
    );
    assert!(field(&state).composition.is_some());
    text_input::event(
        &mut state,
        OverlayEvent::TextInputLeave { focus_id: 9100 },
        &mut 0,
    );
    assert!(field(&state).composition.is_none());
    assert!(state.input.text_input.field_layout.is_none());
    text_input::event(
        &mut state,
        OverlayEvent::TextInputFocus(Focus {
            id: 9101,
            monitor_idx: 1,
        }),
        &mut 0,
    );
    assert_eq!(sync(&mut state), vec![None]);
    text_input::event(
        &mut state,
        OverlayEvent::TextInputFocus(Focus {
            id: 9102,
            monitor_idx: 0,
        }),
        &mut 0,
    );
    assert!(sync(&mut state)[0].is_some());
}

/// 【字段输入测试】【非法整批】无效混合确认不产生部分文字，候选与原选择独立保留，局部撤销不能穿透标注历史，无参数、无返回值
#[test]
fn invalid_commits_preserve_selection_and_local_history_does_not_edit_annotations() {
    let mut state = prepared("size", "20 pt", 2, 0);
    sync(&mut state);
    deliver(
        &mut state,
        Batch {
            commit: "40你".into(),
            ..candidate("ni")
        },
        true,
    );
    assert_eq!(field(&state).text, "20 pt");
    assert_eq!(field(&state).selection_range(), Some((0, 2)));
    assert!(field(&state).composition.is_some());
    assert!(!field(&state).can_undo());
    deliver(&mut state, Batch::default(), true);
    assert_eq!(field(&state).text, " pt");
    super::key(&mut state, "ctrl+z", &mut 0);
    assert_eq!(field(&state).text, "20 pt");
    super::key(&mut state, "ctrl+z", &mut 0);
    assert_eq!(content(&state), "body");
    assert_eq!(crate::editor::text_edits::current_points(&state), 20.0);
}

/// 【字段输入测试】【同步许可】不匹配序号确认先修改字段，延迟同步不能污染已发送正文，匹配后恢复，无参数、无返回值
#[test]
fn mismatched_serial_defers_field_context_until_a_matching_done() {
    let mut state = prepared("search", "", 0, 0);
    sync(&mut state);
    deliver(
        &mut state,
        Batch {
            commit: "你".into(),
            ..candidate("hao")
        },
        false,
    );
    assert_eq!(field(&state).text, "你");
    assert!(sync(&mut state).is_empty());
    assert_eq!(
        state
            .input
            .text_input
            .snapshot
            .as_ref()
            .unwrap()
            .surrounding
            .as_ref()
            .unwrap()
            .text,
        ""
    );
    deliver(
        &mut state,
        Batch {
            commit: "好".into(),
            ..Default::default()
        },
        true,
    );
    assert_eq!(
        sync(&mut state)[0]
            .as_ref()
            .unwrap()
            .surrounding
            .as_ref()
            .unwrap()
            .text,
        "你好"
    );
    super::key(&mut state, "ctrl+z", &mut 0);
    assert_eq!(field(&state).text, "");
    assert_eq!(content(&state), "body");
}

/// 【字段输入测试】【逻辑几何】字号与加权颜色字段在分数输出上使用逻辑位置，移动面板只更新矩形，非法字节整批拒绝，无参数、无返回值
#[test]
fn field_geometry_is_output_scale_independent_and_invalid_bytes_are_atomic() {
    for role in ["search", "hex"] {
        let initial = if role == "hex" { "#112233" } else { "你" };
        let mut state = prepared(
            role,
            initial,
            initial.chars().count(),
            initial.chars().count(),
        );
        let first = sync(&mut state).pop().unwrap().unwrap();
        state.placements[0].scale = 1.2;
        assert!(sync(&mut state).is_empty());
        state.placements[0].scale = 2.0;
        assert!(sync(&mut state).is_empty());
        let layout = state.input.text_input.field_layout.unwrap();
        if role == "hex" {
            state.color_popover.render_pos.0 += 10.0;
        } else {
            state.font_panel.render_pos.0 += 10.0;
        }
        let moved = sync(&mut state).pop().unwrap().unwrap();
        assert_eq!(
            moved.cursor_rect.unwrap().0,
            first.cursor_rect.unwrap().0 + 10
        );
        assert_eq!(
            layout.cursor_x,
            state.input.text_input.field_layout.unwrap().cursor_x
        );
        deliver(&mut state, candidate("ni"), true);
        let before = field(&state).composition.as_ref().unwrap().preedit.clone();
        deliver(
            &mut state,
            Batch {
                delete_before: u32::MAX,
                commit: "bad".into(),
                ..candidate("later")
            },
            true,
        );
        assert_eq!(field(&state).text, initial);
        assert_eq!(field(&state).composition.as_ref().unwrap().preedit, before);
        if role == "search" {
            deliver(
                &mut state,
                Batch {
                    delete_before: 1,
                    commit: "x".into(),
                    ..Default::default()
                },
                true,
            );
            assert_eq!(field(&state).text, "你");
            assert_eq!(field(&state).composition.as_ref().unwrap().preedit, before);
        }
    }
}

/// 【字段输入测试】【通用步进】实际属性字段发送数值用途，候选和非法混合确认不修改倍率，合法确认与局部撤销保持中间状态，无参数、无返回值
#[test]
fn generic_stepper_routes_number_purpose_and_only_applies_finite_confirmed_values() {
    use crate::ui::settings_panel::{SettingsWidget, StepperRange};
    let mut state = super::editor();
    state.selected_tool = crate::tools::Tool::Magnifier;
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    crate::editor::magnifier_edits::set_zoom(&mut state, 2.0, false, &mut 0);
    super::super::refresh_panels(&mut state, &mut 0);
    let index = state
        .settings_panel
        .widgets
        .iter()
        .position(|widget| {
            matches!(
                widget,
                SettingsWidget::Stepper {
                    range: StepperRange::Magnification,
                    ..
                }
            )
        })
        .unwrap();
    state
        .settings_panel
        .fields
        .begin_edit(index, "2".into(), CursorInit::SelectAll);
    text_input::event(
        &mut state,
        OverlayEvent::TextInputFocus(Focus {
            id: 9200,
            monitor_idx: 0,
        }),
        &mut 0,
    );
    assert_eq!(
        sync(&mut state)[0].as_ref().unwrap().content,
        ContentType::Number
    );
    deliver(
        &mut state,
        Batch {
            commit: "3.5你".into(),
            ..candidate("ni")
        },
        true,
    );
    assert_eq!(crate::editor::magnifier_edits::current_zoom(&state), 2.0);
    assert_eq!(
        state
            .settings_panel
            .fields
            .editing
            .as_ref()
            .unwrap()
            .field
            .text,
        "2"
    );
    deliver(
        &mut state,
        Batch {
            commit: "2.5".into(),
            ..Default::default()
        },
        true,
    );
    assert_eq!(crate::editor::magnifier_edits::current_zoom(&state), 2.5);
    assert_eq!(
        state
            .settings_panel
            .fields
            .editing
            .as_ref()
            .unwrap()
            .field
            .text,
        "2.5"
    );
    super::key(&mut state, "ctrl+z", &mut 0);
    assert_eq!(crate::editor::magnifier_edits::current_zoom(&state), 2.0);
    assert_eq!(
        state
            .settings_panel
            .fields
            .editing
            .as_ref()
            .unwrap()
            .field
            .text,
        "2"
    );
    super::key(&mut state, "ctrl+a", &mut 0);
    deliver(
        &mut state,
        Batch {
            commit: "-".into(),
            ..Default::default()
        },
        true,
    );
    assert_eq!(
        state
            .settings_panel
            .fields
            .editing
            .as_ref()
            .unwrap()
            .field
            .text,
        "-"
    );
    assert_eq!(crate::editor::magnifier_edits::current_zoom(&state), 2.0);
}
