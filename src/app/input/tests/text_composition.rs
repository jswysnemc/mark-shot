//! 【输入法测试】【生产事务】通过真实编辑入口比较 Qt 状态，候选显示与已确认正文分别验证

use super::*;
use crate::{
    editor::{
        text_composition,
        text_session::{self, PREVIEW_ID},
    },
    types::{
        AnnotationShape,
        text_input::{Edit, Preedit, Replacement},
    },
};
use cosmic_text::{Cursor, Edit as _, Selection};

/// 【输入法测试】【会话准备】按 initial、cursor、anchor 的 Qt UTF-16 位置返回真实编辑状态，不加载系统识别模型
fn prepared(initial: &str, cursor: usize, anchor: usize) -> EditorState {
    let mut state = editor();
    state.selected_tool = crate::tools::Tool::Text;
    state.tool_settings.font_size = 20.0;
    state.tool_settings.text_family = "Inter".into();
    state.tool_settings.color = Color::from_rgba8(255, 77, 77, 255);
    state.input.keyboard_focus = Some(0);
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.selection.prev_zone = state.selection.zone;
    text_session::begin_new(&mut state, (300.0, 240.0), 1.0, &mut 0);
    crate::tools::text::session_input::text(&mut state, initial, &mut 0);
    let cursor = byte_at_utf16(initial, cursor).unwrap();
    let anchor = byte_at_utf16(initial, anchor).unwrap();
    let input = state.text.editors.get_mut(&PREVIEW_ID).unwrap();
    input.set_cursor(
        crate::features::text::input_method::positions::editor_cursor(initial, cursor).unwrap(),
    );
    input.set_selection(if cursor == anchor {
        Selection::None
    } else {
        Selection::Normal(
            crate::features::text::input_method::positions::editor_cursor(initial, anchor).unwrap(),
        )
    });
    state
}

/// 【输入法测试】【UTF-16 适配】将 text 中的 units Qt 位置转换为 UTF-8 字节，代理对中间位置返回空值；仅供原版样本适配
fn byte_at_utf16(text: &str, units: usize) -> Option<usize> {
    let mut consumed = 0;
    for (byte, character) in text.char_indices() {
        if consumed == units {
            return Some(byte);
        }
        consumed += character.len_utf16();
        if consumed > units {
            return None;
        }
    }
    (consumed == units).then_some(text.len())
}

/// 【输入法测试】【正文读取】从 state 当前草稿返回已确认文本，没有编辑会话时返回空字符串
fn content(state: &EditorState) -> &str {
    match state
        .text_session
        .as_ref()
        .map(|session| &session.working.shape)
    {
        Some(AnnotationShape::Text { content, .. }) => content,
        _ => "",
    }
}

/// 【输入法测试】【Qt 事件适配】按 state 当前文档把 event 的 UTF-16 相对替换转换为生产字节事件，返回值不包含手写编辑结果
fn event(state: &EditorState, event: &serde_json::Value) -> Edit {
    let preedit = event["preedit"].as_str().unwrap();
    let caret = event["caret"].as_i64().unwrap();
    let caret = if caret == -1 {
        None
    } else {
        Some(if caret == -2 {
            preedit.len()
        } else {
            byte_at_utf16(preedit, caret as usize).unwrap()
        })
    };
    let commit = event["commit"].as_str().unwrap().to_owned();
    let replacement = if event["replace_start"] == 0 && event["replace_length"] == 0 {
        None
    } else {
        let mut text = content(state).to_owned();
        let editor = &state.text.editors[&PREVIEW_ID];
        let positions = crate::features::text::input_method::positions::document_offset;
        let mut cursor = positions(&text, editor.cursor()).unwrap();
        if let Some((start, end)) = editor.selection_bounds() {
            let start = positions(&text, start).unwrap();
            let end = positions(&text, end).unwrap();
            text.replace_range(start..end, "");
            cursor = start;
        }
        let old_units = text[..cursor].encode_utf16().count() as i64;
        let start_units = old_units + event["replace_start"].as_i64().unwrap();
        let end_units = start_units + event["replace_length"].as_i64().unwrap();
        let start = byte_at_utf16(&text, start_units as usize).unwrap();
        let end = byte_at_utf16(&text, end_units as usize).unwrap();
        Some(Replacement {
            start: start as isize - cursor as isize,
            length: end - start,
        })
    };
    Edit {
        preedit: Preedit {
            text: preedit.to_owned(),
            cursor: caret.map(|cursor| (cursor, cursor)),
        },
        commit,
        replacement,
    }
}

