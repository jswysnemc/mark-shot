//! 【截图区域识别】【候选输入】短点击采纳，五逻辑像素后转为手动拖选，滚轮切换包含层级

use super::{draw, motion, state::Press};
use crate::{
    editor::{EditorState, dirty::apply_damage_rects},
    features::region_detection::window::topmost,
    tools::Tool,
    types::{MouseButton, ScrollEvent},
};

/// 【截图区域识别】【选择阶段】按 state 判断是否仍处于普通截图范围选择，返回候选输入是否可用
fn selecting(state: &EditorState) -> bool {
    state.region_detection.space.is_some()
        && state.file_canvas.is_none()
        && state.selected_tool == Tool::Selection
        && state.selection.zone.is_none()
        && !state.cancel
}

/// 【截图区域识别】【悬停刷新】按 state 的当前指针更新图像或窗口候选，dirty 返回完整损坏范围
pub(crate) fn refresh(state: &mut EditorState, dirty: &mut u32) {
    if !selecting(state) {
        sync(state, dirty);
        return;
    }
    let region = &mut state.region_detection;
    #[cfg(target_os = "windows")]
    if let Some(rect) = region
        .native
        .as_mut()
        .and_then(|native| native.hover(state.input.pointer.global))
    {
        draw::apply(state, Some(rect));
        apply_damage_rects(state, dirty);
        return;
    }
    let space = region.space.unwrap();
    let point = space.point(state.input.pointer.global);
    let rect = point
        .and_then(|point| {
            let window = topmost(&region.windows, point);
            if let Some(detector) = &region.detector {
                let rect = region.cursor.hover_within(detector, point, window);
                if rect != detector.bounds() || region.cursor.manual_level() {
                    return Some(rect);
                }
            }
            window
        })
        .and_then(|rect| space.global(rect));
    draw::apply(state, rect);
    apply_damage_rects(state, dirty);
}

/// 【截图区域识别】【状态同步】按 state 清理编辑或取消后的候选和任务，dirty 返回旧框损坏，不改变正式选区
pub(crate) fn sync(state: &mut EditorState, dirty: &mut u32) {
    if selecting(state)
        || state.region_detection.pressed.is_some() && state.input.mouse_down && !state.cancel
    {
        return;
    }
    state.region_detection.pressed = None;
    state.region_detection.cursor.reset();
    state.region_detection.worker = None;
    #[cfg(target_os = "windows")]
    {
        state.region_detection.native = None;
    }
    draw::apply(state, None);
    apply_damage_rects(state, dirty);
}

/// 【截图区域识别】【指针移动】按 state 更新悬停或点击阈值，返回是否抑制短距离拖选，dirty 返回候选损坏
pub(crate) fn pointer(state: &mut EditorState, dirty: &mut u32) -> bool {
    state.region_detection.pointer_valid = true;
    if state.region_detection.pressed.is_some() {
        motion::observe(state, state.input.pointer.global);
        return state
            .region_detection
            .pressed
            .as_ref()
            .is_some_and(|press| !press.dismissed && press.candidate.is_some());
    }
    if !state.input.mouse_down {
        refresh(state, dirty);
    }
    false
}

/// 【截图区域识别】【左键采纳】按 state、button、pressed 记录候选点击或交给正式选区，dirty 返回旧候选损坏，工具仍处理交付
pub(crate) fn button(state: &mut EditorState, button: MouseButton, pressed: bool, dirty: &mut u32) {
    if !matches!(button, MouseButton::Left) {
        return;
    }
    if pressed && selecting(state) && !state.input.mouse_down {
        refresh(state, dirty);
        state.region_detection.pressed = Some(Press {
            origin: state.input.pointer.global,
            candidate: state.region_detection.hovered,
            dismissed: false,
        });
    } else if !pressed && let Some(press) = state.region_detection.pressed.take() {
        let point = state.input.pointer.global;
        if !press.dismissed
            && (point.0 - press.origin.0).hypot(point.1 - press.origin.1) < 5.0
            && let Some(candidate) = press.candidate
        {
            state.selection.zone = Some(candidate);
        }
        draw::apply(state, None);
    }
}

/// 【截图区域识别】【滚轮层级】按 state 与 event 扩大或缩小候选，返回是否消费轴事件，dirty 返回新旧框损坏
pub(crate) fn scroll(state: &mut EditorState, event: ScrollEvent, dirty: &mut u32) -> bool {
    if !selecting(state) || state.input.mouse_down {
        return false;
    }
    let region = &mut state.region_detection;
    #[cfg(target_os = "windows")]
    if let Some(rect) = region
        .native
        .as_mut()
        .and_then(|native| native.step(event.angle_y()))
    {
        draw::apply(state, Some(rect));
        apply_damage_rects(state, dirty);
        return true;
    }
    let Some(detector) = &region.detector else {
        return false;
    };
    let space = region.space.unwrap();
    let Some(point) = space.point(state.input.pointer.global) else {
        return false;
    };
    let direction = event.angle_y();
    if direction == 0.0 {
        return true;
    }
    let rect = region.cursor.step_within(
        detector,
        point,
        if direction > 0.0 { 1 } else { -1 },
        topmost(&region.windows, point),
    );
    draw::apply(state, space.global(rect));
    apply_damage_rects(state, dirty);
    true
}
