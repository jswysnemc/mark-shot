//! 【文字菜单测试】【真实路由】右键不改变正文与选区，菜单关闭和编辑提交分别处理

use super::*;
use crate::{tools::Tool, types::MouseButton};
use cosmic_text::Edit;

/// 【文字菜单测试】【编辑准备】返回已有正文和局部历史的独立预览，不依赖剪贴板或持久标注
fn prepared() -> EditorState {
    let mut state = editor();
    state.tool_settings.font_size = 20.0;
    state.selected_tool = Tool::Text;
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    crate::editor::text_session::begin_new(&mut state, (300.0, 200.0), 1.0, &mut 0);
    super::super::handle_key(&mut state, None, Some("alpha beta".into()), &mut 0);
    state
}

/// 【文字菜单测试】【右键输入】在 state 正文内发送完整右键，dirty 返回重绘范围，无返回值
fn open(state: &mut EditorState, dirty: &mut u32) {
    pointer(state, 305.0, 210.0, dirty);
    super::super::handle_pointer_button(state, MouseButton::Right, true, dirty);
    super::super::handle_pointer_button(state, MouseButton::Right, false, dirty);
}

/// 【文字菜单测试】【右键保留】真实右键打开菜单，字符选区、正文、工具和全局历史均不变化，无参数、无返回值
#[test]
fn right_click_opens_a_menu_without_moving_text_selection() {
    let mut state = prepared();
    let mut dirty = 0;
    key(&mut state, "Ctrl+A", &mut dirty);
    let editor = state
        .text
        .editors
        .get(&crate::editor::text_session::PREVIEW_ID)
        .unwrap();
    let selection = (editor.cursor(), editor.selection());
    open(&mut state, &mut dirty);
    assert!(
        state.text_context_menu.is_some(),
        "right mouse must open the text context menu"
    );
    let editor = state
        .text
        .editors
        .get(&crate::editor::text_session::PREVIEW_ID)
        .unwrap();
    assert_eq!((editor.cursor(), editor.selection()), selection);
    assert_eq!(state.selected_tool, Tool::Text);
    assert!(state.annotations.is_empty());
    assert!(state.undo_stack.is_empty());
}

/// 【文字菜单测试】【原版状态】对九组真实 Qt 记录比较菜单项目、快捷键、可用性和几何，不依赖本机剪贴板
#[test]
fn menu_rows_and_geometry_match_qt_records() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../../tests/fixtures/qt-text-menu.json")).unwrap();
    assert_eq!(cases.len(), 9);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let mut state = prepared();
        let mut dirty = 0;
        if name == "empty" {
            key(&mut state, "Ctrl+Z", &mut dirty);
            crate::editor::text_session::commit(&mut state, &mut dirty);
            crate::editor::text_session::begin_new(&mut state, (300.0, 200.0), 1.0, &mut dirty);
        }
        if matches!(name, "selected" | "copy" | "cut" | "delete") {
            key(&mut state, "Ctrl+A", &mut dirty);
        }
        if name == "redo" {
            key(&mut state, "Ctrl+Z", &mut dirty);
        }
        let rows = crate::app::text_context_menu::rows(&state, name != "empty");
        let menu = crate::ui::context_menu::ContextMenu::new(
            rows,
            &mut state.text.font_system,
            (0.0, 0.0),
            (640.0, 480.0),
            0,
        );
        let actual = menu.rows.iter().zip(&menu.layout.rows).map(|(row, rect)| serde_json::json!({
            "label":row.label,"shortcut":row.shortcut,"enabled":row.enabled,"separator":row.separator(),
            "geometry":[rect.left(),rect.top(),rect.width(),rect.height()]
        })).collect::<Vec<_>>();
        let expected = case["rows"].as_array().unwrap();
        assert_eq!(actual.len(), expected.len());
        for (row, reference) in actual.iter().zip(expected) {
            for field in ["label", "shortcut", "enabled", "separator"] {
                assert_eq!(row[field], reference[field], "{name} {field}");
            }
            for index in 0..4 {
                assert_eq!(
                    row["geometry"][index].as_f64(),
                    reference["geometry"][index].as_f64(),
                    "{name} geometry {index}"
                );
            }
        }
        assert_eq!(
            menu.layout.size,
            (
                case["size"][0].as_f64().unwrap() as f32,
                case["size"][1].as_f64().unwrap() as f32
            )
        );
    }
}

