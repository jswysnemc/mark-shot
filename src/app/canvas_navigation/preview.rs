//! 【画布导航】【缩放提示】九百毫秒单次期限，不启动常驻线程或逐帧轮询

use crate::editor::EditorState;
use crate::features::wheel_preview::model::Sample;
use crate::ui::panel::UiPanel;
use std::time::{Duration, Instant};
use tiny_skia::Rect;

pub(crate) struct Preview {
    pub monitor: usize,
    pub rect: Rect,
    pub text: String,
    pub deadline: Instant,
    pub sample: Option<Sample>,
    pub point: (f32, f32),
}

impl Preview {
    /// 【滚轮提示】【完整损坏】无额外参数，返回气泡、十字和尺寸轮廓的完整窗口范围
    pub(crate) fn damage_bounds(&self) -> Rect {
        let rect = self.rect.outset(2.0, 2.0).unwrap_or(self.rect);
        if let Some(sample) = self.sample {
            let radius = (sample.footprint().unwrap_or(0.0) / 2.0 + 3.0).max(12.0);
            let pointer = Rect::from_xywh(
                self.point.0 - radius,
                self.point.1 - radius,
                radius * 2.0,
                radius * 2.0,
            )
            .unwrap();
            rect.join(&pointer).unwrap_or(rect)
        } else {
            rect
        }
    }
}

/// 【画布导航】【提示损坏】按state与dirty登记当前气泡完整边界，返回无值
fn damage(state: &mut EditorState, dirty: &mut u32) {
    if let Some(preview) = &state.navigation.preview {
        let monitor = preview.monitor;
        let rect = preview.damage_bounds();
        state.damage_local(monitor, rect);
        crate::editor::dirty::mark_dirty(dirty, monitor);
    }
}

/// 【画布导航】【气泡开始】按now显示state当前倍率，dirty返回旧新范围；图外锚点不显示提示，无返回值
pub(crate) fn show(state: &mut EditorState, now: Instant, dirty: &mut u32) {
    clear(state, dirty);
    let Some(view) = state.navigation.view else {
        return;
    };
    if !view
        .projection()
        .widget_rect
        .contains(state.navigation_point())
    {
        return;
    }
    let text = format!("{}%", (view.zoom() * 100.0).round() as u32);
    let monitor = state.input.pointer.monitor_idx;
    let size = state.placements[monitor].size;
    let rect = crate::ui::canvas_navigation::zoom::rect(
        &text,
        state.input.pointer.local,
        (size.0 as f32, size.1 as f32),
        state
            .navigation
            .preview_fonts
            .get_or_insert_with(crate::fonts::selection_ui::new),
    );
    state.navigation.preview = Some(Preview {
        monitor,
        rect,
        text,
        deadline: now + Duration::from_millis(900),
        sample: None,
        point: (
            state.input.pointer.local.0 as f32,
            state.input.pointer.local.1 as f32,
        ),
    });
    damage(state, dirty);
}

/// 【滚轮提示】【数值气泡位置】按state、text和sample返回原版四候选位置，工具栏、属性栏与当前弹层参与避让
fn sample_rect(state: &mut EditorState, text: &str, sample: Sample) -> Rect {
    let monitor = state.input.pointer.monitor_idx;
    let point = (
        state.input.pointer.local.0 as f32,
        state.input.pointer.local.1 as f32,
    );
    let size = state.placements[monitor].size;
    let metrics = crate::fonts::ruler::measure(text, 13.0);
    let mut obstacles = Vec::new();
    if !state.ui_hidden {
        for (owner, rect) in [
            (state.toolbar.monitor_idx, state.toolbar.rect()),
            (
                state.action_toolbar.monitor_idx,
                state.action_toolbar.rect(),
            ),
            (
                state.settings_panel.monitor_idx,
                state.settings_panel.rect(),
            ),
            (state.color_popover.monitor_idx, state.color_popover.rect()),
            (
                state.marker_popover.monitor_idx,
                state.marker_popover.rect(),
            ),
            (state.ruler_popover.monitor_idx, state.ruler_popover.rect()),
            (
                state.number_popover.monitor_idx,
                state.number_popover.rect(),
            ),
        ] {
            if owner == monitor
                && let Some(rect) = rect
            {
                obstacles.push(rect);
            }
        }
    }
    state
        .navigation
        .preview_fonts
        .get_or_insert_with(crate::fonts::selection_ui::new);
    crate::features::wheel_preview::layout::badge(
        point,
        (
            metrics.width.ceil() + 44.0,
            (metrics.height.ceil() + 10.0).max(26.0),
        ),
        sample.footprint(),
        Rect::from_xywh(0.0, 0.0, size.0 as f32, size.1 as f32).unwrap(),
        &obstacles,
    )
}

/// 【滚轮提示】【真实数值开始】按sample和now显示state指针旁的尺寸提示，dirty返回旧新区域，无返回值
pub(crate) fn show_sample(state: &mut EditorState, sample: Sample, now: Instant, dirty: &mut u32) {
    clear(state, dirty);
    let text = sample.text();
    let rect = sample_rect(state, &text, sample);
    state.navigation.preview = Some(Preview {
        monitor: state.input.pointer.monitor_idx,
        rect,
        text,
        deadline: now + Duration::from_millis(900),
        sample: Some(sample),
        point: (
            state.input.pointer.local.0 as f32,
            state.input.pointer.local.1 as f32,
        ),
    });
    damage(state, dirty);
}

/// 【滚轮提示】【指针跟随】按state当前指针移动未到期气泡，dirty返回旧新区域；不延长九百毫秒期限
pub(crate) fn follow(state: &mut EditorState, dirty: &mut u32) {
    let Some(preview) = state.navigation.preview.as_ref() else {
        return;
    };
    let point = (
        state.input.pointer.local.0 as f32,
        state.input.pointer.local.1 as f32,
    );
    if preview.point == point && preview.monitor == state.input.pointer.monitor_idx {
        return;
    }
    let deadline = preview.deadline;
    let sample = preview.sample;
    let text = preview.text.clone();
    if Instant::now() >= deadline || crate::app::input::wheel_preview::blocked(state) {
        clear(state, dirty);
        return;
    }
    damage(state, dirty);
    let rect = if let Some(sample) = sample {
        sample_rect(state, &text, sample)
    } else {
        let size = state.placements[state.input.pointer.monitor_idx].size;
        crate::ui::canvas_navigation::zoom::rect(
            &text,
            state.input.pointer.local,
            (size.0 as f32, size.1 as f32),
            state.navigation.preview_fonts.as_mut().unwrap(),
        )
    };
    if let Some(preview) = state.navigation.preview.as_mut() {
        preview.rect = rect;
        preview.point = point;
        preview.monitor = state.input.pointer.monitor_idx;
    }
    damage(state, dirty);
}

/// 【画布导航】【气泡结束】清理state气泡并登记dirty，返回无值；未显示时不产生重绘
pub(crate) fn clear(state: &mut EditorState, dirty: &mut u32) {
    damage(state, dirty);
    state.navigation.preview = None;
}

/// 【画布导航】【期限到达】按now判断state提示是否到期并清理dirty，返回无值，不要求后续鼠标事件
pub(crate) fn tick(state: &mut EditorState, now: Instant, dirty: &mut u32) {
    if state
        .navigation
        .preview
        .as_ref()
        .is_some_and(|preview| now >= preview.deadline)
    {
        clear(state, dirty);
    }
}
