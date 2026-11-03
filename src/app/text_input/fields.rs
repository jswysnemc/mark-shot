//! 【字段输入法】【输入衔接】按当前字段分配目标，原生确认整批验证，候选不进入过滤或颜色应用

use crate::{
    app::panels,
    editor::{EditorState, dirty::apply_damage_rects, edits::damage_color_popover},
    features::text::input_method::surrounding,
    keys::{Chord, Key},
    renderer::line_edit,
    theme::{font, size},
    types::text_input::{ContentType, Edit, FieldLayout, Focus, Snapshot},
    ui::{
        font_panel::state::Field,
        panel::{UiPanel, emit_panel_damage},
        settings_panel::SettingsWidget,
        text_field::{LineEditState, validation::Rule},
    },
};
use cosmic_text::Weight;
use tiny_skia::Rect;

#[derive(Clone, Copy)]
enum Owner {
    Search,
    Size,
    Hex,
    Stepper(usize),
}

/// 【字段输入法】【当前拥有者】按 state 的可见字段返回输入拥有者；菜单、隐藏界面和没有编辑字段时返回空值
fn owner(state: &EditorState) -> Option<Owner> {
    if state.ui_hidden || state.text_context_menu.is_some() {
        return None;
    }
    if state.font_panel.open {
        return state
            .font_panel
            .fields
            .editing
            .as_ref()
            .map(|edit| match edit.key {
                Field::Search => Owner::Search,
                Field::Size => Owner::Size,
            });
    }
    if state.color_popover.is_visible() {
        return state
            .color_popover
            .fields
            .editing
            .as_ref()
            .filter(|edit| {
                edit.key == crate::ui::color_popover::ColorField::Hex
                    && state.color_popover.hex_field_rect().is_some()
            })
            .map(|_| Owner::Hex);
    }
    let edit = state.settings_panel.fields.editing.as_ref()?;
    (state.settings_panel.visible
        && matches!(
            state.settings_panel.widgets.get(edit.key),
            Some(SettingsWidget::Stepper { .. })
        ))
    .then_some(Owner::Stepper(edit.key))
}

/// 【字段输入法】【身份读取】返回 state 当前字段的独立编辑身份，没有可见字段时返回空值
pub(super) fn identity(state: &EditorState) -> Option<u64> {
    match owner(state)? {
        Owner::Search | Owner::Size => state.font_panel.fields.editing.as_ref().map(|edit| edit.id),
        Owner::Hex => state
            .color_popover
            .fields
            .editing
            .as_ref()
            .map(|edit| edit.id),
        Owner::Stepper(_) => state
            .settings_panel
            .fields
            .editing
            .as_ref()
            .map(|edit| edit.id),
    }
}

/// 【字段输入法】【表面归属】返回当前字段所在显示器，未显示输入字段时返回空值；不能在另一个聚焦表面启用旧目标
pub(super) fn monitor(state: &EditorState) -> Option<usize> {
    Some(match owner(state)? {
        Owner::Search | Owner::Size => state.font_panel.monitor_idx,
        Owner::Hex => state.color_popover.monitor_idx,
        Owner::Stepper(_) => state.settings_panel.monitor_idx,
    })
}

/// 【字段输入法】【验证选择】按 owner 返回整批验证规则，不以单字符过滤代替输入法确认
fn rule(owner: Owner) -> Rule {
    match owner {
        Owner::Search => Rule::Text,
        Owner::Size => Rule::Integer {
            min: 8,
            max: 300,
            suffix: " pt",
        },
        Owner::Hex => Rule::Hex,
        Owner::Stepper(_) => Rule::Number,
    }
}

/// 【字段输入法】【字段借用】返回 state 中 owner 对应的可变编辑器，字段已经结束时返回空值
fn field_mut(state: &mut EditorState, owner: Owner) -> Option<&mut LineEditState> {
    match owner {
        Owner::Search | Owner::Size => state
            .font_panel
            .fields
            .editing
            .as_mut()
            .map(|edit| &mut edit.field),
        Owner::Hex => state
            .color_popover
            .fields
            .editing
            .as_mut()
            .map(|edit| &mut edit.field),
        Owner::Stepper(_) => state
            .settings_panel
            .fields
            .editing
            .as_mut()
            .map(|edit| &mut edit.field),
    }
}

