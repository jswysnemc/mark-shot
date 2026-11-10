//! 【工具栏】【屏幕停靠】固定边缘布局、双栏避让和会话内方向切换

use crate::{
    config::capture::ToolbarPosition as Position, editor::EditorState, ui::toolbar::Orientation,
};

pub(super) use crate::ui::toolbar::docking::positions;

/// 【工具栏】【停靠同步】按 state 和 position 应用配置变化，返回当前会话实际停靠边；图片窗口保持自身布局
pub(super) fn prepare(state: &mut EditorState, position: Position) -> Position {
    let configured = if state.file_canvas.is_some() {
        Position::Selection
    } else {
        position
    };
    if state.toolbar.docking != configured {
        state.toolbar.docking = configured;
        state.toolbar.dock_override = None;
        state.toolbar.dock_monitor = None;
        state.toolbar.user_position = None;
        state.action_toolbar.user_position = None;
        state.toolbar.monitor_idx = state.input.pointer.monitor_idx;
    }
    let actual = current(state);
    if actual != Position::Selection {
        if state.toolbar.dock_monitor.is_none() {
            state.toolbar.monitor_idx = state.input.pointer.monitor_idx;
            if state.selection.zone.is_some() && !state.tool_active {
                state.toolbar.dock_monitor = Some(state.toolbar.monitor_idx);
            }
        }
        let orientation = if matches!(actual, Position::Left | Position::Right) {
            Orientation::Vertical
        } else {
            Orientation::Horizontal
        };
        state.toolbar.orientation = orientation;
        state.action_toolbar.orientation = orientation;
    }
    actual
}

/// 【工具栏】【当前边缘】按 state 返回会话覆盖后的停靠位置，不读取或修改文件
pub(crate) fn current(state: &EditorState) -> Position {
    if state.file_canvas.is_some() {
        Position::Selection
    } else {
        state.toolbar.dock_override.unwrap_or(state.toolbar.docking)
    }
}