/// 【输入法测试】【实际状态】把 state 的真实正文、光标与候选序列化为 Qt 可比较状态，不比较 Qt 内部增量计数
fn snapshot(state: &EditorState) -> serde_json::Value {
    let text = content(state);
    let input = state.text.editors.get(&PREVIEW_ID);
    let offset = |cursor: Cursor| {
        crate::features::text::input_method::positions::document_offset(text, cursor)
            .map_or(0, |byte| text[..byte].encode_utf16().count())
    };
    let cursor = input.map_or(0, |input| offset(input.cursor()));
    let anchor = input.map_or(0, |input| match input.selection() {
        Selection::Normal(anchor) | Selection::Word(anchor) | Selection::Line(anchor) => {
            offset(anchor)
        }
        Selection::None => cursor,
    });
    let composition = state
        .text_session
        .as_ref()
        .and_then(|session| session.composition.as_ref());
    serde_json::json!({"editor_text": text, "editor_cursor": cursor, "anchor": anchor,
    "preedit": composition.map_or("", |composition| composition.preedit.text.as_str()),
    "preedit_position": composition.map_or(-1, |composition| {
        let line = text.split('\n').nth(composition.position.line).unwrap();
        line[..composition.position.index].encode_utf16().count() as i64
    })})
}

/// 【输入法测试】【原版逐状态】十七组真实 Qt 输入法事件交给生产编辑器，逐项比较正文、光标、选区和候选，不筛掉相对替换与跨段落场景，无参数、无返回值
#[test]
fn qt_input_method_events_match_production_composition_states() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-text-input-method.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let mut state = prepared(
            case["initial"].as_str().unwrap(),
            case["cursor"].as_u64().unwrap() as usize,
            case["anchor"].as_u64().unwrap() as usize,
        );
        let mut actual = vec![snapshot(&state)];
        for item in case["events"].as_array().unwrap() {
            match item["kind"].as_str().unwrap() {
                "ime" => {
                    let event = event(&state, item);
                    assert!(
                        text_composition::apply(&mut state, event, &mut 0),
                        "{name}: production input method target was not handled"
                    );
                }
                "undo" => key(&mut state, "Ctrl+Z", &mut 0),
                "redo" => key(&mut state, "Ctrl+Shift+Z", &mut 0),
                "annotation-commit" => {
                    text_session::commit(&mut state, &mut 0);
                }
                "focus-out" => {
                    state.input.keyboard_focus = None;
                }
                _ => panic!("unknown fixture operation"),
            }
            actual.push(snapshot(&state));
        }
        for (index, (actual, expected)) in actual
            .iter()
            .zip(case["states"].as_array().unwrap())
            .enumerate()
        {
            for field in [
                "editor_text",
                "editor_cursor",
                "anchor",
                "preedit",
                "preedit_position",
            ] {
                assert_eq!(
                    actual[field], expected[field],
                    "{name}, state {index}, field {field}"
                );
            }
        }
    }
}

/// 【输入法测试】【生产显示】通过真实帧绘制返回正文附近像素，复用生产活动身份，不经过假渲染器
fn whole_frame(state: &mut EditorState, area: Option<tiny_skia::Rect>) -> Vec<u8> {
    let mut dirty = 1;
    let now = state
        .text_session
        .as_ref()
        .and_then(|session| session.caret.deadline())
        .map_or_else(std::time::Instant::now, |deadline| {
            deadline - crate::features::text::caret::FLASH_PERIOD / 2
        });
    crate::app::text_caret::tick(state, now, &mut dirty);
    let frame = crate::app::frame_render::Frame {
        active_text_id: crate::app::frame_render::active_text_id(state),
        scan_badge: None,
        current_color: state.tool_settings.color,
        outline_color: state.tool_settings.outline_color,
        annotations_hidden: false,
    };
    let mut pacing = crate::app::frame_render::Pacing::new(1);
    crate::app::frame_render::render_monitor(state, 0, &frame, area, &mut pacing.dimmed_for[0]);
    crate::app::frame_render::clear_frame_state(state, &mut dirty, &pacing);
    state.canvas[0].data().to_vec()
}

