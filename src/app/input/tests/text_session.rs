//! 【文字事务测试】【真实输入】提交前不改持久标注，提交快捷键不退出截图，局部撤销不触及标注历史

use super::*;
use crate::{
    tools::Tool,
    types::{AnnotationShape, MouseButton},
    ui::text_properties::layout::Control,
};

/// 【文字事务测试】【新建准备】返回选区内文字工具编辑器，工具偏好固定为二十点，不依赖已有状态
fn prepared() -> EditorState {
    let mut state = editor();
    state.tool_settings.font_size = 20.0;
    state.selected_tool = Tool::Text;
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    super::super::refresh_panels(&mut state, &mut 0);
    state
}

/// 【文字事务测试】【点击输入】对 state 的 x、y 发送完整左键，dirty 返回损坏区域，无返回值
fn click(state: &mut EditorState, x: f64, y: f64, dirty: &mut u32) {
    pointer(state, x, y, dirty);
    super::super::handle_pointer_button(state, MouseButton::Left, true, dirty);
    super::super::handle_pointer_button(state, MouseButton::Left, false, dirty);
}

/// 【文字事务测试】【正文输入】把 text 送入 state 的真实文字路由，dirty 返回重绘范围，无返回值
fn input(state: &mut EditorState, text: &str, dirty: &mut u32) {
    super::super::handle_key(state, None, Some(text.to_owned()), dirty);
}

/// 【文字事务测试】【正文读取】返回 state 唯一持久文字的正文，用于实际提交和撤销断言
fn content(state: &EditorState) -> &str {
    let AnnotationShape::Text { content, .. } = &state.annotations[0].shape else {
        panic!("text expected")
    };
    content
}

/// 【文字事务测试】【新建提交】新文字在 Esc 提交前没有持久标注或全局历史，提交裁掉末尾空白且不退出截图，无参数、无返回值
#[test]
fn new_text_is_a_draft_until_escape_commits_it() {
    let mut state = prepared();
    let mut dirty = 0;
    click(&mut state, 300.0, 200.0, &mut dirty);
    assert!(state.annotations.is_empty());
    assert!(state.undo_stack.is_empty());
    input(&mut state, "  alpha\n\n  ", &mut dirty);
    assert!(state.annotations.is_empty());
    key(&mut state, "Escape", &mut dirty);
    assert!(!state.cancel);
    assert!(state.text.editing.is_none());
    assert_eq!(content(&state), "  alpha");
    assert_eq!(state.undo_stack.len(), 1);
    state.undo(&mut dirty);
    assert!(state.annotations.is_empty());
    state.redo(&mut dirty);
    assert_eq!(content(&state), "  alpha");
}

/// 【文字事务测试】【空白提交】空草稿和全空白文字不形成持久标注、不消耗标注编号和历史，无参数、无返回值
#[test]
fn empty_or_whitespace_text_does_not_create_annotations_or_history() {
    for text in ["", " \n\t\u{a0} "] {
        let mut state = prepared();
        let mut dirty = 0;
        let next = state.next_id;
        click(&mut state, 300.0, 200.0, &mut dirty);
        input(&mut state, text, &mut dirty);
        key(&mut state, "Ctrl+Return", &mut dirty);
        assert!(state.text.editing.is_none());
        assert!(state.annotations.is_empty());
        assert!(state.undo_stack.is_empty());
        assert_eq!(state.next_id, next);
        assert!(state.finish.is_none());
    }
}

/// 【文字事务测试】【局部撤销】编辑中 Ctrl+Z、Ctrl+Y 只影响草稿，回车插入换行而 Ctrl+回车提交，无参数、无返回值
#[test]
fn editor_undo_redo_stays_local_and_enter_is_not_a_submit() {
    let mut state = prepared();
    let mut dirty = 0;
    click(&mut state, 300.0, 200.0, &mut dirty);
    input(&mut state, "alpha", &mut dirty);
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert!(state.text.editing.is_some());
    assert!(state.annotations.is_empty());
    key(&mut state, "Ctrl+Y", &mut dirty);
    key(&mut state, "Return", &mut dirty);
    input(&mut state, "beta", &mut dirty);
    assert!(state.annotations.is_empty());
    assert!(state.finish.is_none());
    key(&mut state, "Ctrl+Return", &mut dirty);
    assert_eq!(content(&state), "alpha\nbeta");
    assert!(state.finish.is_none());
    assert!(!state.cancel);
}

