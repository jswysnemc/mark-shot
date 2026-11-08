//! 【字体弹层】【业务生命周期】管理弹层互斥、属性同步与鼠标输入，字体字形缓存只用于可见行

#[cfg(test)]
#[path = "font_popup_tests.rs"]
mod tests;

use crate::{
    editor::{
        EditorState,
        edits::active_annotation_idx,
        text_edits::{self, Property},
    },
    types::AnnotationShape,
    ui::{
        font_panel::{
            layout::FontLayout,
            state::{Field, Hit},
        },
        panel::{UiPanel, sync_panel_hover, sync_panel_rect},
        text_field::CursorInit,
        text_properties::layout::Control,
    },
};

/// 【字体弹层】【样式同步】将 state 当前文字样式写入弹层，无返回值，不覆盖工具默认属性
fn sync(state: &mut EditorState) {
    let tools = &state.tool_settings;
    let (family, bold, italic) = active_annotation_idx(state)
        .and_then(|index| match &state.annotations[index].shape {
            AnnotationShape::Text {
                appearance,
                bold,
                italic,
                ..
            } => Some((appearance.family.clone(), *bold, *italic)),
            _ => None,
        })
        .unwrap_or_else(|| (tools.text_family.clone(), tools.bold, tools.italic));
    let points = text_edits::current_points(state);
    let panel = &mut state.font_panel;
    if panel.family != family
        || panel.points != points
        || panel.bold != bold
        || panel.italic != italic
    {
        panel.dirty = true;
    }
    panel.family = family;
    panel.points = points;
    panel.bold = bold;
    panel.italic = italic;
}

/// 【字体弹层】【界面同步】按 state 属性栏重新定位弹层，dirty 返回新旧范围，无返回值
pub fn update(state: &mut EditorState, dirty: &mut u32) {
    if !state.font_panel.open {
        return;
    }
    if !state.settings_panel.visible
        || state.settings_panel.text_layout.is_none()
        || state.ui_hidden
        || state.tool_active
    {
        close(state, dirty);
        return;
    }
    let old = state.font_panel.rect();
    let monitor = state.font_panel.monitor_idx;
    sync(state);
    let properties = state.settings_panel.rect().unwrap();
    let button = state
        .settings_panel
        .widget_rect(Control::Font.index())
        .unwrap();
    let bounds = state.placements[state.settings_panel.monitor_idx].size;
    let bounds = (bounds.0 as f32, bounds.1 as f32);
    let rows = state
        .font_panel
        .catalog
        .as_ref()
        .map_or(10, |catalog| catalog.families.len());
    let layout = FontLayout::new(rows, bounds);
    state.font_panel.layout = layout;
    state.font_panel.render_pos = layout.position(properties, button, bounds);
    state.font_panel.monitor_idx = state.settings_panel.monitor_idx;
    let changed = state.font_panel.dirty;
    sync_panel_rect(
        &mut state.font_panel,
        old,
        monitor,
        changed,
        &mut state.damage_rects,
        dirty,
    );
}

/// 【字体弹层】【面板关闭】关闭 state 字体弹层并释放字形预览，dirty 返回原面板范围，无返回值
pub fn close(state: &mut EditorState, dirty: &mut u32) {
    let old = state.font_panel.rect();
    let monitor = state.font_panel.monitor_idx;
    state.font_panel.open = false;
    state.font_panel.fields.editing = None;
    state.font_panel.pressed = None;
    state.font_panel.scrollbar_drag = None;
    state.font_panel.preview.clear();
    state.font_panel.pixmap = None;
    sync_panel_rect(
        &mut state.font_panel,
        old,
        monitor,
        old.is_some(),
        &mut state.damage_rects,
        dirty,
    );
    if state.settings_panel.selected == Some(Control::Font.index()) {
        state.settings_panel.selected = None;
        state.settings_panel.dirty = true;
        if let Some(rect) = state.settings_panel.rect() {
            crate::ui::panel::emit_panel_damage(
                rect,
                state.settings_panel.monitor_idx,
                &mut state.damage_rects,
                dirty,
            );
        }
    }
}