/// 【输入法测试】【正文区域】从 state 的真实完整帧提取正文附近像素，不读取候选缓存替代实际屏幕显示
fn body_frame(state: &mut EditorState) -> Vec<u8> {
    let pixels = whole_frame(state, None);
    let mut region = Vec::new();
    for y in 230..310 {
        let start = (y * 640 + 290) * 4;
        region.extend_from_slice(&pixels[start..start + 110 * 4]);
    }
    region
}

/// 【输入法测试】【隐藏光标】显式隐藏候选光标不隐藏候选字形，也不改变真实文档光标，无参数、无返回值
#[test]
fn hidden_preedit_caret_preserves_candidate_pixels_and_the_committed_cursor() {
    let mut state = prepared("", 0, 0);
    let shown = Edit {
        preedit: Preedit::from_protocol("ni ".into(), 3, 3).unwrap(),
        ..Default::default()
    };
    text_composition::apply(&mut state, shown, &mut 0);
    let shown = body_frame(&mut state);
    let hidden = Edit {
        preedit: Preedit::from_protocol("ni ".into(), -1, -1).unwrap(),
        ..Default::default()
    };
    text_composition::apply(&mut state, hidden, &mut 0);
    let hidden = body_frame(&mut state);
    let changes = shown
        .iter()
        .zip(hidden)
        .filter(|(left, right)| **left != *right)
        .count();
    assert!(
        changes > 0 && changes < 200,
        "only the narrow caret may change: {changes}"
    );
    assert_eq!(state.text.editors[&PREVIEW_ID].cursor(), Cursor::new(0, 0));
    assert_eq!(content(&state), "");
}

/// 【输入法测试】【完整拒绝】非法相对替换不删除原选区，不产生历史，也不清除此前有效候选，无参数、无返回值
#[test]
fn invalid_replacement_preserves_the_entire_committed_state() {
    let mut state = prepared("alpha beta", 7, 3);
    let before = snapshot(&state);
    let invalid = Edit {
        preedit: Preedit::from_protocol("ni".into(), 2, 2).unwrap(),
        replacement: Some(Replacement {
            start: -1000,
            length: 1,
        }),
        ..Default::default()
    };
    assert!(!text_composition::apply(&mut state, invalid, &mut 0));
    assert_eq!(snapshot(&state), before);
    let valid = Edit {
        preedit: Preedit::from_protocol("ni".into(), 2, 2).unwrap(),
        ..Default::default()
    };
    text_composition::apply(&mut state, valid, &mut 0);
    let before = snapshot(&state);
    let invalid = Edit {
        replacement: Some(Replacement {
            start: 1000,
            length: 1,
        }),
        ..Default::default()
    };
    assert!(!text_composition::apply(&mut state, invalid, &mut 0));
    assert_eq!(snapshot(&state), before);
}

/// 【输入法测试】【隐藏停止】显式隐藏候选光标取消周期唤醒，恢复普通候选重新计时，不改变正文，无参数、无返回值
#[test]
fn hidden_preedit_caret_stops_periodic_wakeups_and_restores_on_visible_preedit() {
    let now = std::time::Instant::now();
    let mut state = prepared("", 0, 0);
    crate::app::text_caret::tick(&mut state, now, &mut 0);
    assert!(
        state
            .text_session
            .as_ref()
            .unwrap()
            .caret
            .deadline()
            .is_some()
    );
    text_composition::apply(
        &mut state,
        Edit {
            preedit: Preedit::from_protocol("ni".into(), -1, -1).unwrap(),
            ..Default::default()
        },
        &mut 0,
    );
    crate::app::text_caret::tick(&mut state, now, &mut 0);
    assert!(
        state
            .text_session
            .as_ref()
            .unwrap()
            .caret
            .deadline()
            .is_none()
    );
    state.damage_rects.clear();
    let mut dirty = 0;
    crate::app::text_caret::tick(
        &mut state,
        now + std::time::Duration::from_secs(3),
        &mut dirty,
    );
    assert_eq!(dirty, 0);
    assert!(state.damage_rects.is_empty());
    text_composition::apply(
        &mut state,
        Edit {
            preedit: Preedit::from_protocol("ni".into(), 2, 2).unwrap(),
            ..Default::default()
        },
        &mut 0,
    );
    crate::app::text_caret::tick(&mut state, now + std::time::Duration::from_secs(3), &mut 0);
    assert!(
        state
            .text_session
            .as_ref()
            .unwrap()
            .caret
            .deadline()
            .is_some()
    );
    assert_eq!(content(&state), "");
}

