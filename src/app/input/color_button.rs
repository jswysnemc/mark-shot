//! 【属性取色】【按钮输入】属性颜色按钮只在左键同位置释放时切换取色器

use crate::editor::EditorState;
use crate::ui::color_popover::ColorTarget;

/// 【属性取色】【按钮定位】返回 state 实际指针下的颜色目标与显示器；不把其他显示器的同坐标当作按钮
fn hit(state: &EditorState) -> Option<(ColorTarget, usize)> {
    let panel = &state.settings_panel;
    if !panel.visible || state.input.pointer.monitor_idx != panel.monitor_idx {
        return None;
    }
    let (_, index) = panel.hit_test(state.input.pointer.local);
    panel
        .widgets
        .get(index?)?
        .color_target()
        .map(|target| (target, panel.monitor_idx))
}

/// 【属性取色】【按钮按下】在 state 中保存颜色目标；返回是否命中按钮，不立即打开或关闭弹层
pub(super) fn press(state: &mut EditorState) -> bool {
    let Some((target, monitor)) = hit(state) else {
        return false;
    };
    let closing = state.color_popover.open
        && state.color_popover.radial.is_none()
        && state.color_popover.target == target;
    state.input.color_button_press = Some((target, monitor, closing));
    true
}

/// 【属性取色】【按钮释放】确认 state 中相同目标与显示器；dirty 返回变化，移出按钮或非左键打断不切换弹层
pub(super) fn release(state: &mut EditorState, dirty: &mut u32) -> bool {
    let Some((target, monitor, closing)) = state.input.color_button_press.take() else {
        return false;
    };
    if hit(state) == Some((target, monitor)) {
        if closing {
            super::super::panels::color::close_color_popover(state, dirty);
        } else {
            super::super::panels::color::toggle(state, target, dirty);
        }
        super::refresh_panels(state, dirty);
    }
    true
}
