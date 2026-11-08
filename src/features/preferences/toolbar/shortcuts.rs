//! 【工具栏设置】【快捷键编辑】选中工具后展示并录入绑定，复用全局冲突检查

use super::super::{shortcuts, window::Preferences};
use crate::{backend::window::KeyInput, keys::Key, ui::toolbar::catalog};

/// 【工具栏设置】【快捷键字段】从 state 的当前项目返回动作键，分隔线返回空值
pub(super) fn key(state: &Preferences) -> Option<&'static str> {
    let items = state.toolbar.profile.items(&state.toolbar.config);
    catalog::entry(*items.get(state.toolbar.selected?)?)?.shortcut()
}

/// 【工具栏设置】【快捷键文字】按 state 返回当前绑定或录入提示，无副作用
pub(super) fn label(state: &Preferences) -> String {
    if state.toolbar.recording.is_some() {
        return if state.chinese() {
            "按下快捷键…"
        } else {
            "Press shortcut…"
        }
        .into();
    }
    let text = key(state)
        .map(|key| shortcuts::text(&state.values, key))
        .unwrap_or_default();
    if text.is_empty() {
        if state.chinese() {
            "设置快捷键"
        } else {
            "Set shortcut"
        }
        .into()
    } else {
        text
    }
}

/// 【工具栏设置】【快捷键输入】按 input 追加 state 绑定，返回是否消费；Esc 取消录入
pub(super) fn input(state: &mut Preferences, input: &KeyInput) -> bool {
    let Some(key) = state.toolbar.recording else {
        return false;
    };
    if input.key == Some(Key::Escape) {
        state.toolbar.recording = None;
        return true;
    }
    let value = {
        let Some(chord) = shortcuts::chord(input) else {
            return true;
        };
        let mut bindings: Vec<serde_json::Value> = match &state.values["keys"][key] {
            serde_json::Value::Array(values) => values.clone(),
            serde_json::Value::String(text) if !text.is_empty() => vec![text.clone().into()],
            _ => shortcuts::defaults(key),
        };
        if !bindings.iter().any(|value| value.as_str() == Some(&chord)) {
            bindings.push(chord.into());
        }
        bindings.into()
    };
    if state.save_field("keys", key, value) {
        state.toolbar.recording = None;
    }
    true
}
