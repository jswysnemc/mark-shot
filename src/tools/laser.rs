//! 【激光】【输入交互】将指针手势转为独立临时笔迹，并通知屏幕重绘

use std::time::Instant;

use tiny_skia::Rect;

use crate::editor::{DamageZone, EditorState};
use crate::tools::ToolBehavior;
use crate::types::{MouseButton, StrokeGroup};
use crate::utils::{get_full_workspace_rect, get_overlapping_monitors};

pub struct LaserTool;

impl ToolBehavior for LaserTool {
    /// 【激光】【鼠标按钮】处理 state 中的左键 pressed 状态；button 指定按钮，dirty_mask 返回重绘显示器
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
        let damage = if pressed {
            if state.selection.zone.is_none() {
                return;
            }
            let Some(bounds) = state
                .file_canvas
                .as_ref()
                .map(|file| file.viewport)
                .or_else(|| get_full_workspace_rect(&state.placements))
            else {
                return;
            };
            let position = state.canvas_point();
            let units = state.annotation_units();
            let damage = state.laser.begin_in_units(
                (position.0 as f32, position.1 as f32),
                state.tool_settings.color,
                state.tool_settings.stroke(StrokeGroup::Laser),
                bounds,
                units,
            );
            state.input.mouse_down = damage.is_some();
            state.tool_active = damage.is_some();
            damage
        } else {
            state.input.mouse_down = false;
            state.tool_active = false;
            state.laser.commit(Instant::now())
        };
        record_damage(state, damage, dirty_mask);
    }

    /// 【激光】【鼠标移动】将 global 坐标追加到 state 中的草稿；dirty_mask 返回重绘显示器，无返回值
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), dirty_mask: &mut u32) {
        if !state.input.mouse_down {
            return;
        }
        let damage = state.laser.extend((global.0 as f32, global.1 as f32));
        record_damage(state, damage, dirty_mask);
    }

    /// 【激光】【工具退出】取消 state 中的未提交草稿并结束手势；dirty_mask 返回清除区域，无返回值
    fn on_deactivate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        let damage = state.laser.cancel_draft();
        state.input.mouse_down = false;
        state.tool_active = false;
        state.input.drag_start = None;
        record_damage(state, damage, dirty_mask);
    }
}

/// 【激光】【后台刷新】按 now 清理 state 的过期笔迹；dirty_mask 返回全部需重绘显示器，无返回值
pub(crate) fn tick(state: &mut EditorState, dirty_mask: &mut u32, now: Instant) {
    let damage = state.laser.tick(now);
    record_damage(state, damage, dirty_mask);
}

/// 【激光】【重绘通知】把可选 area 加入 state 的损坏区域，并更新 dirty_mask；无返回值
fn record_damage(state: &mut EditorState, area: Option<Rect>, dirty_mask: &mut u32) {
    if let Some(area) = area {
        state.damage_rects.push(DamageZone::Global(area));
        *dirty_mask |= get_overlapping_monitors(&area, &state.placements);
    }
}
