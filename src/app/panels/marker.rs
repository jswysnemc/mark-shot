//! 【形状标记】【选择流程】连接网格选择、工具默认值、已有标注属性和撤销历史

use crate::editor::EditorState;
use crate::editor::edits::{active_annotation_idx, commit_settings_change};
use crate::keys::Chord;
use crate::theme::size;
use crate::tools::Tool;
use crate::types::{AnnotationShape, MarkerShape};
use crate::ui::choice_popover::ChoiceEvent;
use crate::ui::panel::{UiPanel, emit_panel_damage, sync_panel_hover, sync_panel_rect};
use crate::ui::settings_panel::{SettingsAction, SettingsWidget};

/// 【形状标记】【当前形状】返回 state 中已有标注的形状，未选中标记时返回工具默认值
fn current_shape(state: &EditorState) -> MarkerShape {
    match active_annotation_idx(state)
        .and_then(|index| state.annotations.get(index))
        .map(|annotation| &annotation.shape)
    {
        Some(AnnotationShape::Marker { kind, .. }) => *kind,
        _ => state.tool_settings.marker_shape,
    }
}

/// 【形状标记】【面板来源】返回 state 是否处于标记工具或选中已有标记，防止修改其他类型标注
fn available(state: &EditorState) -> bool {
    state.selected_tool == Tool::Marker
        || active_annotation_idx(state)
            .and_then(|index| state.annotations.get(index))
            .is_some_and(|annotation| matches!(&annotation.shape, AnnotationShape::Marker { .. }))
}

/// 【形状标记】【面板同步】适配 state 的显示器空间、位置和当前形状；dirty_mask 返回重绘显示器，无返回值
pub fn update(state: &mut EditorState, dirty_mask: &mut u32) {
    let old_rect = state.marker_popover.rect();
    let old_monitor = state.marker_popover.monitor_idx;
    if state.marker_popover.open && (!available(state) || state.ui_hidden) {
        state.marker_popover.open = false;
    }
    let mut changed = state.marker_popover.dirty;
    if state.marker_popover.is_visible() {
        let index = state.settings_panel.monitor_idx;
        let placement = &state.placements[index];
        let margin = size::margin();
        changed |= state.marker_popover.resize((
            placement.size.0 as f32 - margin * 2.0,
            placement.size.1 as f32 - margin * 2.0,
        ));
        let (position, monitor) = super::settings::compute_popover_placement(
            state,
            state.marker_popover.layout.size,
            margin,
        );
        state.marker_popover.position = position;
        state.marker_popover.render_pos = position;
        state.marker_popover.monitor_idx = monitor;
        let active = MarkerShape::ALL
            .iter()
            .position(|shape| *shape == current_shape(state));
        if state.marker_popover.active != active {
            state.marker_popover.active = active;
            changed = true;
        }
    }
    sync_panel_rect(
        &mut state.marker_popover,
        old_rect,
        old_monitor,
        changed,
        &mut state.damage_rects,
        dirty_mask,
    );
    sync_button(state, dirty_mask);
}

/// 【形状标记】【打开切换】切换 state 的形状面板并关闭其他弹层；dirty_mask 返回重绘显示器，无返回值
pub fn toggle(state: &mut EditorState, dirty_mask: &mut u32) {
    super::text::font_popup::close(state, dirty_mask);
    if !available(state) {
        return;
    }
    super::color::close_color_popover(state, dirty_mask);
    super::model::close_model_popover(state, dirty_mask);
    super::ruler::close(state, dirty_mask);
    super::number::close(state, dirty_mask);
    super::line::close(state, dirty_mask);
    super::filter::close(state, dirty_mask);
    state.marker_popover.open = !state.marker_popover.open;
    state.marker_popover.hovered = None;
    state.marker_popover.cancel_scroll();
    state.marker_popover.dirty = true;
    update(state, dirty_mask);
    if let Some(index) = state.marker_popover.active {
        state.marker_popover.ensure_visible(index);
    }
}

/// 【形状标记】【关闭面板】关闭 state 的形状选择并保留退出动画；dirty_mask 返回重绘显示器，无返回值
pub fn close(state: &mut EditorState, dirty_mask: &mut u32) {
    if !state.marker_popover.is_visible() {
        return;
    }
    state.marker_popover.open = false;
    state.marker_popover.hovered = None;
    state.marker_popover.cancel_scroll();
    state.marker_popover.dirty = true;
    update(state, dirty_mask);
}

/// 【形状标记】【按钮命中】判断 state 的指针是否位于形状选择按钮；返回布尔值
pub fn toggle_button_hit(state: &EditorState) -> bool {
    if state.input.pointer.monitor_idx != state.settings_panel.monitor_idx {
        return false;
    }
    let (_, index) = state.settings_panel.hit_test(state.input.pointer.local);
    matches!(
        index.and_then(|index| state.settings_panel.widgets.get(index)),
        Some(SettingsWidget::Action {
            action: SettingsAction::MarkerShapes,
            ..
        })
    )
}