/// 【字段输入法】【显示通知】按 state、owner 和 text_changed 更新过滤或数值，dirty 返回显示区域；颜色只在字段确认时应用
fn notice(state: &mut EditorState, owner: Owner, text_changed: bool, dirty: &mut u32) {
    match owner {
        Owner::Search | Owner::Size => {
            if text_changed {
                panels::text::keyboard::apply(state, dirty);
            } else {
                state.font_panel.dirty = true;
                panels::text::font_popup::update(state, dirty);
            }
        }
        Owner::Hex => damage_color_popover(state, dirty),
        Owner::Stepper(index) => {
            if text_changed {
                panels::settings::live_apply_stepper_field(state, index, dirty);
            }
            state.settings_panel.dirty = true;
            if let Some(rect) = state.settings_panel.rect() {
                emit_panel_damage(
                    rect,
                    state.settings_panel.monitor_idx,
                    &mut state.damage_rects,
                    dirty,
                );
            }
        }
    }
    apply_damage_rects(state, dirty);
}

/// 【字段输入法】【批次应用】将 event 交给 state 当前字段，dirty 返回变化；返回是否存在输入拥有者，非法确认不能穿透到标注正文
pub(in crate::app) fn apply(state: &mut EditorState, event: Edit, dirty: &mut u32) -> bool {
    let Some(owner) = owner(state) else {
        return false;
    };
    let Some(field) = field_mut(state, owner) else {
        return false;
    };
    let update = field.input_method_update(event, rule(owner));
    if update.changed {
        notice(state, owner, update.text_changed, dirty);
    }
    true
}

/// 【字段输入法】【候选取消】清理 state 所有仍在编辑的字段候选，dirty 返回可见区域；不恢复已经删除的正文，无返回值
pub(super) fn cancel(state: &mut EditorState, dirty: &mut u32) {
    let font = state
        .font_panel
        .fields
        .editing
        .as_mut()
        .is_some_and(|edit| edit.field.cancel_preedit());
    let color = state
        .color_popover
        .fields
        .editing
        .as_mut()
        .is_some_and(|edit| edit.field.cancel_preedit());
    let stepper = state
        .settings_panel
        .fields
        .editing
        .as_mut()
        .and_then(|edit| edit.field.cancel_preedit().then_some(edit.key));
    if font {
        notice(state, Owner::Search, false, dirty);
    }
    if color {
        notice(state, Owner::Hex, false, dirty);
    }
    if let Some(index) = stepper {
        notice(state, Owner::Stepper(index), false, dirty);
    }
}

/// 【字段输入法】【局部撤销】按 state 当前字段处理 chord 的撤销重做，dirty 返回区域；无可撤销动作也消费快捷键，不能穿透标注历史
pub(crate) fn key(state: &mut EditorState, chord: Option<Chord>, dirty: &mut u32) -> bool {
    let Some(owner) = owner(state) else {
        return false;
    };
    let Some(chord) = chord.filter(|chord| chord.mods.ctrl) else {
        return false;
    };
    let redo = match chord.key {
        Key::Char('z') => chord.mods.shift,
        Key::Char('y') => true,
        _ => return false,
    };
    let changed =
        field_mut(state, owner).is_some_and(|field| if redo { field.redo() } else { field.undo() });
    notice(state, owner, changed, dirty);
    true
}

/// 【字段输入法】【字体字段区域】返回 state 中字体字段的实际文本矩形和字形参数，保持绘制与协议位置一致
fn font_rect(state: &EditorState, owner: Owner) -> Option<Rect> {
    let panel = &state.font_panel;
    let local = match owner {
        Owner::Search => Rect::from_xywh(
            panel.layout.search.left() + 9.0,
            panel.layout.search.top(),
            panel.layout.search.width() - 36.0,
            panel.layout.search.height(),
        )?,
        Owner::Size => Rect::from_xywh(
            panel.layout.size_field.left() + 4.0,
            panel.layout.size_field.top(),
            panel.layout.size_field.width() - 22.0,
            26.0,
        )?,
        _ => return None,
    };
    Rect::from_xywh(
        local.left() + panel.render_pos.0,
        local.top() + panel.render_pos.1,
        local.width(),
        local.height(),
    )
}

