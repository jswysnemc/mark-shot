//! 【自由轨迹测试】【真实手势】合并输入与逐点输入必须生成同样的完整笔迹及栅格

use super::*;
use crate::{app::region_rendering_test_support as support, tools::Tool, types::MouseButton};

/// 【自由轨迹测试】【按下环境】按tool构造已按下左键的真实工具状态，返回编辑器及损坏掩码
fn pressed(tool: Tool) -> (EditorState, u32) {
    let mut state = support::editor(1.0, true);
    let mut dirty = 0;
    super::super::input::select_tool(&mut state, tool, &mut dirty);
    support::pointer(
        &mut state,
        (200.0, 200.0),
        Some((MouseButton::Left, true)),
        &mut dirty,
    );
    (state, dirty)
}

/// 【自由轨迹测试】【折返轨迹】无参数，对比真实画笔逐点移动与合并后的折返移动，包含松开和完整像素，无返回值
#[test]
fn coalesced_freehand_keeps_every_corner_and_return_in_order() {
    for tool in [Tool::Pen, Tool::Highlighter] {
        let (mut sequential, mut first_dirty) = pressed(tool);
        let (mut coalesced, mut second_dirty) = pressed(tool);
        let points = [
            (230.0, 200.0),
            (250.0, 200.0),
            (220.0, 200.0),
            (220.0, 240.0),
            (270.0, 210.0),
        ];
        for point in points {
            dispatch(&mut sequential, &[], 0, point.0, point.1, &mut first_dirty);
        }
        let history: Vec<_> = points
            .into_iter()
            .map(|point| PointerMotion::new(0, point))
            .collect();
        dispatch(
            &mut coalesced,
            &history,
            0,
            points[4].0,
            points[4].1,
            &mut second_dirty,
        );
        for (state, dirty) in [
            (&mut sequential, &mut first_dirty),
            (&mut coalesced, &mut second_dirty),
        ] {
            support::pointer(state, points[4], Some((MouseButton::Left, false)), dirty);
        }
        assert_eq!(
            coalesced.annotations[0].shape,
            sequential.annotations[0].shape
        );
        support::equal(
            &support::render(&mut coalesced, None, false),
            &support::render(&mut sequential, None, false),
            "coalesced freehand pixels",
        );
    }
}
