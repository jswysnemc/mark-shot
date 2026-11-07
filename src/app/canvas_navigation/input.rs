//! 【画布导航】【输入分派】屏幕控件优先，画布滚轮、中键和Ctrl复位共用独立视图

use crate::{
    editor::{
        EditorState,
        canvas_navigation::state::{Pan, SliderDrag},
    },
    features::canvas_navigation::policy::{WheelTarget, wheel_factor},
    types::{MouseButton, PointerState, ScrollEvent},
    ui::canvas_navigation::geometry::{hit, parts},
};
use std::time::{Duration, Instant};

/// 【画布导航】【控件保护】按state返回是否有优先接收输入的屏幕控件或弹层
fn over_ui(state: &EditorState) -> bool {
    state.text_context_menu.is_some()
        || state.external_panel.kind.is_some()
        || state.selection_translation.busy()
        || state.upload.busy()
        || state.selection_translation.popup.is_some()
        || state.selection_size.interacting()
        || crate::app::selection_size::input::hit(state).is_some()
        || !matches!(
            crate::app::input::ui_hit::hit_test_ui(state, state.input.pointer.local),
            crate::app::input::ui_hit::UiHit::None
        )
}

/// 【画布导航】【滚动条命中】无额外参数，按state返回当前指针命中的轴，水平轴为true
fn bar_hit(state: &EditorState) -> Option<bool> {
    let point = state.navigation_point();
    let bars = state.navigation.bars;
    if hit(bars.horizontal, point) {
        Some(true)
    } else if hit(bars.vertical, point) {
        Some(false)
    } else {
        None
    }
}

/// 【画布导航】【滚动条同步】按state同步几何、显隐和悬停，dirty返回实际受影响输出，无返回值
fn sync_bars(state: &mut EditorState, dirty: &mut u32) {
    let Some(view) = state.navigation.view else {
        return;
    };
    let point = state.navigation_point();
    let available = state.navigation_policy().available();
    let dragging = (
        state.navigation.slider.is_some_and(|drag| drag.horizontal),
        state.navigation.slider.is_some_and(|drag| !drag.horizontal),
    );
    let old = state.navigation.bars;
    let changed = state
        .navigation
        .bars
        .update(view, available, point, dragging);
    let hover = bar_hit(state);
    let hover_changed = state.navigation.bar_hover != hover;
    state.navigation.bar_hover = hover;
    if changed || hover_changed || hover.is_some() {
        let window = state.navigation.window.unwrap();
        for axis in [
            old.horizontal,
            old.vertical,
            state.navigation.bars.horizontal,
            state.navigation.bars.vertical,
        ]
        .into_iter()
        .filter(|axis| axis.visible)
        {
            if let Some(rect) = axis.rect.raster().and_then(|rect| {
                tiny_skia::Rect::from_xywh(
                    rect.x() + window.left(),
                    rect.y() + window.top(),
                    rect.width(),
                    rect.height(),
                )
            }) {
                // 1. 【画布滚动条】【控件损坏】控件已在屏幕坐标内，不再套用画布变换
                for monitor in 0..state.placements.len() {
                    let offset = state.placements[monitor].offset();
                    if let Some(local) = tiny_skia::Rect::from_xywh(
                        rect.x() - offset.0,
                        rect.y() - offset.1,
                        rect.width(),
                        rect.height(),
                    )
                    .and_then(|rect| rect.intersect(&state.placements[monitor].rect()?))
                    {
                        state.damage_local(monitor, local);
                        crate::editor::dirty::mark_dirty(dirty, monitor);
                    }
                }
            }
        }
    }
}

/// 【画布导航】【指针入口】按monitor、x、y更新屏幕指针，先执行平移或滚动条，再调用原画布分派；dirty返回重绘输出
pub(crate) fn handle_pointer_move(
    state: &mut EditorState,
    monitor: usize,
    x: f64,
    y: f64,
    dirty: &mut u32,
) {
    let offset = state.placements[monitor].position;
    let global = (f64::from(offset.0) + x, f64::from(offset.1) + y);
    let (monitor, lx, ly) =
        crate::utils::global_point_to_local(&state.placements, global, monitor, (x, y));
    state.input.pointer = PointerState::new(monitor, (lx, ly), global);
    super::preview::follow(state, dirty);
    state.ensure_navigation();
    let point = state.navigation_point();
    if let Some(pan) = state.navigation.pan {
        if state
            .navigation
            .view
            .as_mut()
            .is_some_and(|view| view.pan_from(pan.center, pan.start, point, true))
        {
            super::damage::changed(state, dirty);
        }
        sync_bars(state, dirty);
        return;
    }
    sync_bars(state, dirty);
    if let Some(drag) = state.navigation.slider {
        slider_move(state, drag, point, dirty);
        return;
    }
    if !over_ui(state) && bar_hit(state).is_some() {
        if state.ruler.hover.take().is_some() {
            super::damage::changed(state, dirty);
        }
        return;
    }
    crate::app::input::handle_canvas_pointer_move(state, monitor, lx, ly, dirty);
}