/// 【形状标记】【提交选择】把 index 显示项应用到 state；标注变化记录一次撤销，dirty_mask 返回重绘显示器
fn choose(state: &mut EditorState, index: usize, dirty_mask: &mut u32) {
    let Some(&shape) = MarkerShape::ALL.get(index) else {
        return;
    };
    let annotation_changed = active_annotation_idx(state).and_then(|index| state.annotations.get(index)).is_some_and(|annotation| matches!(&annotation.shape, AnnotationShape::Marker { kind, .. } if *kind != shape));
    let changed = annotation_changed || state.tool_settings.marker_shape != shape;

    // 1. 【形状标记】【提交选择】只对已有标注变化记录历史，工具默认值调整不产生空撤销
    commit_settings_change(
        state,
        changed,
        annotation_changed,
        |settings| settings.marker_shape = shape,
        |annotation| {
            if let AnnotationShape::Marker { kind, .. } = &mut annotation.shape {
                *kind = shape;
            }
        },
        dirty_mask,
    );

    // 2. 【形状标记】【提交选择】更新属性与工具图标，再关闭选择面板
    super::settings::update_settings_panel(state, dirty_mask);
    super::toolbar::update_toolbar(state, dirty_mask);
    close(state, dirty_mask);
}

/// 【形状标记】【点击选择】使用 state 当前指针命中选择项；dirty_mask 返回重绘显示器，无返回值
pub fn click(state: &mut EditorState, dirty_mask: &mut u32) {
    if state.input.pointer.monitor_idx == state.marker_popover.monitor_idx
        && let Some(index) = state.marker_popover.item_at(state.input.pointer.local)
    {
        choose(state, index, dirty_mask);
    }
}

/// 【形状标记】【指针悬停】按 state 的实际鼠标移动更新预选，不覆盖键盘导航；dirty_mask 返回重绘显示器
pub fn pointer_move(state: &mut EditorState, dirty_mask: &mut u32) {
    update(state, dirty_mask);
    if state.marker_popover.open {
        let hovered = (state.input.pointer.monitor_idx == state.marker_popover.monitor_idx)
            .then(|| state.marker_popover.item_at(state.input.pointer.local))
            .flatten();
        sync_panel_hover(
            &mut state.marker_popover,
            hovered,
            &mut state.damage_rects,
            dirty_mask,
        );
    }
}

/// 【形状标记】【键盘选择】处理 state 的 chord；dirty_mask 返回重绘显示器，返回是否阻止其他快捷键
pub fn handle_key(state: &mut EditorState, chord: Chord, dirty_mask: &mut u32) -> bool {
    if !state.marker_popover.open {
        return false;
    }
    state.marker_popover.cancel_scroll();
    if chord.mods.ctrl || chord.mods.alt || chord.mods.logo {
        close(state, dirty_mask);
        return false;
    }
    match state.marker_popover.key(chord.key, chord.mods.shift) {
        Some(ChoiceEvent::Selected(index)) => choose(state, index, dirty_mask),
        Some(ChoiceEvent::Close) => close(state, dirty_mask),
        Some(ChoiceEvent::Changed | ChoiceEvent::Unchanged) => update(state, dirty_mask),
        None => {
            close(state, dirty_mask);
            return false;
        }
    }
    true
}

/// 【形状标记】【滚动面板】把 delta 的整步滚动应用到 state 的网格；dirty_mask 返回重绘显示器，返回是否消费输入
pub fn handle_scroll(state: &mut EditorState, delta: f32, dirty_mask: &mut u32) -> bool {
    if !state.marker_popover.open {
        return false;
    }
    if state.input.pointer.monitor_idx == state.marker_popover.monitor_idx
        && state.marker_popover.hit_test(state.input.pointer.local)
    {
        // 1. 【形状标记】【滚动面板】属性输入向上为正，网格首行则向下递增
        state.marker_popover.scroll(-delta);
        pointer_move(state, dirty_mask);
    }
    true
}

/// 【形状标记】【按钮状态】同步 state 中形状选择按钮的打开状态；dirty_mask 返回按钮面板重绘区域
fn sync_button(state: &mut EditorState, dirty_mask: &mut u32) {
    let Some(index) = state.settings_panel.widgets.iter().position(|widget| {
        matches!(
            widget,
            SettingsWidget::Action {
                action: SettingsAction::MarkerShapes,
                ..
            }
        )
    }) else {
        return;
    };
    let selected = if state.marker_popover.open {
        Some(index)
    } else {
        state
            .settings_panel
            .selected
            .filter(|selected| *selected != index)
    };
    if selected == state.settings_panel.selected {
        return;
    }
    state.settings_panel.selected = selected;
    state.settings_panel.dirty = true;
    if let Some(rect) = state.settings_panel.rect() {
        emit_panel_damage(
            rect,
            state.settings_panel.monitor_idx,
            &mut state.damage_rects,
            dirty_mask,
        );
    }
}
