//! 【交互回归】【工具行为】通过真实按钮和滚轮路由检查文字编辑及刚完成对象

use super::{editor, key, pointer};
use crate::{
    app::input::{handle_axis_scroll, handle_key, handle_pointer_button, select_tool},
    tools::Tool,
    types::{AnnotationShape, MouseButton, ScrollEvent},
};
use cosmic_text::Edit;

/// 【设置回归】【会话生效】无参数，真实编辑器应用输出、放大预览和工具配置时保留原图与历史
#[test]
fn preference_changes_apply_without_replacing_the_capture() {
    let mut state = text_editor();
    let before = crate::config::Config::default();
    let mut after = before.clone();
    after.general.to = crate::types::Outputs::parse("s").unwrap();
    after.startup.selection_loupe = Some(false);
    after.startup.normal_tool = Some("pen".into());
    let annotations = state.annotations.clone();
    crate::features::preferences::runtime::apply(&mut state, &before, &after, &mut 0);
    assert_eq!(state.to, after.general.to);
    assert!(state.magnifier.hidden);
    assert_eq!(state.after_selection_tool, Tool::Pen);
    assert_eq!(state.annotations, annotations);
}

/// 【设置回归】【编辑保留】设置快捷键只请求窗口，保持原件、选区、标注和撤销记录，无参数、无返回值
#[test]
fn settings_request_preserves_the_current_capture() {
    let mut state = text_editor();
    let before = state.annotations.clone();
    let zone = state.selection.zone;
    let history = state.undo_stack.len();
    key(&mut state, "Ctrl+,", &mut 0);
    assert!(state.preferences_requested);
    assert!(!state.cancel && state.finish.is_none());
    assert_eq!(state.annotations, before);
    assert_eq!(state.selection.zone, zone);
    assert_eq!(state.undo_stack.len(), history);
}

/// 【交互回归】【画布点击】在 state 的 point 发送完整左键，返回值为空
fn click(state: &mut crate::editor::EditorState, point: (f64, f64)) {
    pointer(state, point.0, point.1, &mut 0);
    handle_pointer_button(state, MouseButton::Left, true, &mut 0);
    handle_pointer_button(state, MouseButton::Left, false, &mut 0);
}

/// 【交互回归】【文字创建】返回包含一段已提交文字的编辑器，字号固定并隐藏外部配置影响
fn text_editor() -> crate::editor::EditorState {
    let mut state = editor();
    state.tool_settings.font_size = 20.0;
    select_tool(&mut state, Tool::Text, &mut 0);
    click(&mut state, (160.0, 170.0));
    handle_key(&mut state, None, Some("abcdef".into()), &mut 0);
    key(&mut state, "Escape", &mut 0);
    state
}

/// 【交互回归】【双击插入】双击显示光标，再次点击正文重新定位；直接输入扩展外框，撤销保留原件
#[test]
fn pick_double_click_opens_caret_and_next_click_positions_it() {
    let mut state = text_editor();
    let original = state.annotations[0].clone();
    select_tool(&mut state, Tool::Pick, &mut 0);
    click(&mut state, (180.0, 185.0));
    assert!(state.text_session.is_none());
    click(&mut state, (180.0, 185.0));
    assert!(state.text_session.is_some());
    assert!(state.ann_drag.is_none());
    let preview = crate::editor::text_session::PREVIEW_ID;
    assert_eq!(state.text.editors[&preview].cursor().index, 0);
    click(&mut state, (195.0, 185.0));
    assert!(state.text.editors[&preview].cursor().index > 0);
    handle_key(&mut state, None, Some("INSERTED LONG TEXT ".into()), &mut 0);
    let working = &state.text_session.as_ref().unwrap().working;
    assert!(working.bbox.width() > original.bbox.width() * 2.0);
    assert_eq!(state.annotations[0], original);
    key(&mut state, "Escape", &mut 0);
    assert!(
        matches!(&state.annotations[0].shape, AnnotationShape::Text { content, .. } if content.contains("INSERTED LONG TEXT"))
    );
    state.undo(&mut 0);
    assert_eq!(state.annotations[0], original);
}

