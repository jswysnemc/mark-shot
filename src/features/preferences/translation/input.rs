//! 【翻译设置】【事件处理】处理字段提交、模型菜单、滚动及异步结果

use super::super::{
    dropdown::{Dropdown, OptionItem, Target, layout::Layout as DropdownLayout},
    window::{Preferences, contains},
};
use super::{
    fields::{Field, SERVICES},
    layout::{Hit, Layout},
};
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowEvent},
    keys::Key,
    ui::scrollbar,
};

/// 【翻译设置】【提交反馈】按 result 更新 state 提示，返回是否成功；同步父页配置且不记录密钥
fn feedback(state: &mut Preferences, result: Result<(), String>) -> bool {
    match result {
        Ok(()) => {
            state.status = if state.chinese() {
                "已保存"
            } else {
                "Saved"
            }
            .into();
            state.values["translation"] = state.translation.root.clone();
            true
        }
        Err(error) => {
            state.status = if let Some((chinese, english)) = error.split_once(" / ") {
                if state.chinese() {
                    chinese.into()
                } else {
                    english.into()
                }
            } else {
                error
            };
            false
        }
    }
}

/// 【翻译设置】【编辑提交】保存 state 当前编辑文字，返回成功状态，错误保留可修改输入
pub(in crate::features::preferences) fn apply(state: &mut Preferences) -> bool {
    let Some((field, editor)) = &state.translation.editing else {
        return true;
    };
    let (field, text) = (*field, editor.text());
    let original = state.translation.value(field);
    let original = original
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| original.to_string());
    if text == original {
        state.translation.editing = None;
        state.translation.reveal = false;
        return true;
    }
    let result = state.translation.save(&state.path, field, &text);
    feedback(state, result)
}

/// 【翻译设置】【模型菜单】按 state 已查询目录显示选择列表，无返回值，不触发额外网络请求
fn model_menu(state: &mut Preferences) {
    if !state.translation.models.is_empty() {
        open_menu(state, Field::Model);
    }
}

/// 【翻译设置】【下拉选项】按 field 构造 state 的语言、服务或模型浮层，保留未收录的当前语言
fn open_menu(state: &mut Preferences, field: Field) {
    use crate::translation::language;
    let layout = Layout::new(state);
    let visible_field = if field == Field::Model {
        Field::Models
    } else {
        field
    };
    state.translation.scroll = state
        .translation
        .scroll
        .saturating_add_signed(layout.reveal(visible_field))
        .min(layout.maximum);
    let layout = Layout::new(state);
    let chinese = state.chinese();
    let current = state
        .translation
        .value(field)
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let (options, selected, hit) = match field {
        Field::Provider => (
            SERVICES
                .iter()
                .enumerate()
                .map(|(index, service)| {
                    let (label, detail) = service.caption(chinese);
                    OptionItem::new(index.to_string(), label, detail)
                })
                .collect(),
            state.translation.selected.to_string(),
            Hit::Field(Field::Provider),
        ),
        Field::Model => (
            state
                .translation
                .models
                .iter()
                .map(|model| OptionItem::new(model, model, ""))
                .collect(),
            current,
            Hit::Models,
        ),
        Field::Target => {
            let labels = language::labels(language::UiLanguage::Chinese);
            let canonical = language::canonical(&current);
            let selected = language::OPTIONS
                .iter()
                .find(|option| canonical.is_some() && language::canonical(option) == canonical)
                .copied()
                .unwrap_or(&current)
                .to_owned();
            let mut options: Vec<_> = language::OPTIONS
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    OptionItem::new(
                        *value,
                        if chinese { labels[index] } else { value },
                        if chinese { value } else { labels[index] },
                    )
                })
                .collect();
            if !options.iter().any(|option| option.value == selected) {
                options.insert(
                    0,
                    OptionItem::new(
                        &current,
                        &current,
                        if chinese {
                            "当前自定义语言"
                        } else {
                            "Current custom language"
                        },
                    ),
                );
            }
            (options, selected, Hit::Field(Field::Target))
        }
        _ => return,
    };
    let anchor = layout
        .controls
        .iter()
        .find(|(control, _)| *control == hit)
        .map(|(_, bounds)| *bounds)
        .unwrap_or(super::layout::rect(
            layout.viewport.left() + 14.0,
            layout.viewport.top() + 44.0,
            layout.viewport.width() - 42.0,
            32.0,
        ));
    let mut menu = Dropdown::new(
        Target::Translation(field),
        anchor,
        options,
        &selected,
        &mut state.fonts,
    );
    menu.reveal(DropdownLayout::new(&menu, state.size).capacity);
    state.search_focus = false;
    state.dropdown = Some(menu);
}