/// 【文字菜单测试】【键盘独占】Esc 只关闭菜单，正文与保存快捷键在菜单内均不穿透，关闭后 Esc 才提交，无参数、无返回值
#[test]
fn menu_escape_dismisses_without_committing_and_keys_do_not_leak() {
    let mut state = prepared();
    let mut dirty = 0;
    open(&mut state, &mut dirty);
    super::super::handle_key(&mut state, None, Some("leaked".into()), &mut dirty);
    key(&mut state, "Ctrl+S", &mut dirty);
    assert!(state.finish.is_none());
    key(&mut state, "Escape", &mut dirty);
    assert!(state.text_context_menu.is_none());
    assert!(state.text_session.is_some());
    assert!(state.annotations.is_empty());
    key(&mut state, "Escape", &mut dirty);
    assert!(state.text_session.is_none());
    assert!(
        matches!(&state.annotations[0].shape, crate::types::AnnotationShape::Text { content, .. } if content == "alpha beta")
    );
}

/// 【文字菜单测试】【外部吞键】菜单外第一次左键只关闭菜单，正文不提交且不开始标注或字符拖选，无参数、无返回值
#[test]
fn outside_click_dismisses_menu_without_starting_canvas_gestures() {
    let mut state = prepared();
    let mut dirty = 0;
    open(&mut state, &mut dirty);
    pointer(&mut state, 60.0, 180.0, &mut dirty);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(state.text_context_menu.is_none());
    assert!(state.text_session.is_some());
    assert!(state.annotations.is_empty());
    assert!(!state.text_session.as_ref().unwrap().dragging);
    assert!(!state.input.left_canvas);
}

/// 【文字菜单测试】【行选择】按 state 菜单 index 行执行完整左键，dirty 返回区域，无返回值
fn row_click(state: &mut EditorState, index: usize, dirty: &mut u32) {
    let menu = state.text_context_menu.as_ref().unwrap();
    let rect = menu.layout.rows[index];
    let point = (
        menu.position.0 + rect.left() + 20.0,
        menu.position.1 + rect.top() + rect.height() / 2.0,
    );
    pointer(state, f64::from(point.0), f64::from(point.1), dirty);
    super::super::handle_pointer_button(state, MouseButton::Left, true, dirty);
    super::super::handle_pointer_button(state, MouseButton::Left, false, dirty);
}

/// 【文字菜单测试】【删除局部历史】无选区删除禁用，全选再删除可撤销和重做，全程不提交文字，无参数、无返回值
#[test]
fn delete_only_removes_selection_and_menu_history_stays_local() {
    let mut state = prepared();
    let mut dirty = 0;
    open(&mut state, &mut dirty);
    row_click(&mut state, 6, &mut dirty);
    assert!(state.text_context_menu.is_some());
    row_click(&mut state, 8, &mut dirty);
    assert!(state.text_context_menu.is_none());
    assert!(crate::tools::text::session_input::has_selection(&state));
    open(&mut state, &mut dirty);
    row_click(&mut state, 6, &mut dirty);
    assert!(state.text_context_menu.is_none());
    let content = |state: &EditorState| {
        if let crate::types::AnnotationShape::Text { content, .. } =
            &state.text_session.as_ref().unwrap().working.shape
        {
            content.clone()
        } else {
            unreachable!()
        }
    };
    assert_eq!(content(&state), "");
    open(&mut state, &mut dirty);
    row_click(&mut state, 0, &mut dirty);
    assert_eq!(content(&state), "alpha beta");
    open(&mut state, &mut dirty);
    row_click(&mut state, 1, &mut dirty);
    assert_eq!(content(&state), "");
    assert!(state.annotations.is_empty());
    assert!(state.undo_stack.is_empty());
}

/// 【文字菜单测试】【滚轮独占】菜单打开时滚轮不改变字号，键盘跳过禁用项与分隔线，首尾导航循环，无参数、无返回值
#[test]
fn menu_navigation_skips_disabled_items_and_wheel_does_not_change_size() {
    let mut state = prepared();
    let mut dirty = 0;
    open(&mut state, &mut dirty);
    let points = state.tool_settings.font_size;
    super::super::handle_scroll(&mut state, 0.0, 15.0, &mut dirty);
    assert_eq!(state.tool_settings.font_size, points);
    key(&mut state, "Home", &mut dirty);
    assert_eq!(state.text_context_menu.as_ref().unwrap().active, Some(0));
    key(&mut state, "Up", &mut dirty);
    assert_eq!(state.text_context_menu.as_ref().unwrap().active, Some(8));
    key(&mut state, "Return", &mut dirty);
    assert!(state.text_context_menu.is_none());
    assert!(crate::tools::text::session_input::has_selection(&state));
    assert!(state.annotations.is_empty());
}