/// 【字体弹层】【面板切换】切换 state 字体弹层，dirty 返回受影响区域，无返回值，打开时关闭其他属性弹层
pub fn toggle(state: &mut EditorState, dirty: &mut u32) {
    if state.font_panel.open {
        close(state, dirty);
        return;
    }
    super::super::color::close_color_popover(state, dirty);
    super::super::model::close_model_popover(state, dirty);
    super::super::marker::close(state, dirty);
    super::super::ruler::close(state, dirty);
    super::super::number::close(state, dirty);
    super::super::line::close(state, dirty);
    super::super::filter::close(state, dirty);
    sync(state);
    state.font_panel.begin();
    state.settings_panel.selected = Some(Control::Font.index());
    state.settings_panel.dirty = true;
    update(state, dirty);
    state.font_panel.center_current();
}

/// 【字体弹层】【焦点切换】将 state 面板焦点移到 field，cursor 决定插入位置，无返回值
pub fn focus(state: &mut EditorState, field: Field, cursor: CursorInit) {
    if let Some((key, text)) = state.font_panel.fields.commit_edit() {
        state.font_panel.fields.values.insert(key, text);
    }
    let text = if field == Field::Search {
        state.font_panel.query().to_owned()
    } else {
        format!("{} pt", state.font_panel.points.round() as i32)
    };
    state.font_panel.fields.begin_edit(field, text, cursor);
    if field == Field::Size
        && let Some(edit) = state.font_panel.fields.editing.as_mut()
    {
        let length = edit.field.text.trim_end_matches(" pt").chars().count();
        edit.field.cursor = match cursor {
            CursorInit::At(index) => index.min(length),
            _ => length,
        };
        edit.field.selection_anchor = matches!(cursor, CursorInit::SelectAll).then_some(0);
    }
    state.font_panel.dirty = true;
}

/// 【字体弹层】【字号缓冲】用 state 当前点字号替换完整字段文本并保持数值部分光标，无返回值；程序替换清理候选与历史
pub(crate) fn set_size_text(state: &mut EditorState) {
    let number = (text_edits::current_points(state).round() as i32).to_string();
    state
        .font_panel
        .fields
        .set_editing_text(Field::Size, format!("{number} pt"));
    if let Some(edit) = state.font_panel.fields.editing.as_mut()
        && edit.key == Field::Size
    {
        edit.field.cursor = number.chars().count();
    }
}

/// 【字体弹层】【鼠标按下】按 state 实际指针启动控件或滚动手势，dirty 返回范围，无返回值
pub fn press(state: &mut EditorState, dirty: &mut u32) {
    let hit = state.font_panel.hit(state.input.pointer.local);
    state.font_panel.pressed = hit;
    state.font_panel.dirty = true;
    match hit {
        Some(Hit::Search) => focus(state, Field::Search, CursorInit::End),
        Some(Hit::Size) => focus(state, Field::Size, CursorInit::SelectAll),
        Some(Hit::SizeUp | Hit::SizeDown) => {
            let delta = if hit == Some(Hit::SizeUp) { 1.0 } else { -1.0 };
            text_edits::set_control_points(state, state.font_panel.points + delta, dirty);
            sync(state);
            set_size_text(state);
        }
        Some(Hit::Scrollbar) => {
            if let Some(thumb) = state.font_panel.thumb() {
                let y = state.input.pointer.local.1 as f32 - state.font_panel.render_pos.1;
                if y >= thumb.top() && y < thumb.bottom() {
                    state.font_panel.scrollbar_drag = Some(y - thumb.top());
                } else {
                    state.font_panel.scroll(if y < thumb.top() {
                        -state.font_panel.layout.list.height()
                    } else {
                        state.font_panel.layout.list.height()
                    });
                }
            }
        }
        Some(Hit::ScrollUp) => {
            state.font_panel.scroll(-28.0);
        }
        Some(Hit::ScrollDown) => {
            state.font_panel.scroll(28.0);
        }
        _ => {}
    }
    update(state, dirty);
}