/// 【画布导航】【滑块位移】按drag与point重算完整滚动值并应用state，dirty返回视图实际变化，无返回值
fn slider_move(state: &mut EditorState, drag: SliderDrag, point: (f64, f64), dirty: &mut u32) {
    let bars = state.navigation.bars;
    let axis = if drag.horizontal {
        bars.horizontal
    } else {
        bars.vertical
    };
    let Some((track, thumb)) = parts(axis, drag.horizontal, true) else {
        return;
    };
    let (position, start, length, thumb_length) = if drag.horizontal {
        (point.0, track.x, track.width, thumb.width)
    } else {
        (point.1, track.y, track.height, thumb.height)
    };
    let travel = length - thumb_length;
    let value = if travel <= 0.0 {
        0
    } else {
        ((position - start - drag.anchor) / travel * f64::from(axis.maximum)).round() as i32
    };
    let values = if drag.horizontal {
        (value, bars.vertical.value)
    } else {
        (bars.horizontal.value, value)
    };
    if state
        .navigation
        .view
        .as_mut()
        .is_some_and(|view| bars.apply_values(view, values, true))
    {
        super::damage::changed(state, dirty);
    }
    sync_bars(state, dirty);
}

/// 【画布导航】【按钮入口】按button、pressed分派滚动条或中键手势，剩余事件交给原UI和工具，dirty返回重绘范围
pub(crate) fn handle_pointer_button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) {
    if pressed {
        super::preview::clear(state, dirty);
    }
    if button == MouseButton::Middle && !pressed && state.navigation.pan.take().is_some() {
        sync_bars(state, dirty);
        return;
    }
    if button == MouseButton::Left && !pressed && state.navigation.slider.take().is_some() {
        sync_bars(state, dirty);
        return;
    }
    state.ensure_navigation();
    if pressed && !over_ui(state) {
        if let Some(horizontal) = bar_hit(state) {
            if button == MouseButton::Left {
                let axis = if horizontal {
                    state.navigation.bars.horizontal
                } else {
                    state.navigation.bars.vertical
                };
                let point = state.navigation_point();
                if let Some((_, thumb)) = parts(axis, horizontal, true) {
                    if thumb.contains(point) {
                        state.navigation.slider = Some(SliderDrag {
                            horizontal,
                            anchor: if horizontal {
                                point.0 - thumb.x
                            } else {
                                point.1 - thumb.y
                            },
                        });
                    } else {
                        let before = if horizontal {
                            point.0 < thumb.x
                        } else {
                            point.1 < thumb.y
                        };
                        let step = if before {
                            -axis.page_step
                        } else {
                            axis.page_step
                        };
                        let bars = state.navigation.bars;
                        let values = if horizontal {
                            (axis.value.saturating_add(step), bars.vertical.value)
                        } else {
                            (bars.horizontal.value, axis.value.saturating_add(step))
                        };
                        if state
                            .navigation
                            .view
                            .as_mut()
                            .is_some_and(|view| bars.apply_values(view, values, true))
                        {
                            super::damage::changed(state, dirty);
                        }
                    }
                }
                sync_bars(state, dirty);
            }
            return;
        }
        if button == MouseButton::Middle && state.navigation_policy().available() {
            // 1. 【画布导航】【中键开始】先提交文字并结束旧交互，默认无单击动作时立即平移
            crate::editor::text_session::commit(state, dirty);
            state.end_collection_interaction();
            crate::editor::annotation_drag::commit_drag_if_changed(state);
            crate::editor::drawing_gesture::cancel(state, dirty);
            state.laser.cancel_draft();
            state.toolbar_drag = None;
            state.input.left_canvas = false;
            state.navigation.pan =
                state
                    .navigation
                    .view
                    .and_then(|view| view.center())
                    .map(|center| Pan {
                        center,
                        start: state.navigation_point(),
                    });
            super::damage::changed(state, dirty);
            return;
        }
    }
    crate::app::input::handle_canvas_pointer_button(state, button, pressed, dirty);
}