/// 【翻译设置】【下拉提交】将 field 对应的稳定 value 保存到 state 配置，返回保存结果
pub(in crate::features::preferences) fn choose(
    state: &mut Preferences,
    field: Field,
    value: &str,
) -> bool {
    let result = if field == Field::Provider {
        let Some(index) = value
            .parse::<usize>()
            .ok()
            .filter(|index| *index < SERVICES.len())
        else {
            return false;
        };
        state.translation.select(&state.path, index)
    } else {
        state.translation.save(&state.path, field, value)
    };
    let success = feedback(state, result);
    if success && field == Field::Provider {
        state.status = if state.chinese() {
            "正在查看服务配置；点击启用后才切换翻译服务"
        } else {
            "Viewing configuration; enable to change the active service"
        }
        .into();
    }
    success
}

/// 【翻译设置】【文本定位】按 position 和 drag 更新当前输入选区，返回是否消费，不定位遮蔽字段
fn edit_pointer(
    state: &mut Preferences,
    layout: &Layout,
    position: (f64, f64),
    drag: bool,
) -> bool {
    let Some((field, editor)) = &mut state.translation.editing else {
        return false;
    };
    if field.secret() && !state.translation.reveal {
        return false;
    }
    let Some((_, bounds)) = layout
        .controls
        .iter()
        .find(|(hit, _)| *hit == Hit::Field(*field))
    else {
        return false;
    };
    if !drag && !contains(*bounds, position) {
        return false;
    }
    editor.pointer(
        &mut state.fonts,
        (
            position.0 - f64::from(bounds.left() + 8.0),
            position.1 - f64::from(bounds.top() + 3.0),
        ),
        drag,
    );
    true
}

/// 【翻译设置】【控件执行】按 hit 修改 state，保存成功才切换服务或退出字段编辑
fn activate(state: &mut Preferences, hit: Hit) {
    match hit {
        Hit::Enable | Hit::Auto => {
            if apply(state) {
                let result = state.translation.enable(&state.path, hit == Hit::Auto);
                feedback(state, result);
            }
        }
        Hit::Apply => {
            apply(state);
        }
        Hit::Reveal => state.translation.reveal = !state.translation.reveal,
        Hit::Query => {
            if state.translation.task.take().is_some() {
                state.status = if state.chinese() {
                    "已取消模型查询"
                } else {
                    "Model query canceled"
                }
                .into();
            } else if apply(state) {
                let result = state.translation.query();
                if feedback(state, result) {
                    state.status = if state.chinese() {
                        "正在查询模型…"
                    } else {
                        "Loading models…"
                    }
                    .into();
                }
            }
        }
        Hit::Models => {
            if apply(state) {
                model_menu(state);
            }
        }
        Hit::Field(field) => {
            if state
                .translation
                .editing
                .as_ref()
                .is_some_and(|(active, _)| *active == field)
            {
                return;
            }
            if !apply(state) {
                return;
            }
            match field {
                Field::Provider | Field::Target => open_menu(state, field),
                Field::Filter | Field::Disable => {
                    let value = !state.translation.value(field).as_bool().unwrap_or(false);
                    let result = state
                        .translation
                        .save(&state.path, field, &value.to_string());
                    feedback(state, result);
                }
                _ => {
                    let layout = Layout::new(state);
                    state.translation.scroll = state
                        .translation
                        .scroll
                        .saturating_add_signed(layout.reveal(field))
                        .min(layout.maximum);
                    state.translation.edit(field, &mut state.fonts);
                }
            }
        }
    }
}

/// 【翻译设置】【滚动更新】按 offset 更新 state 当前列表，返回空值
fn offset(state: &mut Preferences, offset: usize) {
    state.translation.scroll = offset;
}