/// 【字段输入法】【快照生成】按 state、focus 和 target_id 返回实际变化后的有限快照；字形缓存只按字段版本与字体变化更新，闪烁或静止不复制全文
pub(super) fn snapshot(state: &mut EditorState, focus: Focus, target_id: u64) -> Option<Snapshot> {
    let owner = owner(state)?;
    let (rect, size, weight, monitor) = match owner {
        Owner::Search | Owner::Size => (
            font_rect(state, owner)?,
            16.0,
            Weight::NORMAL,
            state.font_panel.monitor_idx,
        ),
        Owner::Hex => (
            state.color_popover.hex_field_rect()?,
            12.0,
            Weight(600),
            state.color_popover.monitor_idx,
        ),
        Owner::Stepper(index) => {
            let item = state.settings_panel.widget_rect(index)?;
            let cfg = &crate::config::get().settings_panel;
            (
                Rect::from_xywh(
                    item.left() + size::padding(),
                    item.top(),
                    (item.width() - cfg.stepper_arrow_zone - size::padding()).max(1.0),
                    item.height(),
                )?,
                font::label(),
                if cfg.stepper_bold {
                    Weight::BOLD
                } else {
                    Weight::NORMAL
                },
                state.settings_panel.monitor_idx,
            )
        }
    };
    if monitor != focus.monitor_idx {
        return None;
    }
    let previous_layout = state.input.text_input.field_layout;
    let (field, fonts) = match owner {
        Owner::Search | Owner::Size => (
            &state.font_panel.fields.editing.as_ref()?.field,
            &mut state.text.font_system,
        ),
        Owner::Hex => (
            &state.color_popover.fields.editing.as_ref()?.field,
            &mut state
                .color_popover
                .picker_fonts
                .get_or_insert_with(crate::ui::color_popover::fonts::PickerFonts::new)
                .system,
        ),
        Owner::Stepper(_) => (
            &state.settings_panel.fields.editing.as_ref()?.field,
            &mut state.text.font_system,
        ),
    };
    let font_count = fonts.db().faces().count();
    let field_id = match owner {
        Owner::Search | Owner::Size => state.font_panel.fields.editing.as_ref()?.id,
        Owner::Hex => state.color_popover.fields.editing.as_ref()?.id,
        Owner::Stepper(_) => state.settings_panel.fields.editing.as_ref()?.id,
    };
    let cursor_x = match previous_layout.filter(|old| {
        old.field_id == field_id
            && old.revision == field.revision
            && old.font_size == size.to_bits()
            && old.weight == weight.0
            && old.font_count == font_count
    }) {
        Some(old) => old.cursor_x,
        None => line_edit::shape(field, fonts, size, weight, None).caret,
    };
    let layout = FieldLayout {
        field_id,
        revision: field.revision,
        font_size: size.to_bits(),
        weight: weight.0,
        font_count,
        cursor_x,
    };
    let anchor = field
        .selection_anchor
        .map_or(field.cursor_byte(), |anchor| {
            field
                .text
                .char_indices()
                .nth(anchor)
                .map_or(field.text.len(), |(byte, _)| byte)
        });
    let context = surrounding::view(&field.text, field.cursor_byte(), anchor);
    let cursor = if matches!(owner, Owner::Hex) {
        let shift = (cursor_x - (rect.width() - 22.0) + 2.0).max(0.0);
        (
            (rect.left() + 11.0 + cursor_x - shift).round() as i32,
            (rect.top() + 5.0).floor() as i32,
            1,
            16,
        )
    } else {
        line_edit::caret_rect(cursor_x, rect, true)
    };
    let content = match owner {
        Owner::Size => ContentType::Digits,
        Owner::Stepper(_) => ContentType::Number,
        _ => ContentType::Text,
    };
    let target = &state.input.text_input;
    let same = target.snapshot.as_ref().is_some_and(|old| {
        old.target_id == target_id
            && old.focus_id == focus.id
            && old.cursor_rect == Some(cursor)
            && old.content == content
            && match (context, old.surrounding.as_ref()) {
                (None, None) => true,
                (Some(context), Some(old)) => context.matches(old),
                _ => false,
            }
    });
    let next = (!same).then(|| Snapshot {
        target_id,
        focus_id: focus.id,
        monitor_idx: monitor,
        surrounding: context.map(surrounding::Window::owned),
        cursor_rect: Some(cursor),
        cause: target.cause,
        content,
    });
    state.input.text_input.field_layout = Some(layout);
    next
}