/// 【画布导航】【滚轮入口】按event判定画布职责并调整state，dirty返回变化；返回是否已消费缩放、阻塞或滚动条输入
pub(crate) fn scroll(state: &mut EditorState, event: ScrollEvent, dirty: &mut u32) -> bool {
    if over_ui(state) {
        return false;
    }
    state.ensure_navigation();
    if let Some(horizontal) = bar_hit(state) {
        let bars = state.navigation.bars;
        let axis = if horizontal {
            bars.horizontal
        } else {
            bars.vertical
        };
        let value = axis.value.saturating_sub(
            (event.canvas_steps() * f64::from(axis.single_step) * 3.0).round() as i32,
        );
        let values = if horizontal {
            (value, bars.vertical.value)
        } else {
            (bars.horizontal.value, value)
        };
        if state
            .navigation
            .view
            .as_mut()
            .is_some_and(|view| bars.apply_values(view, values, true))
        {
            super::damage::changed(state, dirty);
        }
        sync_bars(state, dirty);
        return true;
    }
    let blocked = state.input.left_canvas
        || state.input.mouse_down
        || state.tool_active
        || state.ann_drag.is_some()
        || state.collection.drag.is_some()
        || state.toolbar_drag.is_some()
        || state.navigation.pan.is_some();
    let target = state.navigation_policy().wheel_target(
        state.selected_tool,
        state.selected_annotation_ids().len(),
        state
            .text_session
            .as_ref()
            .is_some_and(|session| session.original_id.is_some()),
        blocked,
        event,
    );
    if target == WheelTarget::ImageZoom {
        let point = state.navigation_point();
        if state
            .navigation
            .view
            .as_mut()
            .is_some_and(|view| view.zoom_at(wheel_factor(event), point, true))
        {
            super::damage::changed(state, dirty);
        }
        sync_bars(state, dirty);
        super::preview::show(state, Instant::now(), dirty);
        return true;
    }
    blocked && target == WheelTarget::None
}

/// 【画布导航】【修饰键入口】按ctrl、shift同步state并记录非重复Ctrl按下，dirty返回复位和绘图变化
pub(crate) fn handle_modifiers(state: &mut EditorState, ctrl: bool, shift: bool, dirty: &mut u32) {
    let pressed = ctrl && !state.input.ctrl;
    crate::app::input::handle_canvas_modifiers(state, ctrl, shift, dirty);
    if pressed {
        super::preview::clear(state, dirty);
        ctrl_pressed(state, Instant::now(), dirty);
    }
}

/// 【画布导航】【Ctrl双击】按now与state已存按下时间判断360毫秒复位，dirty返回实际视图变化，无返回值
pub(crate) fn ctrl_pressed(state: &mut EditorState, now: Instant, dirty: &mut u32) {
    if !state.navigation_policy().available() || !state.ensure_navigation() {
        return;
    }
    if state.navigation.ctrl_tap.is_some_and(|previous| {
        now.saturating_duration_since(previous) <= Duration::from_millis(360)
    }) {
        state.navigation.ctrl_tap = None;
        if state
            .navigation
            .view
            .as_mut()
            .is_some_and(|view| view.reset(true))
        {
            super::damage::changed(state, dirty);
        }
        sync_bars(state, dirty);
    } else {
        state.navigation.ctrl_tap = Some(now);
    }
}

/// 【画布导航】【光标归属】按state返回是否由平移或滚动条接收画布指针，不修改状态
pub(crate) fn owns_pointer(state: &EditorState) -> bool {
    state.navigation.pan.is_some()
        || state.navigation.slider.is_some()
        || !over_ui(state) && bar_hit(state).is_some()
}

/// 【画布导航】【焦点中断】结束state的平移、滑块与Ctrl点击序列，dirty返回控件清理，保留当前视图，无返回值
pub(crate) fn focus_lost(state: &mut EditorState, dirty: &mut u32) {
    super::preview::clear(state, dirty);
    let pan = state.navigation.pan.take().is_some();
    let slider = state.navigation.slider.take().is_some();
    let active = pan || slider;
    state.navigation.ctrl_tap = None;
    if active {
        sync_bars(state, dirty);
        super::damage::changed(state, dirty);
    }
}