/// 【文字菜单测试】【取消输入组】原版取消菜单后继续输入仍合并为一次撤销，不把打开菜单当作正文修改边界，无参数、无返回值
#[test]
fn canceled_menu_does_not_split_continuous_typing_history() {
    let reference: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../../tests/fixtures/qt-text-menu.json")).unwrap();
    let reference = reference
        .iter()
        .find(|case| case["name"] == "unselected")
        .unwrap();
    let mut state = prepared();
    let mut dirty = 0;
    open(&mut state, &mut dirty);
    key(&mut state, "Escape", &mut dirty);
    super::super::handle_key(&mut state, None, Some("x".into()), &mut dirty);
    let text = |state: &EditorState| {
        if let crate::types::AnnotationShape::Text { content, .. } =
            &state.text_session.as_ref().unwrap().working.shape
        {
            content.clone()
        } else {
            unreachable!()
        }
    };
    assert_eq!(
        text(&state),
        reference["post_menu_typing"]["editor_text"]
            .as_str()
            .unwrap()
    );
    key(&mut state, "Ctrl+Z", &mut dirty);
    assert_eq!(
        text(&state),
        reference["post_menu_undo"]["editor_text"].as_str().unwrap()
    );
    assert!(state.text_session.is_some());
    assert!(state.undo_stack.is_empty());
}

/// 【文字弹层测试】【过期通知】旧菜单的关闭与指针事件不能影响新菜单，当前关闭只结束菜单，不提交正文，无参数、无返回值
#[test]
fn stale_native_popup_events_do_not_close_a_new_menu() {
    use crate::types::popup::PopupEvent;
    let mut state = prepared();
    let mut dirty = 0;
    open(&mut state, &mut dirty);
    let old = state.text_context_menu.as_ref().unwrap().id;
    key(&mut state, "Escape", &mut dirty);
    open(&mut state, &mut dirty);
    let current = state.text_context_menu.as_ref().unwrap().id;
    assert_ne!(old, current);
    crate::app::text_menu_surface::event(&mut state, old, PopupEvent::Closed, &mut dirty);
    crate::app::text_menu_surface::event(
        &mut state,
        old,
        PopupEvent::Pointer { x: 20.0, y: 20.0 },
        &mut dirty,
    );
    assert_eq!(state.text_context_menu.as_ref().unwrap().id, current);
    assert!(state.text_context_menu.as_ref().unwrap().active.is_none());
    crate::app::text_menu_surface::event(&mut state, current, PopupEvent::Closed, &mut dirty);
    assert!(state.text_context_menu.is_none());
    assert!(state.text_session.is_some());
    assert!(state.annotations.is_empty());
    assert!(!state.cancel);
}

/// 【文字弹层测试】【原生动作顺序】原生菜单确认只保存待交付命令，表面关闭前不执行删除，不产生标注历史，无参数、无返回值
#[test]
fn native_menu_action_is_deferred_until_surface_close() {
    use crate::types::popup::PopupEvent;
    let mut state = prepared();
    let mut dirty = 0;
    key(&mut state, "Ctrl+A", &mut dirty);
    open(&mut state, &mut dirty);
    let menu = state.text_context_menu.as_mut().unwrap();
    menu.native = true;
    let id = menu.id;
    let row = menu.layout.rows[6];
    let (x, y) = (f64::from(row.left() + 10.0), f64::from(row.top() + 10.0));
    for pressed in [true, false] {
        crate::app::text_menu_surface::event(
            &mut state,
            id,
            PopupEvent::Button {
                x,
                y,
                button: MouseButton::Left,
                pressed,
            },
            &mut dirty,
        );
    }
    assert!(state.text_context_menu.is_none());
    assert_eq!(
        state.pending_text_menu_action,
        Some(crate::tools::text::session_input::Command::DeleteSelection)
    );
    assert!(
        matches!(&state.text_session.as_ref().unwrap().working.shape, crate::types::AnnotationShape::Text { content, .. } if content == "alpha beta")
    );
    assert!(state.undo_stack.is_empty());
}

/// 【文字菜单测试】【按钮序列隔离】右键菜单打断之前的左键双击序列，关闭后首次左键只定位光标，不选择整词，无参数、无返回值
#[test]
fn right_menu_interrupts_the_previous_left_double_click_sequence() {
    let mut state = prepared();
    let mut dirty = 0;
    for before_menu in [true, false] {
        pointer(&mut state, 315.0, 213.0, &mut dirty);
        super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        super::super::handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert!(!crate::tools::text::session_input::has_selection(&state));
        if before_menu {
            open(&mut state, &mut dirty);
            key(&mut state, "Escape", &mut dirty);
        }
    }
}
