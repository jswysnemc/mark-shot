//! 【尺子】【输入手势】在原图范围内绘制、调整和取消长度或角度测量

use tiny_skia::Rect;

use crate::editor::annotation_drag::{apply_annotation_drag, commit_drag_if_changed};
use crate::editor::ruler_session::{cancel_draft, clear_hover};
use crate::editor::{DamageZone, EditorState};
use crate::features::ruler::angle_draft::{self, DraftStep};
use crate::features::ruler::drag;
use crate::features::ruler::labels::{InfoLabel, clamped};
use crate::features::ruler::state::RulerHover;
use crate::features::ruler::{Measurement, RulerGeometry, RulerMode};
use crate::tools::{ToolBehavior, drawing_space::DrawingSpace};
use crate::types::{
    AnnDragState, Annotation, AnnotationShape, CursorIcon, MouseButton, SelectionHandle,
    StrokeGroup,
};
use crate::ui::magnifier::sample_pixel;
use crate::ui::panel::UiPanel;
use crate::utils::{cursor_for_handle, get_overlapping_monitors};

pub struct RulerTool;

impl ToolBehavior for RulerTool {
    /// 【尺子】【鼠标按钮】处理 state 中 button 的 pressed 状态；只响应左键，dirty_mask 返回重绘显示器，无返回值
    fn on_button(
        &self,
        state: &mut EditorState,
        button: MouseButton,
        pressed: bool,
        dirty_mask: &mut u32,
    ) {
        if !matches!(button, MouseButton::Left) {
            return;
        }
        let Some(space) = DrawingSpace::from_editor(state) else {
            return;
        };
        if pressed {
            let pending = angle_pending(state);
            let position = if pending {
                space.clamp_point(state.input.pointer.global)
            } else {
                space.start_point(state.input.pointer.global)
            };
            let Some(position) = position else {
                return;
            };
            if pending {
                update_draft(state, position, space, dirty_mask);
                state.input.mouse_down = true;
                state.tool_active = true;
                return;
            }
            if let Some((index, handle)) = hit(state, position) {
                select_for_drag(state, index, handle, dirty_mask);
                return;
            }
            begin(state, position, space, dirty_mask);
        } else {
            state.input.mouse_down = false;
            state.tool_active = false;
            if state.ann_drag.is_some() {
                commit_drag_if_changed(state);
                return;
            }
            let Some(position) = space.clamp_point(state.input.pointer.global) else {
                return;
            };
            update_draft(state, position, space, dirty_mask);
            finish(state, position, space, dirty_mask);
        }
    }

    /// 【尺子】【指针移动】用 global 更新 state 的悬停、拖动或待定角度；dirty_mask 返回重绘显示器，无返回值
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), dirty_mask: &mut u32) {
        let Some(space) = DrawingSpace::from_editor(state) else {
            return;
        };
        let Some(position) = space.clamp_point(global) else {
            return;
        };
        update_hover(state, space, dirty_mask);
        if state.ann_drag.is_some() {
            apply_annotation_drag(state, global);
        } else if state.input.mouse_down || angle_pending(state) {
            update_draft(state, position, space, dirty_mask);
        }
    }

    /// 【尺子】【工具进入】在 state 中显示当前源图悬停信息；dirty_mask 返回重绘显示器，无返回值
    fn on_activate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        if let Some(space) = DrawingSpace::from_editor(state) {
            update_hover(state, space, dirty_mask);
        }
    }

    /// 【尺子】【工具退出】结束 state 中已有尺子拖动，取消未完成草稿并清除悬停；dirty_mask 返回重绘显示器，无返回值
    fn on_deactivate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        commit_drag_if_changed(state);
        cancel_draft(state, dirty_mask);
        state.input.mouse_down = false;
        state.tool_active = false;
        clear_hover(state, dirty_mask);
    }

    /// 【尺子】【工具光标】根据 state 的草稿或已有控制点返回十字线、移动或缩放光标
    fn cursor(&self, state: &EditorState) -> CursorIcon {
        if let Some(drag) = &state.ann_drag {
            return cursor_for_handle(drag.handle, true).unwrap_or(CursorIcon::Crosshair);
        }
        if angle_pending(state) {
            return CursorIcon::Crosshair;
        }
        let position = (
            state.input.pointer.global.0 as f32,
            state.input.pointer.global.1 as f32,
        );
        hit(state, position)
            .and_then(|(_, handle)| cursor_for_handle(handle, false))
            .unwrap_or(CursorIcon::Crosshair)
    }
}

/// 【尺子】【待定角度】返回 state 是否保留已放置顶点、尚未提交的角度尺
pub fn angle_pending(state: &EditorState) -> bool {
    state.ann_drag.is_none() && state.pending.as_ref().is_some_and(|annotation| matches!(annotation.shape, AnnotationShape::Ruler { measurement } if measurement.mode() == RulerMode::Angle))
}

