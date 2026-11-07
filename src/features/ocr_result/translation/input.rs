//! 【OCR 翻译】【编辑拥有者】三类文字输入与单元格分别分派，忙碌译文只允许导航、选择与复制

use super::super::{
    control_input::FocusTarget,
    layout::contains,
    window::{ResultWindow, View},
};
use crate::{
    backend::window::{KeyInput, WindowAction},
    keys::Key,
    ui::text_editor::{EditEvent, TextEditor},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Field {
    Source,
    Translation,
    Language,
    Table,
}

/// 【OCR 翻译】【字段查找】返回 window 当前具有焦点的可见编辑字段，不以页面替代译文或语言身份
pub(crate) fn field(window: &ResultWindow) -> Option<Field> {
    match (window.view, window.focus_target) {
        (View::Text, FocusTarget::Content) => Some(Field::Source),
        (View::Text, FocusTarget::Translation) if window.translation.state.visible => {
            Some(Field::Translation)
        }
        (View::Text, FocusTarget::Language) if window.translation.state.visible => {
            Some(Field::Language)
        }
        (View::Table, FocusTarget::Content) if window.table.as_ref()?.editor.is_some() => {
            Some(Field::Table)
        }
        _ => None,
    }
}

/// 【OCR 翻译】【编辑资格】按 window 和 field 返回修改资格，翻译期间只冻结译文和目标语言，不冻结原文
pub(crate) fn editable(window: &ResultWindow, field: Field) -> bool {
    !matches!(field, Field::Translation | Field::Language) || !window.translation.state.busy()
}

/// 【OCR 翻译】【共享编辑器】按 window 和 field 返回只读编辑器借用，表格没有草稿时返回空值
pub(crate) fn editor(window: &ResultWindow, field: Field) -> Option<&TextEditor> {
    Some(match field {
        Field::Source => &window.source,
        Field::Translation => &window.translation.editor,
        Field::Language => &window.translation.language,
        Field::Table => window.table.as_ref()?.editor.as_ref()?,
    })
}

/// 【OCR 翻译】【组合编辑】按 window、field 执行 operation，返回操作结果，各编辑器与 fonts 分别借用，不复制编辑历史
pub(crate) fn with_editor<T>(
    window: &mut ResultWindow,
    field: Field,
    operation: impl FnOnce(&mut TextEditor, &mut cosmic_text::FontSystem) -> T,
) -> Option<T> {
    let editor = match field {
        Field::Source => &mut window.source,
        Field::Translation => &mut window.translation.editor,
        Field::Language => &mut window.translation.language,
        Field::Table => window.table.as_mut()?.editor.as_mut()?,
    };
    Some(operation(editor, &mut window.fonts))
}

/// 【OCR 翻译】【语言按键】按 input 处理 window 语言确认、选项和弹出入口，返回已经消费的窗口动作，普通编辑返回空值
pub(crate) fn language_key(window: &mut ResultWindow, input: &KeyInput) -> Option<WindowAction> {
    if field(window) != Some(Field::Language) {
        return None;
    }
    if window.translation.state.busy() {
        return Some(WindowAction::None);
    }
    if input.key == Some(Key::Enter) && !input.mods.ctrl && !input.mods.alt && !input.mods.logo {
        super::controller::commit_target(window);
        return Some(WindowAction::Redraw);
    }
    if input.key == Some(Key::F(4)) || input.mods.alt && input.key == Some(Key::Down) {
        return Some(super::language_menu::open(window));
    }
    if input.mods == crate::keys::Mods::default() && matches!(input.key, Some(Key::Up | Key::Down))
    {
        let current = crate::translation::language::OPTIONS
            .iter()
            .position(|value| *value == window.translation.target());
        let next = match input.key {
            Some(Key::Up) => current.unwrap_or(0).saturating_sub(1),
            _ => current.map_or(0, |index| (index + 1).min(8)),
        };
        return Some(super::controller::choose(window, next));
    }
    None
}

/// 【OCR 翻译】【字段按键】按 input 处理 window 当前编辑器，返回编辑事件；只读译文允许选区复制及方向导航，修改键不调用编辑器
pub(crate) fn key(window: &mut ResultWindow, input: &KeyInput) -> EditEvent {
    let Some(field) = field(window) else {
        return EditEvent::None;
    };
    // 1. 【OCR 翻译】【按键一致】多行字段不继承共享编辑器的 Ctrl+Y 重做别名
    if matches!(field, Field::Source | Field::Translation)
        && input.mods.ctrl
        && !input.mods.alt
        && !input.mods.logo
        && input.key == Some(Key::Char('y'))
    {
        return EditEvent::None;
    }
    if !editable(window, field) {
        let copying =
            input.mods.ctrl && !input.mods.alt && matches!(input.key, Some(Key::Char('a' | 'c')));
        let navigation = !input.mods.alt
            && !input.mods.logo
            && matches!(
                input.key,
                Some(
                    Key::Left
                        | Key::Right
                        | Key::Up
                        | Key::Down
                        | Key::Home
                        | Key::End
                        | Key::PageUp
                        | Key::PageDown
                )
            );
        if field == Field::Language || !(copying || navigation) {
            return EditEvent::None;
        }
    }
    window.ensure_fonts(&input.text);
    with_editor(window, field, |editor, fonts| editor.key(fonts, input)).unwrap_or(EditEvent::None)
}

/// 【OCR 翻译】【正文选择】按 window、position、drag 和 clicks 分派本地坐标，返回是否消费，语言字段使用自己的横向偏移
pub(crate) fn select(
    window: &mut ResultWindow,
    position: (f64, f64),
    drag: bool,
    clicks: u8,
) -> bool {
    let Some(layout) = window.layout.translation.as_ref() else {
        return false;
    };
    let (target, rect, offset) = match window.focus_target {
        FocusTarget::Translation => (Field::Translation, layout.editor, 0.0),
        FocusTarget::Language => (
            Field::Language,
            layout.language_text,
            window.translation.language_offset,
        ),
        _ => return false,
    };
    with_editor(window, target, |editor, fonts| {
        editor.resize(fonts, rect.width(), rect.height());
        editor.pointer(
            fonts,
            (
                position.0 - f64::from(rect.x() - offset),
                position.1 - f64::from(rect.y()),
            ),
            drag,
            clicks,
        );
    });
    true
}

/// 【OCR 翻译】【字段按下】按 position 设置 window 的译文、语言或分栏拥有者，返回已消费动作，不把右键当作左键事务
pub(crate) fn press(
    window: &mut ResultWindow,
    position: (f64, f64),
    clicks: u8,
) -> Option<WindowAction> {
    let layout = window.layout.translation.as_ref()?;
    let target = if contains(layout.splitter, position) {
        window.translation.splitter_drag = true;
        window.dragging = false;
        return Some(WindowAction::Redraw);
    } else if contains(layout.language, position) && !window.translation.state.busy() {
        FocusTarget::Language
    } else if contains(layout.pane, position) && contains(layout.frame, position) {
        FocusTarget::Translation
    } else {
        return None;
    };
    window.focus_control(target);
    window.dragging = true;
    select(window, position, false, clicks);
    Some(WindowAction::Redraw)
}

/// 【OCR 翻译】【分栏拖动】按 position 更新 window 当前方向的比例，返回是否消费，改变方向或收起不继续旧拖动
pub(crate) fn split_motion(window: &mut ResultWindow, position: (f64, f64)) -> bool {
    if !window.translation.splitter_drag {
        return false;
    }
    let Some(layout) = &window.layout.translation else {
        window.translation.splitter_drag = false;
        return false;
    };
    let (point, origin, extent) = if layout.horizontal {
        (
            position.0 as f32,
            window.layout.body.x(),
            window.layout.body.width(),
        )
    } else {
        (
            position.1 as f32,
            window.layout.body.y(),
            window.layout.body.height(),
        )
    };
    window.translation.ratio = ((point - origin - 4.0) / (extent - 8.0)).clamp(0.0, 1.0);
    window.translation.layout_dirty = true;
    true
}
