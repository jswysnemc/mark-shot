//! 【工具栏回归】【多屏边界】覆盖全屏选区、异尺寸显示器和重新排版后的可达性

use super::super::update_toolbar;
use crate::{
    app::input::tests::editor, editor::EditorState, types::Placement, ui::toolbar::Toolbar,
};

/// 【工具栏回归】【显示布局】按 screens 构造显示器位置和尺寸，返回不分配大屏像素的编辑器
fn screens(screens: &[(i32, i32, i32, i32)]) -> EditorState {
    let mut state = editor();
    state.placements = screens
        .iter()
        .map(|&(x, y, width, height)| Placement {
            position: (x, y),
            size: (width, height),
            scale: 1.0,
        })
        .collect();
    state
}

/// 【工具栏回归】【可见断言】按 toolbar 和 state 检查整个工具栏位于所属显示器的八像素边距内
fn assert_visible(toolbar: &Toolbar, state: &EditorState) {
    let bounds = state.placements[toolbar.monitor_idx].size;
    assert!(toolbar.position.0 >= 8.0 && toolbar.position.1 >= 8.0);
    assert!(toolbar.position.0 + toolbar.size.0 <= bounds.0 as f32 - 8.0);
    assert!(toolbar.position.1 + toolbar.size.1 <= bounds.1 as f32 - 8.0);
}

/// 【工具栏回归】【全屏选区】无参数，三屏全选与单屏全选时，两栏始终限制在指针所在显示器
#[test]
fn full_selection_keeps_both_bars_inside_each_of_three_outputs() {
    let mut state = screens(&[
        (-1920, 0, 1920, 1080),
        (0, 0, 1920, 1080),
        (1920, 0, 1600, 2560),
    ]);
    for monitor in 0..3 {
        state.input.pointer.monitor_idx = monitor;
        let placement = &state.placements[monitor];
        let local = tiny_skia::Rect::from_xywh(
            placement.position.0 as f32,
            0.0,
            placement.size.0 as f32,
            placement.size.1 as f32,
        );
        for region in [
            local,
            tiny_skia::Rect::from_xywh(-1920.0, 0.0, 5440.0, 2560.0),
        ] {
            state.selection.zone = region;
            update_toolbar(&mut state, &mut 0);
            assert_eq!(state.toolbar.monitor_idx, monitor);
            assert_eq!(state.action_toolbar.monitor_idx, monitor);
            assert_visible(&state.toolbar, &state);
            assert_visible(&state.action_toolbar, &state);
        }
    }
}

/// 【工具栏回归】【异屏排版】无参数，副栏位于小屏时按自己的显示器换行，手动位置随尺寸变化重新收敛
#[test]
fn moved_bars_reflow_and_clamp_against_their_own_outputs() {
    let mut state = screens(&[(0, 0, 1920, 1080), (1920, 0, 280, 320)]);
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 1920.0, 1080.0);
    update_toolbar(&mut state, &mut 0);
    state.toolbar.user_position = Some((1900.0, 1060.0));
    state.action_toolbar.monitor_idx = 1;
    state.action_toolbar.user_position = Some((260.0, 300.0));
    state.action_toolbar.orientation = crate::ui::toolbar::Orientation::Horizontal;
    update_toolbar(&mut state, &mut 0);
    assert_visible(&state.toolbar, &state);
    assert_visible(&state.action_toolbar, &state);
    assert_eq!(state.action_toolbar.monitor_idx, 1);
    assert!(state.action_toolbar.size.0 <= 264.0);
}