/// 【字体弹层】【鼠标释放】在 state 相同控件内确认动作，dirty 返回变化，返回是否消费，不同显示器或移出控件不应用
pub fn release(state: &mut EditorState, dirty: &mut u32) -> bool {
    let had_drag = state.font_panel.scrollbar_drag.take().is_some();
    // 1. 【字体弹层】【释放刷新】按下和拖动颜色立即失效，移出控件释放也不能保留原按下底板
    state.font_panel.dirty |= had_drag || state.font_panel.pressed.is_some();
    let Some(hit) = state.font_panel.pressed.take() else {
        update(state, dirty);
        return had_drag;
    };
    if state.input.pointer.monitor_idx != state.font_panel.monitor_idx
        || state.font_panel.hit(state.input.pointer.local) != Some(hit)
    {
        update(state, dirty);
        return true;
    }
    match hit {
        Hit::Family(index) => {
            let family = state
                .font_panel
                .catalog
                .as_ref()
                .and_then(|catalog| catalog.families.get(index))
                .map(|family| family.name.clone());
            if let Some(family) = family {
                text_edits::set(state, Property::Family(family), true, dirty);
            }
            focus(state, Field::Size, CursorInit::SelectAll);
        }
        Hit::Bold => text_edits::set(state, Property::Bold(!state.font_panel.bold), true, dirty),
        Hit::Italic => text_edits::set(
            state,
            Property::Italic(!state.font_panel.italic),
            true,
            dirty,
        ),
        Hit::Clear => {
            focus(state, Field::Search, CursorInit::End);
            state
                .font_panel
                .fields
                .set_editing_text(Field::Search, String::new());
            state.font_panel.filter();
        }
        _ => {}
    }
    super::super::settings::update_settings_panel(state, dirty);
    update(state, dirty);
    true
}

/// 【字体弹层】【指针移动】按 state 当前指针同步悬停与滚动拖动，dirty 返回范围，无返回值
pub fn pointer_move(state: &mut EditorState, dirty: &mut u32) {
    if !state.font_panel.open {
        return;
    }
    if let Some(offset) = state.font_panel.scrollbar_drag
        && let Some(thumb) = state.font_panel.thumb()
    {
        let list = state.font_panel.layout.list;
        let y = state.input.pointer.global.1 as f32
            - state.placements[state.font_panel.monitor_idx].position.1 as f32
            - state.font_panel.render_pos.1;
        let ratio =
            (y - offset - list.top() - 14.0) / (list.height() - 28.0 - thumb.height()).max(1.0);
        state.font_panel.offset =
            ((ratio.clamp(0.0, 1.0) * state.font_panel.max_offset() / 28.0).round() * 28.0)
                .clamp(0.0, state.font_panel.max_offset());
        state.font_panel.dirty = true;
    }
    let hovered = (state.input.pointer.monitor_idx == state.font_panel.monitor_idx)
        .then(|| state.font_panel.hit(state.input.pointer.local))
        .flatten();
    sync_panel_hover(
        &mut state.font_panel,
        hovered,
        &mut state.damage_rects,
        dirty,
    );
    update(state, dirty);
}

/// 【字体弹层】【滚轮输入】用 event 角度刻度调整 state 列表或字号，dirty 返回范围，返回是否阻止底层控件滚动；纯像素不改变原版按行列表
pub fn scroll(state: &mut EditorState, event: crate::types::ScrollEvent, dirty: &mut u32) -> bool {
    if !state.font_panel.open {
        return false;
    }
    state.commit_text_size_wheel();
    state
        .input
        .text_wheels
        .prepare(state.settings_panel.active_source);
    if state.input.pointer.monitor_idx == state.font_panel.monitor_idx
        && state.font_panel.hit_test(state.input.pointer.local)
    {
        if matches!(
            state.font_panel.hit(state.input.pointer.local),
            Some(Hit::Size | Hit::SizeUp | Hit::SizeDown)
        ) {
            let delta = state
                .input
                .text_wheels
                .spin
                .spin(event.angle_y(), state.input.ctrl);
            if delta != 0 {
                text_edits::set_control_points(
                    state,
                    state.font_panel.points + delta as f32,
                    dirty,
                );
                set_size_text(state);
            }
        } else if matches!(
            state.font_panel.hit(state.input.pointer.local),
            Some(Hit::Family(_) | Hit::Scrollbar | Hit::ScrollUp | Hit::ScrollDown)
        ) {
            let page = (state.font_panel.layout.list.height()
                / crate::ui::font_panel::layout::ROW_HEIGHT)
                .floor()
                .max(1.0) as i32;
            let delta = state.input.text_wheels.list.slider(
                event.angle_y(),
                state.input.ctrl || state.input.shift,
                page,
            );
            if delta != 0
                && !state
                    .font_panel
                    .scroll(-delta as f32 * crate::ui::font_panel::layout::ROW_HEIGHT)
            {
                state.input.text_wheels.list.reset();
            }
        }
        update(state, dirty);
    }
    true
}
