//! 【截图输入】【按钮职责】左键才操作控件与画布，右键回到标注选择，中键进入全图编辑

use super::ui_hit::{UiHit, hit_test_ui};
use crate::{
    editor::{EditorState, dirty::mark_all_dirty},
    interaction::ClickTarget,
    tools::Tool,
    types::MouseButton,
    ui::{
        panel::UiPanel,
        toolbar::{ToolbarButton, ToolbarItem},
    },
};

/// 【截图输入】【非左键分流】处理 state 中 button 的 pressed 状态；返回是否消费事件，dirty 返回重绘显示器
pub(super) fn consume(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if matches!(button, MouseButton::Left) {
        return false;
    }
    if !pressed {
        return true;
    }
    if matches!(button, MouseButton::Right) && crate::tools::text::session_input::inside(state) {
        return true;
    }
    state.input.toolbar_press = None;
    state.input.color_button_press = None;
    super::text_properties::finish(state, false, dirty);
    state.font_panel.pressed = None;
    state.font_panel.scrollbar_drag = None;
    if matches!(button, MouseButton::Right)
        && state.color_popover.open
        && state.color_popover.radial.is_some()
    {
        if right_double_click(state) {
            super::super::panels::radial_palette::toggle(state, dirty);
        }
        return true;
    }
    // 1. 【截图输入】【界面保护】非左键不触发按钮，也不穿透面板修改下方画布
    if !matches!(hit_test_ui(state, state.input.pointer.local), UiHit::None) {
        if state.color_popover.open {
            super::super::panels::color::close_color_popover(state, dirty);
        }
        if state.model_popover.open {
            super::super::panels::model::close_model_popover(state, dirty);
        }
        super::super::panels::marker::close(state, dirty);
        super::super::panels::ruler::close(state, dirty);
        super::super::panels::number::close(state, dirty);
        super::super::panels::line::close(state, dirty);
        super::super::panels::filter::close(state, dirty);
        super::font_popup::close(state, dirty);
        return true;
    }
    if matches!(button, MouseButton::Middle) {
        if state.selection.zone.is_none() && !state.input.mouse_down {
            toggle_scope(state, dirty);
        }
        return true;
    }
    // 2. 【截图输入】【右键取消】原版选择阶段关闭会话，编辑阶段保留选区和已完成标注
    if state.selection.zone.is_none()
        || (state.selected_tool == Tool::Selection && state.input.drag_start.is_some())
    {
        if state.selected_tool == Tool::Selection {
            state.cancel = true;
        } else {
            super::select_tool(state, Tool::Selection, dirty);
        }
        return true;
    }
    if state.ann_drag.is_some() {
        state.undo(dirty);
    }
    state.end_collection_interaction();
    crate::editor::drawing_gesture::cancel(state, dirty);
    state.laser.cancel_draft();
    state.toolbar_drag = None;
    state.input.left_canvas = false;
    if state.selected_tool != Tool::Pick {
        super::select_tool(state, Tool::Pick, dirty);
    }
    if right_double_click(state) {
        super::super::panels::radial_palette::toggle(state, dirty);
    }
    mark_all_dirty(dirty, state.placements.len());
    true
}

/// 【截图输入】【右键双击】按 state 的全局位置识别不重叠的点击对；返回本次是否完成双击，三连击不重复切换
fn right_double_click(state: &mut EditorState) -> bool {
    let point = (
        state.input.pointer.global.0 as f32,
        state.input.pointer.global.1 as f32,
    );
    let double = state
        .input
        .right_clicks
        .register(ClickTarget::Selection, point);
    if double {
        state.input.right_clicks.clear();
    }
    double
}

/// 【截图工具栏】【范围切换】在 state 中切换全图和原选区；dirty 返回重绘显示器，不删除标注
pub(super) fn toggle_scope(state: &mut EditorState, dirty: &mut u32) {
    if state.tool_active || state.pending.is_some() || state.ann_drag.is_some() {
        return;
    }
    if state.fullscreen_scope {
        state.selection.zone = state.scope_before_fullscreen.take();
        state.fullscreen_scope = false;
    } else {
        state.scope_before_fullscreen = state.selection.zone;
        state.selection.zone = crate::utils::get_full_workspace_rect(&state.placements);
        state.fullscreen_scope = true;
    }
    super::select_tool(state, Tool::Selection, dirty);
    mark_all_dirty(dirty, state.placements.len());
    super::refresh_panels(state, dirty);
}