/// 【工具栏】【排列切换】按 state 在横向边缘和左右侧之间切换，返回是否使用停靠模式；保留配置的左侧方向
pub(crate) fn toggle(state: &mut EditorState) -> bool {
    state.toolbar.dock_override = match current(state) {
        Position::Selection => return false,
        Position::Left | Position::Right => Some(Position::Bottom),
        Position::Top | Position::Bottom => Some(if state.toolbar.docking == Position::Left {
            Position::Left
        } else {
            Position::Right
        }),
    };
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【工具栏测试】【停靠边界】无参数，横竖屏与错行布局均在屏幕内，两栏不重叠
    #[test]
    fn docked_bars_fit_and_do_not_overlap() {
        for (edge, bounds, main, side) in [
            (Position::Top, (1066.0, 666.0), (830.0, 52.0), (510.0, 46.0)),
            (
                Position::Bottom,
                (640.0, 480.0),
                (420.0, 84.0),
                (280.0, 40.0),
            ),
            (Position::Top, (1280.0, 800.0), (600.0, 44.0), (300.0, 40.0)),
            (
                Position::Right,
                (640.0, 480.0),
                (84.0, 400.0),
                (40.0, 280.0),
            ),
            (
                Position::Right,
                (900.0, 1200.0),
                (44.0, 600.0),
                (40.0, 300.0),
            ),
            (Position::Left, (640.0, 480.0), (84.0, 400.0), (40.0, 280.0)),
            (
                Position::Left,
                (900.0, 1200.0),
                (44.0, 600.0),
                (40.0, 300.0),
            ),
        ] {
            let (a, b) = positions(edge, bounds, main, Some(side));
            let rect = |p: (f32, f32), size: (f32, f32)| {
                tiny_skia::Rect::from_xywh(p.0, p.1, size.0, size.1).unwrap()
            };
            let a = rect(a, main);
            let b = rect(b, side);
            for r in [a, b] {
                assert!(r.left() >= 8.0 && r.top() >= 8.0);
                assert!(r.right() <= bounds.0 - 8.0 && r.bottom() <= bounds.1 - 8.0);
            }
            assert!(!crate::utils::rects_overlap(&a, &b));
        }
    }

    /// 【工具栏测试】【会话切换】无参数，停靠排列切换只覆盖本次会话，配置变化清除临时覆盖
    #[test]
    fn orientation_toggle_keeps_a_docked_edge_until_config_changes() {
        let mut state = crate::app::input::tests::editor();
        prepare(&mut state, Position::Top);
        assert!(toggle(&mut state));
        assert_eq!(prepare(&mut state, Position::Top), Position::Right);
        assert!(toggle(&mut state));
        assert_eq!(prepare(&mut state, Position::Top), Position::Bottom);
        prepare(&mut state, Position::Selection);
        assert!(!toggle(&mut state));
        prepare(&mut state, Position::Left);
        assert_eq!(state.toolbar.orientation, Orientation::Vertical);
        assert!(toggle(&mut state));
        assert_eq!(prepare(&mut state, Position::Left), Position::Bottom);
        assert!(toggle(&mut state));
        assert_eq!(prepare(&mut state, Position::Left), Position::Left);
    }

    /// 【工具栏测试】【跨屏停靠】无参数，首次完成选区后锁定屏幕，指针跨屏不移动工具栏
    #[test]
    fn completed_selection_locks_dock_to_its_monitor() {
        let mut state = crate::app::input::tests::editor();
        prepare(&mut state, Position::Top);
        state.input.pointer.monitor_idx = 1;
        state.selection.zone = tiny_skia::Rect::from_xywh(680.0, 80.0, 300.0, 200.0);
        state.tool_active = true;
        prepare(&mut state, Position::Top);
        assert_eq!(state.toolbar.dock_monitor, None);
        state.tool_active = false;
        prepare(&mut state, Position::Top);
        assert_eq!(state.toolbar.dock_monitor, Some(1));
        state.input.pointer.monitor_idx = 0;
        prepare(&mut state, Position::Top);
        assert_eq!(state.toolbar.monitor_idx, 1);
        assert!(toggle(&mut state));
        prepare(&mut state, Position::Top);
        assert_eq!(state.toolbar.monitor_idx, 1);
    }

    /// 【工具栏测试】【侧栏避让】无参数，左右停靠下最大字号属性栏按剩余空间换行，避免覆盖主栏与侧栏
    #[test]
    fn side_docked_properties_wrap_inside_remaining_space() {
        use crate::ui::{panel::UiPanel, settings_panel, text_properties::layout::TextLayout};
        for edge in [Position::Left, Position::Right] {
            let mut state = crate::app::input::tests::editor();
            prepare(&mut state, edge);
            state.toolbar.size = (84.0, 400.0);
            state.action_toolbar.size = (40.0, 280.0);
            let (main, side) = positions(
                edge,
                (640.0, 480.0),
                state.toolbar.size,
                Some(state.action_toolbar.size),
            );
            state.toolbar.render_pos = main;
            state.action_toolbar.render_pos = side;
            state.action_toolbar.opacity = 1.0;
            let available = settings_panel::available_width(&state);
            let text = TextLayout::new(56.0, 24.0, 48.0, true).fit(available);
            assert!(text.size.0 <= available);
            state.settings_panel.size = text.size;
            let (point, _) = settings_panel::compute_settings_placement(&state);
            let panel =
                tiny_skia::Rect::from_xywh(point.0, point.1, text.size.0, text.size.1).unwrap();
            assert!(panel.left() >= 8.0 && panel.right() <= 632.0);
            assert!(!crate::utils::rects_overlap(
                &panel,
                &state.toolbar.rect().unwrap()
            ));
            assert!(!crate::utils::rects_overlap(
                &panel,
                &state.action_toolbar.rect().unwrap()
            ));
        }
    }
}