/// 【文字事务测试】【已有入口】选择工具的编辑按钮不切换工具，重新编辑提交前保留持久正文，提交后同一标注可撤销恢复，无参数、无返回值
#[test]
fn selected_text_editor_keeps_tool_and_commits_one_global_change() {
    let mut state = prepared();
    let mut dirty = 0;
    super::text::add_text(&mut state);
    state.selected_tool = Tool::Pick;
    super::super::refresh_panels(&mut state, &mut dirty);
    let id = state.annotations[0].id;
    let rect = state
        .settings_panel
        .widget_rect(Control::Edit.index())
        .unwrap();
    click(
        &mut state,
        (rect.left() + rect.width() / 2.0) as f64,
        (rect.top() + rect.height() / 2.0) as f64,
        &mut dirty,
    );
    assert_eq!(state.selected_tool, Tool::Pick);
    key(&mut state, "Ctrl+A", &mut dirty);
    input(&mut state, "updated  ", &mut dirty);
    assert_eq!(content(&state), "original");
    assert!(state.undo_stack.is_empty());
    key(&mut state, "Ctrl+Return", &mut dirty);
    assert_eq!(content(&state), "updated");
    assert_eq!(state.annotations[0].id, id);
    assert_eq!(state.selected_annotation, Some(0));
    assert_eq!(state.undo_stack.len(), 1);
    state.undo(&mut dirty);
    assert_eq!(content(&state), "original");
}

/// 【文字事务测试】【状态对照】返回 state 与 Qt 纯文本接口同义的状态，不把预览正文当作持久标注
fn observed(state: &EditorState) -> serde_json::Value {
    let session = state.text_session.as_ref();
    let texts = state
        .annotations
        .iter()
        .map(|annotation| {
            if let AnnotationShape::Text { content, .. } = &annotation.shape {
                content.clone()
            } else {
                String::new()
            }
        })
        .collect::<Vec<_>>();
    serde_json::json!({"texts":texts,"editor_text":session.map(|session|crate::editor::text_session::plain_text(if let AnnotationShape::Text {content,..}=&session.working.shape {content} else {""})).unwrap_or_default(),"editor_visible":session.is_some(),"editor_undo":session.is_some_and(|session|session.history.available(false)),"editor_redo":session.is_some_and(|session|session.history.available(true)),"selected":state.selected_annotation.is_some(),"tool":if state.selected_tool==Tool::Text {"text"} else {"select"}})
}