/// 【交互回归】【完成后滚轮】绘制矩形后滚轮调整该对象而非仅修改默认粗细，一次撤销恢复原图形
#[test]
fn wheel_after_drawing_changes_the_completed_annotation() {
    let mut state = editor();
    select_tool(&mut state, Tool::Rectangle, &mut 0);
    pointer(&mut state, 150.0, 150.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 300.0, 250.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    let before = state.annotations[0].clone();
    handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut 0);
    assert_eq!(state.selected_annotation_ids(), vec![before.id]);
    assert_ne!(state.annotations[0].stroke_width, before.stroke_width);
    state.undo(&mut 0);
    assert_eq!(state.annotations[0], before);
}

/// 【交互回归】【文字滚轮】已提交文字仍通过画布滚轮调整字号，不能缩放视图
#[test]
fn wheel_after_text_commit_changes_font_size() {
    let mut state = text_editor();
    let before = state.annotations[0].clone();
    pointer(&mut state, 300.0, 250.0, &mut 0);
    let transform = state.canvas_transform();
    handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut 0);
    assert!(
        matches!(&state.annotations[0].shape, AnnotationShape::Text { font_size, .. } if *font_size != 20.0)
    );
    assert_eq!(state.canvas_transform(), transform);
    state.commit_text_size_wheel();
    state.undo(&mut 0);
    assert_eq!(state.annotations[0], before);
}

/// 【交互回归】【全部清除】点击工具栏垃圾桶清除全部标注及文字编辑，撤销恢复全部已提交内容
#[test]
fn trash_clears_all_annotations_and_active_text_then_undo_restores_them() {
    let mut state = text_editor();
    let mut second = state.annotations[0].clone();
    second.id += 10;
    state.annotations.push(second);
    let first = state.annotations[0].id;
    crate::editor::text_session::begin_existing(&mut state, first, &mut 0);
    handle_key(&mut state, None, Some("updated ".into()), &mut 0);
    super::super::refresh_panels(&mut state, &mut 0);
    state.toolbar.opacity = 1.0;
    state.toolbar.render_pos = state.toolbar.position;
    let index = state
        .toolbar
        .items
        .iter()
        .position(|item| {
            matches!(
                item,
                crate::ui::toolbar::ToolbarItem::Button(crate::ui::toolbar::ToolbarButton::Clear)
            )
        })
        .unwrap();
    assert!(!state.toolbar.disabled.contains(&index));
    let rect = state.toolbar.cell(index).unwrap();
    let point = (
        f64::from(state.toolbar.render_pos.0 + rect.left() + rect.width() / 2.0),
        f64::from(state.toolbar.render_pos.1 + rect.top() + rect.height() / 2.0),
    );
    click(&mut state, point);
    assert!(
        state.annotations.is_empty() && state.text_session.is_none() && state.pending.is_none()
    );
    state.undo(&mut 0);
    assert_eq!(state.annotations.len(), 2);
    assert!(
        matches!(&state.annotations[0].shape, AnnotationShape::Text {content, ..} if content.starts_with("updated "))
    );
}

/// 【交互回归】【编辑中字号】双击重编辑后属性面板隐藏也能用滚轮调整文字；图片入口的新草稿同样优先调整字号
#[test]
fn wheel_changes_active_text_even_when_properties_are_hidden() {
    for existing in [false, true] {
        let mut state = text_editor();
        let source = tiny_skia::Pixmap::new(640, 480).unwrap();
        state.file_canvas =
            crate::editor::file_canvas::FileCanvas::new(source, state.selection.zone.unwrap());
        if existing {
            select_tool(&mut state, Tool::Pick, &mut 0);
            click(&mut state, (180.0, 185.0));
            click(&mut state, (180.0, 185.0));
            assert!(!state.settings_panel.visible);
        } else {
            click(&mut state, (340.0, 250.0));
        }
        let before = crate::editor::text_edits::current_points(&state);
        let transform = state.canvas_transform();
        handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut 0);
        assert_ne!(crate::editor::text_edits::current_points(&state), before);
        assert_eq!(state.canvas_transform(), transform);
    }
}
