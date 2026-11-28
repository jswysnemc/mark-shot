//! 【多选区截图】【编辑交接】只有已确认的遮罩图能进入图片编辑器，编辑不触发截图交付

use super::bar::Bar;
use crate::{
    editor::EditorState,
    features::multi_capture::{Session, export},
    types::MouseButton,
};
use tiny_skia::Pixmap;

pub(crate) struct State {
    pub(in crate::app) regions: Session,
    pub(super) pressed_on: Option<usize>,
    image: Option<Pixmap>,
}

impl State {
    /// 【多选区截图】【编辑状态】按 regions 建立选择阶段，未按下按钮且没有待交接原图
    pub(super) fn new(regions: Session) -> Self {
        Self {
            regions,
            pressed_on: None,
            image: None,
        }
    }

    /// 【多选区截图】【编辑资格】返回当前选区是否非空且已结束拖动，不依赖输出目标
    pub(in crate::app) fn ready(&self) -> bool {
        self.regions.operation().is_none() && !self.regions.confirmed.is_empty()
    }

    /// 【多选区截图】【中断手势】清除按钮按下或拖选草稿，返回是否需要恢复窗口外观
    pub(super) fn cancel_gesture(&mut self) -> bool {
        let pressed = self.pressed_on.take().is_some();
        self.regions.cancel_drag() || pressed
    }
}

/// 【多选区截图】【准备原图】从 state 的冻结截图按并集裁切，保留完整物理分辨率；失败时停留在选区阶段
pub(super) fn request(state: &mut EditorState) {
    let Some(selection) = state
        .multi_capture
        .as_mut()
        .filter(|selection| selection.ready() && selection.image.is_none())
    else {
        return;
    };
    selection.image = export::compose(
        &state.captures,
        &state.placements,
        &selection.regions.confirmed,
    )
    .map(|(image, _, _)| image);
}

/// 【多选区截图】【取走原图】返回 state 中只交接一次的已遮罩原图；没有编辑请求时返回空值
pub(in crate::app) fn take_image(state: &mut EditorState) -> Option<Pixmap> {
    state.multi_capture.as_mut()?.image.take()
}

/// 【多选区截图】【入口命中】按 state 当前指针返回是否命中可用的编辑按钮，不拦截正在拖选的手势
pub(in crate::app) fn edit_hit(state: &EditorState) -> bool {
    let pointer = &state.input.pointer;
    state.multi_capture.as_ref().is_some_and(State::ready)
        && state
            .placements
            .get(pointer.monitor_idx)
            .and_then(|placement| Bar::new(placement.rect()?))
            .is_some_and(|bar| crate::ui::selection_size::state::contains(bar.edit, pointer.local))
}

/// 【多选区截图】【按钮确认】按 button 与 pressed 处理同屏按下和松开，dirty 登记按钮变化；返回是否消费本次事件
pub(super) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if button != MouseButton::Left {
        return state
            .multi_capture
            .as_ref()
            .is_some_and(|selection| selection.pressed_on.is_some());
    }
    let inside = edit_hit(state);
    let monitor = state.input.pointer.monitor_idx;
    let Some(selection) = state.multi_capture.as_mut() else {
        return false;
    };
    let owner = if pressed && inside {
        selection.pressed_on = Some(monitor);
        monitor
    } else if !pressed && let Some(owner) = selection.pressed_on.take() {
        if inside && owner == monitor {
            request(state);
        }
        owner
    } else {
        return false;
    };
    if let Some(bar) = state.placements[owner].rect().and_then(Bar::new) {
        state.damage_local(owner, bar.edit);
        crate::editor::dirty::mark_dirty(dirty, owner);
    }
    true
}

/// 【多选区截图】【按钮刷新】拖选改变可编辑状态时，按各屏本地布局登记按钮损坏；dirty 返回对应输出
pub(super) fn damage_buttons(state: &mut EditorState, dirty: &mut u32) {
    for monitor in 0..state.placements.len() {
        if let Some(bar) = state.placements[monitor].rect().and_then(Bar::new) {
            state.damage_local(monitor, bar.edit);
            crate::editor::dirty::mark_dirty(dirty, monitor);
        }
    }
}
