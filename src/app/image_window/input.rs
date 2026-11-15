//! 【图片窗口】【输入归属】合并工具栏空白拖动交给窗口管理器，工具按钮和画布维持各自输入

use crate::{
    editor::EditorState,
    types::{MouseButton, image_window::Action},
    ui::image_window::{self, Hit},
};

/// 【图片窗口】【窗口尺寸】按 state 返回唯一图片窗口的逻辑尺寸
fn size(state: &EditorState) -> (u32, u32) {
    (
        state.placements[0].size.0 as u32,
        state.placements[0].size.1 as u32,
    )
}

/// 【图片窗口】【合并栏命中】按state返回窗口操作，工具按钮内部交给工具输入处理
pub(crate) fn hit(state: &EditorState) -> Option<Hit> {
    let point = state.input.pointer.local;
    let hit = image_window::layout::hit(
        size(state),
        image_window::layout::bar_height(state.toolbar.size.1),
        point,
    );
    if hit == Some(Hit::Move) && state.toolbar.hit_test(point).1.is_some() {
        None
    } else {
        hit
    }
}

/// 【图片窗口】【指针反馈】按 state 更新窗口按钮悬停，dirty 返回重绘；返回是否阻止画布处理
pub(crate) fn pointer(state: &mut EditorState, dirty: &mut u32) -> bool {
    if !state.image_window.enabled {
        return false;
    }
    let hit = hit(state);
    if state.image_window.hovered != hit {
        state.image_window.hovered = hit;
        crate::editor::dirty::mark_all_dirty(dirty, 1);
        state.damage_local(0, state.placements[0].rect().unwrap());
    }
    !canvas_gesture(state) && (hit.is_some() || state.image_window.pressed.is_some())
}

/// 【图片窗口】【画布持有】按 state 判断是否有从画布开始的活动手势，窗口边框不能中断其移动或释放
fn canvas_gesture(state: &EditorState) -> bool {
    state.input.mouse_down
        || state.input.left_canvas
        || state.tool_active
        || state.ann_drag.is_some()
        || state.navigation.pan.is_some()
        || state.navigation.slider.is_some()
        || state
            .text_session
            .as_ref()
            .is_some_and(|session| session.dragging)
}

/// 【图片窗口】【系统操作】按 state、button 和 pressed 设置待执行动作，dirty 返回重绘，返回是否消费本次按钮
pub(crate) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if !state.image_window.enabled {
        return false;
    }
    if canvas_gesture(state) {
        return false;
    }
    let mut hit = hit(state);
    if hit.is_none() && state.toolbar.hit_test(state.input.pointer.local) == (true, None) {
        hit = Some(Hit::Move);
    }
    if button != MouseButton::Left {
        return pressed && hit.is_some();
    }
    if !pressed && let Some(owner) = state.image_window.pressed.take() {
        if Some(owner) == hit {
            state.image_window.action = match owner {
                Hit::Close => {
                    state.cancel = true;
                    None
                }
                Hit::Minimize => Some(Action::Minimize),
                Hit::Maximize => Some(Action::ToggleMaximize),
                _ => None,
            };
        }
        crate::editor::dirty::mark_all_dirty(dirty, 1);
        return true;
    }
    if pressed && let Some(hit) = hit {
        state.image_window.pressed = Some(hit);
        crate::editor::dirty::mark_all_dirty(dirty, 1);
        state.image_window.action = match hit {
            Hit::Move => Some(Action::Move),
            Hit::Resize(edge) => Some(Action::Resize(edge)),
            _ => None,
        };
        return true;
    }
    false
}