/// 【尺子】【已有尺子命中】在 state 中优先命中选中尺子，再从上到下查找其他尺子；position 为全局坐标，返回索引和调整方式
fn hit(state: &EditorState, position: (f32, f32)) -> Option<(usize, SelectionHandle)> {
    if let Some(index) = state.selected_annotation
        && let Some(annotation) = state.annotations.get(index)
        && let AnnotationShape::Ruler { measurement } = annotation.shape
    {
        let handle = drag::hit_test(measurement, position, annotation.stroke_width, true, false);
        if handle != SelectionHandle::None {
            return Some((index, handle));
        }
    }
    state
        .annotations
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, annotation)| {
            let AnnotationShape::Ruler { measurement } = annotation.shape else {
                return None;
            };
            let handle =
                drag::hit_test(measurement, position, annotation.stroke_width, false, false);
            (handle != SelectionHandle::None).then_some((index, handle))
        })
}

/// 【尺子】【开始调整】为 state 中 index 标注保存 handle 拖动快照；dirty_mask 返回选择框重绘，无返回值
fn select_for_drag(
    state: &mut EditorState,
    index: usize,
    handle: SelectionHandle,
    dirty_mask: &mut u32,
) {
    if let Some(previous) = state
        .selected_annotation
        .and_then(|index| state.annotations.get(index))
    {
        damage(state, previous.damage_bbox(true), dirty_mask);
    }
    let annotation = state.annotations[index].clone();
    state.selected_annotation = Some(index);
    state.ann_drag = Some(AnnDragState {
        handle,
        start_global: state.input.pointer.global,
        prev_global: state.input.pointer.global,
        orig: annotation.clone(),
        orig_index: index,
    });
    state.input.mouse_down = true;
    state.tool_active = true;
    damage(state, annotation.damage_bbox(true), dirty_mask);
}

/// 【尺子】【创建草稿】以 position、space 在 state 中开始当前模式的尺子；dirty_mask 返回重绘显示器，无返回值
fn begin(state: &mut EditorState, position: (f32, f32), space: DrawingSpace, dirty_mask: &mut u32) {
    if state.selection.zone.is_none() {
        return;
    }
    let Some(measurement) =
        Measurement::new(state.tool_settings.ruler_mode, position, space.pixel_scale)
    else {
        return;
    };
    if let Some(previous) = state
        .selected_annotation
        .and_then(|index| state.annotations.get(index))
    {
        damage(state, previous.damage_bbox(true), dirty_mask);
    }
    state.selected_annotation = None;
    let mut annotation = Annotation {
        id: state.next_id,
        shape: AnnotationShape::Ruler { measurement },
        color: state.tool_settings.color,
        shadow_color: crate::renderer::shadow_color_for(state.tool_settings.color),
        stroke_width: state.tool_settings.stroke(StrokeGroup::Ruler),
        bbox: Rect::from_xywh(position.0, position.1, 1.0, 1.0).unwrap(),
    };
    annotation.update_bbox();
    state.input.mouse_down = true;
    state.tool_active = true;
    damage(state, annotation.damage_bbox(false), dirty_mask);
    state.pending = Some(annotation.clone());
    state.prev_pending = Some(annotation);
}

/// 【尺子】【草稿跟随】以 position、space 更新 state 的尺子草稿，Shift 吸附角度边；dirty_mask 返回新旧区域，无返回值
fn update_draft(
    state: &mut EditorState,
    position: (f32, f32),
    space: DrawingSpace,
    dirty_mask: &mut u32,
) {
    let Some(annotation) = state.pending.as_mut() else {
        return;
    };
    let old = annotation.damage_bbox(false);
    let AnnotationShape::Ruler { measurement } = &mut annotation.shape else {
        return;
    };
    match &mut measurement.geometry {
        RulerGeometry::Length { end, .. } => *end = position,
        RulerGeometry::Angle { points, count } => {
            let point = angle_draft::constrained_point(points[0], position, state.input.shift);
            if let Some(point) = space.clamp_point((point.0 as f64, point.1 as f64)) {
                points[usize::from(count.saturating_sub(1).min(2))] = point;
            }
        }
    }
    annotation.color = state.tool_settings.color;
    annotation.stroke_width = state.tool_settings.stroke(StrokeGroup::Ruler);
    annotation.update_bbox();
    let new = annotation.damage_bbox(false);
    damage(state, old, dirty_mask);
    damage(state, new, dirty_mask);
}

