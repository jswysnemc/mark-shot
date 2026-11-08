//! 【外部动作输入测试】【会话保护】列表入口、右键隔离、键盘关闭、滚轮保护及动作交付

use super::*;
use crate::{
    features::external_actions::{Action, extensions},
    ui::{
        external_panel::{ExternalPanel, Kind, layout::Layout},
        toolbar::{Integration, ToolbarButton, ToolbarItem},
    },
};

/// 【外部动作输入测试】【受控列表】为 state 提供独立扩展列表，避免测试执行用户扩展或依赖系统应用目录
fn install(state: &mut EditorState) {
    let command =
        extensions::parse(r#"{"name":"Controlled","command":"echo data","closeOnStart":false}"#)
            .unwrap()
            .remove(0);
    state.external_panel = ExternalPanel::open(
        Kind::Extensions,
        vec![Action::Extension(command)],
        String::new(),
        Layout::new(Kind::Extensions, 1, 90.0, 36.0, (640.0, 480.0)),
        (12.0, 80.0),
        0,
    );
}

/// 【外部动作输入测试】【列表隔离】右键与滚轮不会改变选中工具或笔宽，Escape 关闭列表且保留截图，左键交付只记录请求
#[test]
fn external_list_input_never_leaks_into_canvas_or_tool_settings() {
    let mut state = editor();
    let mut dirty = 0;
    super::super::select_tool(&mut state, crate::tools::Tool::Pen, &mut dirty);
    install(&mut state);
    let zone = state.selection.zone;
    let width = state
        .tool_settings
        .stroke(crate::types::tool_settings::StrokeGroup::Freehand);
    pointer(&mut state, 50.0, 128.0, &mut dirty);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Right,
        true,
        &mut dirty,
    );
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Right,
        false,
        &mut dirty,
    );
    super::super::handle_scroll(&mut state, 0.0, 80.0, &mut dirty);
    assert_eq!(state.selected_tool, crate::tools::Tool::Pen);
    assert_eq!(
        state
            .tool_settings
            .stroke(crate::types::tool_settings::StrokeGroup::Freehand),
        width
    );
    assert!(state.external_action_requested.is_none() && state.pending.is_none());
    key(&mut state, "Escape", &mut dirty);
    assert!(state.external_panel.kind.is_none() && !state.cancel);
    assert_eq!(state.selection.zone, zone);
    install(&mut state);
    pointer(&mut state, 50.0, 128.0, &mut dirty);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        true,
        &mut dirty,
    );
    assert!(state.external_action_requested.is_none());
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        false,
        &mut dirty,
    );
    assert!(state.external_action_requested.is_some() && state.external_panel.kind.is_none());
    assert!(state.pending.is_none() && state.finish.is_none() && !state.cancel);
}

/// 【外部动作输入测试】【可用入口】副栏按实现状态开放集成动作，右键不能打开列表，重复左键关闭而不重新打开
#[test]
fn toolbar_integrations_enable_only_implemented_actions_and_repeat_click_closes() {
    let mut state = editor();
    let mut dirty = 0;
    super::super::select_tool(&mut state, crate::tools::Tool::Pen, &mut dirty);
    for (index, item) in state.action_toolbar.items.iter().enumerate() {
        if let ToolbarItem::Button(ToolbarButton::Integration(integration)) = item {
            assert_eq!(
                state.action_toolbar.disabled.contains(&index),
                !matches!(
                    integration,
                    Integration::OpenWith
                        | Integration::Extensions
                        | Integration::Upload
                        | Integration::Translate
                        | Integration::Settings
                )
            );
        }
    }
    let index = state
        .action_toolbar
        .items
        .iter()
        .position(|item| {
            *item == ToolbarItem::Button(ToolbarButton::Integration(Integration::Extensions))
        })
        .unwrap();
    let cell = state.action_toolbar.cell(index).unwrap();
    let point = (
        f64::from(state.action_toolbar.render_pos.0 + cell.left() + cell.width() / 2.0),
        f64::from(state.action_toolbar.render_pos.1 + cell.top() + cell.height() / 2.0),
    );
    install(&mut state);
    pointer(&mut state, point.0, point.1, &mut dirty);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Right,
        true,
        &mut dirty,
    );
    assert!(state.external_panel.kind.is_some());
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        true,
        &mut dirty,
    );
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Left,
        false,
        &mut dirty,
    );
    assert!(state.external_panel.kind.is_none() && state.external_action_requested.is_none());
}
