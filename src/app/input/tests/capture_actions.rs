//! 【截图手势测试】【动作边界】验证独立双击、中键拖动和完成动作

use super::{editor, pointer};
use crate::{app::input::capture_actions, config::capture::ClickAction, types::MouseButton};

/// 【截图手势测试】【双击独立】无参数，双击复制不继承默认保存输出，关闭双击动作仍可继续编辑
#[test]
fn double_click_uses_its_own_action_and_none_does_not_finish() {
    for action in [ClickAction::Copy, ClickAction::None] {
        let mut state = editor();
        let mut dirty = 0;
        state.selection.zone = tiny_skia::Rect::from_xywh(80.0, 80.0, 400.0, 300.0);
        state.to.save = true;
        pointer(&mut state, 250.0, 200.0, &mut dirty);
        assert!(!capture_actions::button(
            &mut state,
            MouseButton::Left,
            true,
            action,
            &mut dirty
        ));
        capture_actions::button(&mut state, MouseButton::Left, false, action, &mut dirty);
        capture_actions::button(&mut state, MouseButton::Left, true, action, &mut dirty);
        if action == ClickAction::Copy {
            let finish = state.finish.unwrap();
            assert!(finish.copy && !finish.save);
        } else {
            assert!(state.finish.is_none());
        }
    }
}

/// 【截图手势测试】【拖动取消】无参数，中键超出阈值再回到原点也不能成为单击，选区外不执行动作
#[test]
fn middle_drag_cannot_turn_back_into_click() {
    let mut state = editor();
    let mut dirty = 0;
    state.selection.zone = tiny_skia::Rect::from_xywh(80.0, 80.0, 400.0, 300.0);
    pointer(&mut state, 250.0, 200.0, &mut dirty);
    capture_actions::middle_press(&mut state, ClickAction::Cancel);
    pointer(&mut state, 280.0, 230.0, &mut dirty);
    capture_actions::moved(&mut state);
    pointer(&mut state, 250.0, 200.0, &mut dirty);
    capture_actions::middle_release(&mut state, &mut dirty);
    assert!(!state.cancel);
    capture_actions::middle_press(&mut state, ClickAction::Cancel);
    capture_actions::middle_release(&mut state, &mut dirty);
    assert!(state.cancel);
}