/// 【尺子】【草稿提交】在 position 松开 state 的草稿；space 用于像素约束，dirty_mask 返回重绘显示器，角度未完成时继续保留草稿
fn finish(
    state: &mut EditorState,
    position: (f32, f32),
    space: DrawingSpace,
    dirty_mask: &mut u32,
) {
    let Some(annotation) = state.pending.as_mut() else {
        return;
    };
    let AnnotationShape::Ruler { measurement } = &mut annotation.shape else {
        return;
    };
    if measurement.mode() == RulerMode::Angle {
        let point =
            angle_draft::constrained_point(measurement.start(), position, state.input.shift);
        let Some(point) = space.clamp_point((point.0 as f64, point.1 as f64)) else {
            return;
        };
        if angle_draft::release(measurement, point, 4.0) == DraftStep::Continue {
            annotation.update_bbox();
            return;
        }
    }
    let measures = measurement.measures();
    let annotation = state.pending.take().unwrap();
    state.prev_pending = None;
    damage(state, annotation.damage_bbox(false), dirty_mask);
    if !measures {
        return;
    }
    state.push_undo();
    state.next_id += 1;
    state.bake_annotation(&annotation);
    state.annotations.push(annotation);
}

/// 【尺子】【界面悬停】返回 state 指针是否位于可见工具栏、属性栏或弹层；不把画布当作界面控件
fn over_ui(state: &EditorState) -> bool {
    let pointer = state.input.pointer;
    [
        state
            .toolbar
            .rect()
            .filter(|_| pointer.monitor_idx == state.toolbar.monitor_idx),
        state
            .settings_panel
            .rect()
            .filter(|_| pointer.monitor_idx == state.settings_panel.monitor_idx),
        state
            .color_popover
            .rect()
            .filter(|_| pointer.monitor_idx == state.color_popover.monitor_idx),
        state
            .model_popover
            .rect()
            .filter(|_| pointer.monitor_idx == state.model_popover.monitor_idx),
        state
            .marker_popover
            .rect()
            .filter(|_| pointer.monitor_idx == state.marker_popover.monitor_idx),
        state
            .ruler_popover
            .rect()
            .filter(|_| pointer.monitor_idx == state.ruler_popover.monitor_idx),
    ]
    .into_iter()
    .flatten()
    .any(|rect| {
        crate::features::ruler::geometry::contains(
            rect,
            (pointer.local.0 as f32, pointer.local.1 as f32),
        )
    })
}

/// 【尺子】【悬停更新】用 space 的原图比例读取 state 指针位置与颜色；dirty_mask 返回旧十字线与新十字线区域，无返回值
fn update_hover(state: &mut EditorState, space: DrawingSpace, dirty_mask: &mut u32) {
    if !state.input.mouse_down
        && (over_ui(state) || space.start_point(state.input.pointer.global).is_none())
    {
        clear_hover(state, dirty_mask);
        return;
    }
    let Some(point) = space.clamp_point(state.input.pointer.global) else {
        return;
    };
    let image_point = (
        (point.0 - space.bounds.left()) * space.pixel_scale,
        (point.1 - space.bounds.top()) * space.pixel_scale,
    );
    let color = if let Some(file) = &state.file_canvas {
        sample_pixel(&file.original, (image_point.0 as f64, image_point.1 as f64))
    } else {
        state
            .captures
            .get(state.input.pointer.monitor_idx)
            .and_then(|capture| {
                sample_pixel(
                    &capture.pixmap,
                    capture.to_native(state.input.pointer.local),
                )
            })
    };
    let color = color.unwrap_or(tiny_skia::Color::TRANSPARENT).to_color_u8();
    let text = format!(
        "x {}  y {}  #{:02X}{:02X}{:02X}",
        image_point.0.round() as i64,
        image_point.1.round() as i64,
        color.red(),
        color.green(),
        color.blue()
    );
    let width =
        crate::renderer::measure_line_width(&text, 13.0, &mut state.text.font_system) + 18.0;
    let height = 26.0;
    let left = if point.0 + 14.0 + width > space.bounds.right() {
        point.0 - 14.0 - width
    } else {
        point.0 + 14.0
    };
    let top = if point.1 + 14.0 + height > space.bounds.bottom() {
        point.1 - 14.0 - height
    } else {
        point.1 + 14.0
    };
    let Some(rect) = Rect::from_xywh(left, top, width, height) else {
        return;
    };
    let hover = RulerHover {
        point,
        bounds: space.bounds,
        label: InfoLabel {
            rect: clamped(rect, space.bounds, 8.0),
            unit: 1.0,
            lines: vec![text],
            font_size: 13.0,
        },
    };
    if state.ruler.hover.as_ref() == Some(&hover) {
        return;
    }
    if let Some(previous) = state.ruler.hover.as_ref() {
        damage(state, previous.bounds, dirty_mask);
    }
    damage(state, hover.bounds, dirty_mask);
    state.ruler.hover = Some(hover);
}

/// 【尺子】【重绘通知】把 area 加入 state 的逻辑重绘区域；dirty_mask 返回全部重叠显示器，无返回值
fn damage(state: &mut EditorState, area: Rect, dirty_mask: &mut u32) {
    state.damage_rects.push(DamageZone::Global(area));
    *dirty_mask |= get_overlapping_monitors(&area, &state.placements);
}