/// 【截图工具栏】【左键按钮】在 state 中执行 secondary 面板的 index 按钮；dirty 返回重绘显示器
pub(super) fn toolbar_click(
    state: &mut EditorState,
    secondary: bool,
    index: usize,
    dirty: &mut u32,
) {
    let toolbar = if secondary {
        &state.action_toolbar
    } else {
        &state.toolbar
    };
    if toolbar.disabled.contains(&index) {
        return;
    }
    let Some(ToolbarItem::Button(button)) = toolbar.items.get(index).copied() else {
        return;
    };
    if state.input.mouse_down || state.ann_drag.is_some() {
        return;
    }
    if state.pending.is_some()
        && !matches!(
            button,
            ToolbarButton::Tool(_) | ToolbarButton::Undo | ToolbarButton::Cancel
        )
    {
        return;
    }
    if let Some(action) = button.action() {
        super::super::actions::run(state, action, dirty);
        return;
    }
    match button {
        ToolbarButton::Clear => {
            if !state.annotations.is_empty() {
                state.push_undo();
                state.annotations.clear();
                state.select_annotation_index(None);
                state.number_sequence = Default::default();
                state.text.editors.clear();
                state.text.editing = None;
                state.annotations_dirty = true;
                state.layer_damage_rects.clear();
            }
        }
        ToolbarButton::Scope => toggle_scope(state, dirty),
        ToolbarButton::Layout => {
            state.toolbar.orientation = match state.toolbar.orientation {
                crate::ui::toolbar::Orientation::Horizontal => {
                    crate::ui::toolbar::Orientation::Vertical
                }
                crate::ui::toolbar::Orientation::Vertical => {
                    crate::ui::toolbar::Orientation::Horizontal
                }
            };
        }
        _ => {}
    }
    super::refresh_panels(state, dirty);
    mark_all_dirty(dirty, state.placements.len());
}

/// 【截图工具栏】【握柄按下】在 state 的 secondary 面板内记录拖动偏移；返回是否命中握柄
pub(super) fn begin_toolbar_drag(state: &mut EditorState, secondary: bool) -> bool {
    let toolbar = if secondary {
        &state.action_toolbar
    } else {
        &state.toolbar
    };
    let point = tiny_skia::Point::from_xy(
        state.input.pointer.local.0 as f32 - toolbar.render_pos.0,
        state.input.pointer.local.1 as f32 - toolbar.render_pos.1,
    );
    if crate::ui::toolbar::contains(toolbar.grip(), (point.x, point.y)) {
        state.toolbar_drag = Some((secondary, (point.x, point.y)));
        return true;
    }
    false
}

/// 【截图工具栏】【握柄移动】按 state 的当前指针更新面板位置；dirty 返回重绘显示器
pub(super) fn move_toolbar(state: &mut EditorState, dirty: &mut u32) {
    let Some((secondary, offset)) = state.toolbar_drag else {
        return;
    };
    let monitor = state.input.pointer.monitor_idx;
    let bounds = state.placements[monitor].size;
    let toolbar = if secondary {
        &mut state.action_toolbar
    } else {
        &mut state.toolbar
    };
    if let Some(rect) = toolbar.rect() {
        state.damage_rects.push(crate::editor::DamageZone::Local {
            monitor_idx: toolbar.monitor_idx,
            rect,
        });
    }
    toolbar.monitor_idx = monitor;
    toolbar.user_position = Some((
        (state.input.pointer.local.0 as f32 - offset.0)
            .clamp(8.0, (bounds.0 as f32 - toolbar.size.0 - 8.0).max(8.0)),
        (state.input.pointer.local.1 as f32 - offset.1)
            .clamp(8.0, (bounds.1 as f32 - toolbar.size.1 - 8.0).max(8.0)),
    ));
    toolbar.position = toolbar.user_position.unwrap();
    toolbar.render_pos = toolbar.position;
    toolbar.dirty = true;
    super::refresh_panels(state, dirty);
}