/// 【文字事务测试】【原版差分】经真实输入路由比较九组 Qt 新建、已有对象和局部撤销记录，显式目录可保存实际结果
#[test]
fn real_input_routes_match_qt_editing_records() {
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-text-editing.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 9);
    let mut actual = Vec::new();
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        let mut state = prepared();
        let mut dirty = 0;
        state.tool_settings.text_family = "Inter".into();
        state.tool_settings.bold = false;
        if name.starts_with("existing-") {
            super::text::add_text(&mut state);
            if let AnnotationShape::Text { content, bold, .. } = &mut state.annotations[0].shape {
                *content = "Mark Shot".into();
                *bold = false;
            }
            crate::editor::annotation_drag::rebuild_annotation(&mut state, 0);
            state.selected_tool = Tool::Pick;
            super::super::refresh_panels(&mut state, &mut dirty);
            let rect = state
                .settings_panel
                .widget_rect(Control::Edit.index())
                .unwrap();
            click(
                &mut state,
                (rect.left() + rect.width() / 2.0) as f64,
                (rect.top() + rect.height() / 2.0) as f64,
                &mut dirty,
            );
        } else {
            click(&mut state, 300.0, 200.0, &mut dirty);
        }
        let mut states = vec![observed(&state)];
        match name {
            "new-escape" | "new-control-enter" | "new-outside" => {
                assert_eq!(
                    state.text_session.as_ref().unwrap().working.bbox.width(),
                    13.0
                );
                assert_eq!(
                    state.text_session.as_ref().unwrap().working.bbox.height(),
                    41.0
                );
                input(&mut state, "  alpha\n\n  ", &mut dirty);
                states.push(observed(&state));
                if name == "new-outside" {
                    click(&mut state, 500.0, 350.0, &mut dirty);
                } else {
                    key(
                        &mut state,
                        if name == "new-escape" {
                            "Escape"
                        } else {
                            "Ctrl+Return"
                        },
                        &mut dirty,
                    );
                }
                states.push(observed(&state));
                state.undo(&mut dirty);
                states.push(observed(&state));
                state.redo(&mut dirty);
                states.push(observed(&state));
            }
            "new-empty" | "new-whitespace" => {
                input(
                    &mut state,
                    if name == "new-empty" {
                        ""
                    } else {
                        " \n\t\u{a0} "
                    },
                    &mut dirty,
                );
                states[0] = observed(&state);
                key(&mut state, "Ctrl+Return", &mut dirty);
                states.push(observed(&state));
            }
            "existing-changed" | "existing-deleted" | "existing-unchanged" => {
                if name != "existing-unchanged" {
                    key(&mut state, "Ctrl+A", &mut dirty);
                    if name == "existing-changed" {
                        input(&mut state, "updated  ", &mut dirty);
                    } else {
                        key(&mut state, "Backspace", &mut dirty);
                    }
                }
                states.push(observed(&state));
                key(&mut state, "Ctrl+Return", &mut dirty);
                states.push(observed(&state));
                state.undo(&mut dirty);
                states.push(observed(&state));
                state.redo(&mut dirty);
                states.push(observed(&state));
            }
            "local-undo" => {
                for ch in "one two".chars() {
                    input(&mut state, &ch.to_string(), &mut dirty);
                }
                states.push(observed(&state));
                key(&mut state, "Return", &mut dirty);
                for ch in "three".chars() {
                    input(&mut state, &ch.to_string(), &mut dirty);
                }
                states.push(observed(&state));
                for _ in 0..4 {
                    key(&mut state, "Ctrl+Z", &mut dirty);
                    states.push(observed(&state));
                }
            }
            _ => panic!("unknown Qt case: {name}"),
        }
        let reference=case["states"].as_array().unwrap().iter().map(|state|serde_json::json!({"texts":state["annotations"].as_array().unwrap().iter().map(|annotation|annotation["text"].clone()).collect::<Vec<_>>(),"editor_text":state["editor_text"],"editor_visible":state["editor_visible"],"editor_undo":state["editor_undo"],"editor_redo":state["editor_redo"],"selected":state["selected"].as_i64().unwrap()!=-1,"tool":state["tool"]})).collect::<Vec<_>>();
        assert_eq!(states, reference, "{name}");
        actual.push(serde_json::json!({"name":name,"states":states}));
    }
    if let Ok(directory) = std::env::var("MARK_SHOT_TEXT_SESSION_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join("traces.json"),
            serde_json::to_string_pretty(&actual).unwrap(),
        )
        .unwrap();
    }
}

/// 【文字事务测试】【已有准备】返回通过真实编辑按钮开始编辑的已有文字，工具保持选择模式
fn existing() -> EditorState {
    let mut state = prepared();
    super::text::add_text(&mut state);
    state.selected_tool = Tool::Pick;
    super::super::refresh_panels(&mut state, &mut 0);
    let rect = state
        .settings_panel
        .widget_rect(Control::Edit.index())
        .unwrap();
    click(
        &mut state,
        (rect.left() + rect.width() / 2.0) as f64,
        (rect.top() + rect.height() / 2.0) as f64,
        &mut 0,
    );
    state
}

