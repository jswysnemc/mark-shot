//! 【快捷键】【当前提示】从已解析且去除冲突的映射生成可见快捷键

use super::{Action, Chord, Key, Keymap};

impl Keymap {
    /// 【快捷键】【动作提示】按 action 返回最短有效绑定，未绑定返回空值，不重新解释配置
    pub(crate) fn label(&self, action: Action) -> Option<String> {
        let mut labels: Vec<_> = self
            .binds
            .iter()
            .filter(|(_, value)| **value == action)
            .map(|(chord, _)| label(*chord))
            .collect();
        labels.sort_by_key(|label| (label.len(), label.clone()));
        labels.into_iter().next()
    }
}

/// 【快捷键】【按键名称】按 chord 返回包含修饰键的紧凑显示文本
fn label(chord: Chord) -> String {
    let mut parts = Vec::new();
    for (enabled, name) in [
        (chord.mods.ctrl, "Ctrl"),
        (chord.mods.alt, "Alt"),
        (chord.mods.shift, "Shift"),
        (chord.mods.logo, "Super"),
    ] {
        if enabled {
            parts.push(name.to_owned());
        }
    }
    parts.push(match chord.key {
        Key::Char(value) => value.to_ascii_uppercase().to_string(),
        Key::F(value) => format!("F{value}"),
        Key::Escape => "Esc".into(),
        Key::Enter => "Enter".into(),
        other => format!("{other:?}"),
    });
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【快捷键测试】【有效提示】自定义、禁用和冲突后的提示必须与实际分派一致
    #[test]
    fn labels_follow_resolved_bindings() {
        let overrides = std::collections::BTreeMap::from([
            ("cancel".into(), crate::config::Binding::One("F8".into())),
            ("copy".into(), crate::config::Binding::One("".into())),
            ("save".into(), crate::config::Binding::One("Return".into())),
        ]);
        let map = Keymap::build(&overrides);
        assert_eq!(map.label(Action::Cancel).as_deref(), Some("F8"));
        assert_eq!(map.label(Action::Finish(crate::types::Finish::Copy)), None);
        assert_eq!(map.label(Action::Finish(crate::types::Finish::Save)), None);
    }
}
