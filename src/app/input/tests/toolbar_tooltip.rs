//! 【工具提示】【输入回归】验证真实工具栏命中、有效快捷键、静止唤醒和移开恢复

use super::{editor, pointer};
use crate::{
    app::{
        event_loop::poll_timeout,
        input::{handle_pointer_button, refresh_panels},
        panels::tooltip,
    },
    editor::EditorState,
    keys::Keymap,
    tools::Tool,
    types::MouseButton,
    ui::{
        language::Language,
        toolbar::{ToolbarButton, ToolbarItem},
        tooltip::DELAY,
    },
};
use std::time::{Duration, Instant};
use tiny_skia::Rect;

/// 【工具提示】【编辑场景】无参数，返回已有选区、工具栏停止动画且背景固定的编辑器
fn scene() -> EditorState {
    // 1. 【工具提示】【布局准备】先按当前配置排版，再固定工具栏位置与截图选区
    let mut state = editor();
    state.selection.zone = Rect::from_xywh(80.0, 150.0, 480.0, 250.0);
    state.selected_tool = Tool::Pick;
    refresh_panels(&mut state, &mut 0);
    state.toolbar.user_position = Some((8.0, 8.0));
    refresh_panels(&mut state, &mut 0);
    // 2. 【工具提示】【动画稳定】排除初次淡入和位置移动，直接验证悬停计时
    for toolbar in [&mut state.toolbar, &mut state.action_toolbar] {
        toolbar.opacity = 1.0;
        toolbar.render_pos = toolbar.position;
    }
    state
}

/// 【工具提示】【按钮中心】按 state 的实际布局与 button 返回按钮中心，用于真实指针输入
fn button_center(state: &EditorState, button: ToolbarButton) -> (f64, f64) {
    let index = state
        .toolbar
        .items
        .iter()
        .position(|item| *item == ToolbarItem::Button(button))
        .unwrap();
    let cell = state.toolbar.cell(index).unwrap();
    (
        f64::from(state.toolbar.render_pos.0 + cell.left() + cell.width() / 2.0),
        f64::from(state.toolbar.render_pos.1 + cell.top() + cell.height() / 2.0),
    )
}

/// 【工具提示】【实际悬停】在 state 中找到 button 并移动至中心，返回真实输入生成的显示期限
fn hover(state: &mut EditorState, button: ToolbarButton) -> Instant {
    let point = button_center(state, button);
    pointer(state, point.0, point.1, &mut 0);
    state.toolbar_tooltip.deadline().unwrap()
}

/// 【工具提示】【出现后跟踪】无参数，工具栏在静止指针下完成显示时仍能重新等待并出现提示
#[test]
fn toolbar_appearing_under_a_stationary_pointer_starts_the_delay() {
    let mut state = scene();
    let point = button_center(&state, ToolbarButton::Tool(Tool::Arrow));
    state.toolbar.opacity = 0.0;
    pointer(&mut state, point.0, point.1, &mut 0);
    assert_eq!(state.toolbar_tooltip.deadline(), None);
    state.toolbar.opacity = 1.0;
    let now = Instant::now();
    tooltip::tick(&mut state, now, &mut 0);
    assert_eq!(state.toolbar_tooltip.deadline(), Some(now + DELAY));
    tooltip::tick(&mut state, now + DELAY, &mut 0);
    assert!(state.toolbar_tooltip.view.is_some());
}

/// 【工具提示】【静止显示】无参数，实际指针进入按钮后等待期限才显示，显示后恢复事件驱动等待
#[test]
fn actual_toolbar_hover_shows_after_delay_and_stops_waking() {
    let mut state = scene();
    let deadline = hover(&mut state, ToolbarButton::Tool(Tool::Arrow));
    assert!(poll_timeout(&state).is_some());
    tooltip::tick(&mut state, deadline - Duration::from_millis(1), &mut 0);
    assert!(state.toolbar_tooltip.view.is_none());
    tooltip::tick(&mut state, deadline, &mut 0);
    assert!(state.toolbar_tooltip.view.is_some());
    assert_eq!(state.toolbar_tooltip.deadline(), None);
    assert_eq!(poll_timeout(&state), None);
}

/// 【工具提示】【关闭行为】无参数，移开、按下和界面隐藏均取消提示与计时，点击后静止指针不会自行重开
#[test]
fn pointer_leave_click_and_hidden_ui_dismiss_the_tooltip() {
    let mut state = scene();
    let button = ToolbarButton::Tool(Tool::Arrow);
    let deadline = hover(&mut state, button);
    tooltip::tick(&mut state, deadline, &mut 0);
    pointer(&mut state, 500.0, 350.0, &mut 0);
    assert!(state.toolbar_tooltip.view.is_none() && state.toolbar_tooltip.deadline().is_none());
    let deadline = hover(&mut state, button);
    tooltip::tick(&mut state, deadline, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    tooltip::tick(&mut state, deadline + DELAY, &mut 0);
    assert!(state.toolbar_tooltip.view.is_none() && state.toolbar_tooltip.deadline().is_none());
    let deadline = hover(&mut state, button);
    state.ui_hidden = true;
    tooltip::tick(&mut state, deadline, &mut 0);
    assert!(state.toolbar_tooltip.view.is_none() && state.toolbar_tooltip.deadline().is_none());
}

/// 【工具提示】【绑定一致】无参数，自定义快捷键、清除绑定和中文名称都必须来自实际映射
#[test]
fn tooltip_names_use_localization_and_resolved_custom_bindings() {
    let map = Keymap::build(&std::collections::BTreeMap::from([
        (
            "tool_arrow".into(),
            crate::config::Binding::One("F6".into()),
        ),
        ("undo".into(), crate::config::Binding::One("".into())),
    ]));
    assert_eq!(
        tooltip::label(ToolbarButton::Tool(Tool::Arrow), Language::Chinese, &map).as_deref(),
        Some("箭头 (F6)")
    );
    assert_eq!(
        tooltip::label(ToolbarButton::Undo, Language::English, &map).as_deref(),
        Some("Undo")
    );
    assert_eq!(
        tooltip::label(ToolbarButton::Clear, Language::Chinese, &map).as_deref(),
        Some("清除全部标注")
    );
}

/// 【工具提示】【局部帧恢复】无参数，提示显示和移开后的实际局部帧必须与完整帧逐像素一致
#[cfg(target_os = "linux")]
#[test]
fn tooltip_show_and_leave_restore_the_same_pixels_as_whole_frames() {
    use crate::app::region_rendering_test_support as support;
    let mut actual = scene();
    let mut expected = scene();
    for state in [&mut actual, &mut expected] {
        state.enable_region_rendering();
        support::paint(state, None, false);
        support::finish_frame(state);
        let deadline = hover(state, ToolbarButton::Tool(Tool::Arrow));
        tooltip::tick(state, deadline, &mut 0);
    }
    let dirty = actual.monitor_dirty_rect(0);
    let frame = support::render(&mut actual, dirty, false);
    let full = support::render(&mut expected, None, false);
    support::equal(&frame, &full, "tooltip shown");
    for state in [&mut actual, &mut expected] {
        support::finish_frame(state);
        pointer(state, 500.0, 350.0, &mut 0);
    }
    let dirty = actual.monitor_dirty_rect(0);
    let frame = support::render(&mut actual, dirty, false);
    let full = support::render(&mut expected, None, false);
    support::equal(&frame, &full, "tooltip dismissed");
}