/// 【翻译设置】【页面事件】按 event 更新 state，返回消费结果；窗口导航与搜索继续交给父窗口
pub(in crate::features::preferences) fn event(
    state: &mut Preferences,
    event: &WindowEvent,
) -> Option<WindowAction> {
    let layout = Layout::new(state);
    match event {
        WindowEvent::Wake => {
            let result = state
                .translation
                .task
                .as_ref()
                .and_then(|task| task.finished())?;
            state.translation.task = None;
            match result {
                Ok(models) => {
                    state.status = format!(
                        "{}: {}",
                        if state.chinese() {
                            "获取模型数量"
                        } else {
                            "Models received"
                        },
                        models.len()
                    );
                    state.translation.models = models;
                    if state.translation_page()
                        && state.translation.editing.is_none()
                        && state.dropdown.is_none()
                        && !state.search_focus
                    {
                        model_menu(state);
                    }
                }
                Err(error) => state.status = error.to_string(),
            }
        }
        WindowEvent::Resize { size } => {
            state.size = *size;
            let layout = Layout::new(state);
            let current = state.translation.scroll;
            offset(state, current.min(layout.maximum));
            state.translation.drag = None;
            state.translation.pressed = None;
            return None;
        }
        WindowEvent::Focused(false) => {
            state.translation.drag = None;
            state.translation.pressed = None;
            state.translation.reveal = false;
            return None;
        }
        WindowEvent::PointerMove { position } => {
            let changed = layout.hit(state.pointer) != layout.hit(*position);
            state.pointer = *position;
            if matches!(state.translation.pressed, Some(Hit::Field(_)))
                && edit_pointer(state, &layout, *position, true)
            {
                return Some(WindowAction::Redraw);
            }
            if let Some(drag) = state.translation.drag
                && let Some(bar) = &layout.scrollbar
            {
                offset(state, drag.value(*position, bar));
            } else if !contains(layout.viewport, *position) {
                return None;
            } else if !changed {
                return Some(WindowAction::None);
            }
        }
        WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed,
        } => {
            state.pointer = *position;
            if !pressed && state.translation.drag.take().is_some() {
                return Some(WindowAction::Redraw);
            }
            if !contains(layout.viewport, *position) {
                state.translation.pressed = None;
                return None;
            }
            state.search_focus = false;
            if *pressed {
                if let Some(bar) = &layout.scrollbar
                    && contains(bar.track, *position)
                {
                    if contains(bar.handle, *position) {
                        state.translation.drag = Some(scrollbar::Drag::new(*position, bar));
                    } else {
                        offset(
                            state,
                            if position.1 < bar.handle.top() as f64 {
                                0
                            } else {
                                bar.maximum
                            },
                        );
                    }
                } else {
                    state.translation.pressed = layout.hit(*position);
                    edit_pointer(state, &layout, *position, false);
                }
            } else if let Some(hit) = state
                .translation
                .pressed
                .take()
                .filter(|hit| Some(*hit) == layout.hit(*position))
            {
                activate(state, hit);
            }
        }
        WindowEvent::Scroll { position, delta } if contains(layout.viewport, *position) => {
            let step = 64;
            let current = state.translation.scroll;
            offset(
                state,
                current
                    .saturating_add_signed(if delta.1 > 0.0 {
                        step
                    } else if delta.1 < 0.0 {
                        -step
                    } else {
                        0
                    })
                    .min(layout.maximum),
            );
        }
        WindowEvent::Key(input) if !state.search_focus => {
            if input.mods.ctrl && input.key == Some(Key::Char('f')) {
                return None;
            }
            if input.key == Some(Key::Tab) {
                if apply(state) {
                    let controls: Vec<_> = Layout::new(state)
                        .controls
                        .into_iter()
                        .map(|(hit, _)| hit)
                        .collect();
                    let next = state
                        .translation
                        .focus
                        .and_then(|hit| controls.iter().position(|entry| *entry == hit))
                        .map_or(
                            if input.mods.shift {
                                controls.len() - 1
                            } else {
                                0
                            },
                            |index| {
                                if input.mods.shift {
                                    (index + controls.len() - 1) % controls.len()
                                } else {
                                    (index + 1) % controls.len()
                                }
                            },
                        );
                    state.translation.focus = Some(controls[next]);
                    if let Hit::Field(field) = controls[next] {
                        state.translation.scroll = state
                            .translation
                            .scroll
                            .saturating_add_signed(layout.reveal(field))
                            .min(layout.maximum);
                    }
                }
                return Some(WindowAction::Redraw);
            }
            if input.key == Some(Key::Enter)
                && state.translation.editing.is_none()
                && let Some(hit) = state.translation.focus
            {
                activate(state, hit);
                return Some(WindowAction::Redraw);
            }
            if state.translation.editing.is_some() {
                match input.key {
                    Some(Key::Escape) => {
                        state.translation.editing = None;
                        state.translation.reveal = false;
                    }
                    Some(Key::Enter) => {
                        apply(state);
                    }
                    Some(Key::Tab) => {
                        let field = state.translation.editing.as_ref().unwrap().0;
                        if apply(state) {
                            let fields = state.translation.fields();
                            let index =
                                (fields.iter().position(|entry| *entry == field).unwrap_or(0) + 1)
                                    % fields.len();
                            state.translation.scroll = state
                                .translation
                                .scroll
                                .saturating_add_signed(layout.reveal(fields[index]))
                                .min(layout.maximum);
                            activate(
                                state,
                                if fields[index] == Field::Models {
                                    Hit::Models
                                } else {
                                    Hit::Field(fields[index])
                                },
                            );
                        }
                    }
                    _ => {
                        state
                            .translation
                            .editing
                            .as_mut()
                            .unwrap()
                            .1
                            .key(&mut state.fonts, input);
                    }
                }
            } else {
                let next = match input.key {
                    Some(Key::PageDown) => state.translation.scroll + layout.capacity,
                    Some(Key::PageUp) => state.translation.scroll.saturating_sub(layout.capacity),
                    Some(Key::Home) => 0,
                    Some(Key::End) => layout.maximum,
                    Some(Key::Tab) => {
                        activate(state, Hit::Field(Field::Provider));
                        return Some(WindowAction::Redraw);
                    }
                    _ => return None,
                };
                state.translation.scroll = next.min(layout.maximum);
            }
        }
        _ => return None,
    }
    Some(WindowAction::Redraw)
}
