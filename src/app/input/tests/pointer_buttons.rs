//! 【输入测试】【按钮职责】按原版验证选择取消、编辑右键、控件隔离和中键范围切换

use super::{editor, pointer};
use crate::{
    app::input::{handle_pointer_button, refresh_panels, select_tool},
    tools::Tool,
    types::MouseButton,
    ui::{
        panel::UiPanel,
        toolbar::{ToolbarButton, ToolbarItem},
    },
};

/// 【输入测试】【右键退出】没有确认选区时右键关闭会话，不产生导出
#[test]
fn right_click_cancels_the_selection_phase() {
    let mut state = editor();
    let mut dirty = 0;
    pointer(&mut state, 200.0, 200.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    assert!(state.cancel && state.finish.is_none());
}

/// 【输入测试】【拖选取消】初次选区拖动已有临时矩形，但仍属于选择阶段，右键不能误进入编辑
#[test]
fn right_click_during_initial_drag_still_cancels_the_session() {
    let mut state = editor();
    let mut dirty = 0;
    pointer(&mut state, 100.0, 100.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 220.0, 220.0, &mut dirty);
    assert!(state.selection.zone.is_some());
    handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    assert!(state.cancel);
}

/// 【输入测试】【编辑右键】编辑阶段保留选区，切回标注选择，第二次右键才打开颜色面板
#[test]
fn editing_right_click_preserves_region_and_double_click_opens_palette() {
    let mut state = editor();
    let mut dirty = 0;
    select_tool(&mut state, Tool::Pen, &mut dirty);
    let region = state.selection.zone;
    pointer(&mut state, 300.0, 220.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    assert_eq!(state.selected_tool, Tool::Pick);
    assert_eq!(state.selection.zone, region);
    assert!(!state.cancel && !state.color_popover.open);
    handle_pointer_button(&mut state, MouseButton::Right, false, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    assert!(state.color_popover.open);
}

/// 【输入测试】【按钮隔离】右键点击副栏复制按钮不能导出、切换工具或删除选区
#[test]
fn right_click_on_action_button_does_not_activate_or_leak_to_canvas() {
    let mut state = editor();
    let mut dirty = 0;
    select_tool(&mut state, Tool::Pen, &mut dirty);
    let region = state.selection.zone;
    let index = state
        .action_toolbar
        .items
        .iter()
        .position(|item| {
            matches!(
                item,
                ToolbarItem::Button(ToolbarButton::Finish(crate::types::Finish::Copy))
            )
        })
        .unwrap();
    let panel = &state.action_toolbar;
    let cell = panel.cell(index).unwrap();
    let point = (
        f64::from(panel.render_pos.0 + cell.left() + cell.width() / 2.0),
        f64::from(panel.render_pos.1 + cell.top() + cell.height() / 2.0),
    );
    pointer(&mut state, point.0, point.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Right, true, &mut dirty);
    assert_eq!(state.selected_tool, Tool::Pen);
    assert_eq!(state.selection.zone, region);
    assert!(state.finish.is_none());
}

/// 【输入测试】【中键全图】中键从选择阶段进入全图，范围按钮能够回到没有选区的状态
#[test]
fn middle_click_enters_fullscreen_and_scope_can_return_to_selection() {
    let mut state = editor();
    let mut dirty = 0;
    pointer(&mut state, 300.0, 230.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Middle, true, &mut dirty);
    assert!(state.fullscreen_scope && state.selection.zone.is_some());
    super::super::pointer_policy::toggle_scope(&mut state, &mut dirty);
    assert!(!state.fullscreen_scope && state.selection.zone.is_none());
    assert!(state.toolbar.rect().is_none());
}

/// 【输入测试】【快捷键兼容】原版未展示的辅助工具仍可以通过快捷键入口选择，不要求主栏拥有按钮
#[test]
fn keyboard_only_tools_remain_available() {
    let mut state = editor();
    let mut dirty = 0;
    select_tool(&mut state, Tool::NumeratedArrow, &mut dirty);
    assert_eq!(state.selected_tool, Tool::NumeratedArrow);
    select_tool(&mut state, Tool::Eyedropper, &mut dirty);
    assert_eq!(state.selected_tool, Tool::Eyedropper);
    refresh_panels(&mut state, &mut dirty);
}

/// 【输入测试】【点击确认】按钮只在左键原位释放时执行，按下后移出按钮不能提前复制
#[test]
fn toolbar_buttons_require_left_release_on_the_same_button() {
    let mut state = editor();
    let mut dirty = 0;
    select_tool(&mut state, Tool::Pen, &mut dirty);
    let index = state
        .action_toolbar
        .items
        .iter()
        .position(|item| {
            matches!(
                item,
                ToolbarItem::Button(ToolbarButton::Finish(crate::types::Finish::Copy))
            )
        })
        .unwrap();
    let panel = &state.action_toolbar;
    let cell = panel.cell(index).unwrap();
    let point = (
        f64::from(panel.render_pos.0 + cell.left() + cell.width() / 2.0),
        f64::from(panel.render_pos.1 + cell.top() + cell.height() / 2.0),
    );
    pointer(&mut state, point.0, point.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert!(state.finish.is_none());
    pointer(&mut state, 300.0, 220.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(state.finish.is_none());
    pointer(&mut state, point.0, point.1, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    assert!(state.finish.is_some_and(|outputs| outputs.copy));
}

/// 【输入测试】【多屏隔离】显示器 A 的属性面板不能接收显示器 B 相同本地坐标的输入
#[test]
fn panels_on_another_monitor_are_not_hit() {
    let mut state = editor();
    let mut dirty = 0;
    select_tool(&mut state, Tool::Pen, &mut dirty);
    let panel = &state.settings_panel;
    let point = (
        f64::from(panel.render_pos.0 + 20.0),
        f64::from(panel.render_pos.1 + panel.size.1 / 2.0),
    );
    state.input.pointer.monitor_idx = 1;
    state.input.pointer.local = point;
    assert!(matches!(
        super::super::ui_hit::hit_test_ui(&state, point),
        super::super::ui_hit::UiHit::None
    ));
}