/// 【文字事务测试】【删除恢复】删除现有文字后撤销恢复对象与选中，编号和工具偏好按各自语义保留，无参数、无返回值
#[test]
fn deleting_text_restores_its_selection_on_undo() {
    let mut state = existing();
    let mut dirty = 0;
    let id = state.annotations[0].id;
    key(&mut state, "Ctrl+A", &mut dirty);
    key(&mut state, "Backspace", &mut dirty);
    key(&mut state, "Ctrl+Return", &mut dirty);
    assert!(state.annotations.is_empty());
    assert!(state.selected_annotation.is_none());
    state.undo(&mut dirty);
    assert_eq!(state.selected_annotation, Some(0));
    assert_eq!(state.annotations[0].id, id);
    assert_eq!(content(&state), "original");
    state.redo(&mut dirty);
    assert!(state.annotations.is_empty());
}

/// 【文字事务测试】【编号恢复】新文本撤销和重做恢复下一编号，空草稿不占用编号，无参数、无返回值
#[test]
fn committed_text_history_restores_next_annotation_id() {
    let mut state = prepared();
    let mut dirty = 0;
    let next = state.next_id;
    click(&mut state, 300.0, 200.0, &mut dirty);
    input(&mut state, "text", &mut dirty);
    key(&mut state, "Escape", &mut dirty);
    assert_eq!(state.next_id, next + 1);
    state.undo(&mut dirty);
    assert_eq!(state.next_id, next);
    state.redo(&mut dirty);
    assert_eq!(state.next_id, next + 1);
    assert_eq!(state.annotations[0].id, next);
}

/// 【文字事务测试】【属性边界】重新编辑先隐藏属性栏，正文变化不显示；独立字号变化显示属性栏并与文本提交分开撤销，无参数、无返回值
#[test]
fn editing_hides_properties_until_a_style_change_and_keeps_independent_history() {
    let mut state = existing();
    let mut dirty = 0;
    assert!(!state.settings_panel.visible);
    key(&mut state, "Ctrl+A", &mut dirty);
    input(&mut state, "updated", &mut dirty);
    assert!(!state.settings_panel.visible);
    crate::editor::text_edits::set(
        &mut state,
        crate::editor::text_edits::Property::Points(30.0),
        true,
        &mut dirty,
    );
    super::super::refresh_panels(&mut state, &mut dirty);
    assert!(state.settings_panel.visible);
    assert_eq!(content(&state), "original");
    key(&mut state, "Escape", &mut dirty);
    assert_eq!(state.undo_stack.len(), 2);
    assert_eq!(content(&state), "updated");
    state.undo(&mut dirty);
    assert_eq!(content(&state), "original");
    assert_eq!(crate::editor::text_edits::current_points(&state), 30.0);
    state.undo(&mut dirty);
    assert_eq!(crate::editor::text_edits::current_points(&state), 20.0);
    assert_eq!(state.tool_settings.font_size, 30.0);
}

/// 【文字事务测试】【字符拖选】正文内拖动选择字符而不移动标注，右键不提交或切换工具，无参数、无返回值
#[test]
fn text_drag_selects_characters_without_annotation_drag() {
    let mut state = prepared();
    let mut dirty = 0;
    click(&mut state, 300.0, 200.0, &mut dirty);
    input(&mut state, "abcdefgh", &mut dirty);
    let before = state.text_session.as_ref().unwrap().working.bbox;
    pointer(&mut state, 309.0, 212.0, &mut dirty);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 440.0, 212.0, &mut dirty);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(crate::tools::text::session_input::has_selection(&state));
    assert!(state.ann_drag.is_none());
    assert_eq!(state.text_session.as_ref().unwrap().working.bbox, before);
    assert!(state.annotations.is_empty());
    pointer(&mut state, 309.0, 212.0, &mut dirty);
    super::super::handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    assert!(state.text_session.is_some());
    assert_eq!(state.selected_tool, Tool::Text);
}