/// 【输入法测试】【分数局部】候选下划线、选区和透明背景在多种比例下局部与整帧逐字节一致，闪烁不重建候选全文或持久图层，无参数、无返回值
#[test]
fn fractional_composition_blinks_keep_exact_pixels_and_reuse_the_preview() {
    for scale in [0.75_f32, 1.0, 1.2, 2.0] {
        let mut state = prepared("", 0, 0);
        state.placements[0].scale = scale;
        let size = ((640.0 * scale) as u32, (480.0 * scale) as u32);
        let mut source = Pixmap::new(size.0, size.1).unwrap();
        source.fill(Color::from_rgba8(30, 34, 38, 255));
        state.captures[0].pixmap = source.clone();
        state.captures[0].scale = scale;
        state.canvas[0] = source.clone();
        state.dimmed[0] = source;
        state.annotations_layer[0] = Pixmap::new(size.0, size.1).unwrap();
        state.text_session.as_mut().unwrap().working.color = Color::from_rgba8(255, 77, 77, 128);
        let AnnotationShape::Text { appearance, .. } =
            &mut state.text_session.as_mut().unwrap().working.shape
        else {
            unreachable!()
        };
        appearance.background = Color::from_rgba8(40, 90, 145, 100);
        let event = Edit {
            preedit: Preedit::from_protocol("ni ".into(), 0, 2).unwrap(),
            ..Default::default()
        };
        text_composition::apply(&mut state, event, &mut 0);
        whole_frame(&mut state, None);
        state.annotations_dirty = false;
        state.layer_damage_rects.clear();
        let data = match &state.text_session.as_ref().unwrap().presented().shape {
            AnnotationShape::Text { content, .. } => content.as_ptr(),
            _ => unreachable!(),
        };
        for _ in 0..4 {
            let deadline = state
                .text_session
                .as_ref()
                .unwrap()
                .caret
                .deadline()
                .unwrap();
            let mut dirty = 0;
            crate::app::text_caret::tick(&mut state, deadline, &mut dirty);
            let area = state.monitor_dirty_rect(0).unwrap();
            assert!(area.width() < 20.0, "{scale}: {area:?}");
            assert!(!state.annotations_dirty);
            assert!(state.layer_damage_rects.is_empty());
            let partial = whole_frame(&mut state, Some(area));
            let full = whole_frame(&mut state, None);
            let changes = partial
                .iter()
                .zip(full)
                .filter(|(left, right)| **left != *right)
                .count();
            assert_eq!(changes, 0, "{scale}: local candidate pixels accumulated");
            text_composition::refresh_style(&mut state, &mut 0);
            let AnnotationShape::Text { content, .. } =
                &state.text_session.as_ref().unwrap().presented().shape
            else {
                unreachable!()
            };
            assert_eq!(
                content.as_ptr(),
                data,
                "a caret frame rebuilt the candidate text"
            );
        }
    }
}

/// 【输入法测试】【显示隔离】候选实际出现在生产帧但不写入正文，取消精确恢复原像素，不留下缓存或持久图层损坏，无参数、无返回值
#[test]
fn composing_text_is_visible_without_entering_the_document_or_persistent_layer() {
    let mut state = prepared("", 0, 0);
    let before = body_frame(&mut state);
    state.annotations_dirty = false;
    state.layer_damage_rects.clear();
    let event = Edit {
        preedit: Preedit::from_protocol("ni".into(), 2, 2).unwrap(),
        ..Default::default()
    };
    assert!(text_composition::apply(&mut state, event, &mut 0));
    assert_eq!(content(&state), "");
    assert!(!state.annotations_dirty);
    assert!(state.layer_damage_rects.is_empty());
    let shown = body_frame(&mut state);
    let red = |pixels: &[u8]| {
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| pixel[0] > 150 && pixel[1] < 100 && pixel[2] < 100)
            .count()
    };
    assert!(
        red(&shown) > red(&before) + 20,
        "the candidate was not drawn through the production preview"
    );
    assert!(text_composition::cancel(&mut state, &mut 0));
    let restored = body_frame(&mut state);
    let differences = before
        .iter()
        .zip(restored)
        .filter(|(left, right)| **left != *right)
        .count();
    assert_eq!(differences, 0, "cancelled candidate left different pixels");
    assert!(
        !state
            .text
            .editors
            .contains_key(&text_composition::PREEDIT_ID)
    );
}
