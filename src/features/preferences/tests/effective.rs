//! 【设置实际值回归】【原生事务】验证实际值显示、编辑、下拉继承及恢复，不将派生值误写入配置

use super::{Preferences, entry};
use crate::features::preferences::{catalog::ENTRIES, window::Control};

/// 【设置实际值回归】【无改动编辑】无参数，验证默认目录编辑时显示真实路径，失焦且无改动不会创建显式覆盖
#[test]
fn opening_and_leaving_inherited_directory_does_not_persist_the_effective_path() {
    let mut state = Preferences::new(false).unwrap();
    for path in [
        "history.directory",
        "recording.directory",
        "save.directory",
        "log.path",
    ] {
        let index = entry(path);
        state.page = ENTRIES[index].page;
        let shown = state.value(index);
        assert!(
            std::path::Path::new(&shown).is_absolute(),
            "{path}: {shown}"
        );
        state.activate(Control::Row(index));
        assert_eq!(state.editing.as_ref().unwrap().1.text(), shown);
        assert!(state.apply());
        assert!(!crate::config::settings_store::explicit(
            &crate::config::settings_store::source(&state.path).unwrap(),
            path
        ));
    }
}

/// 【设置实际值回归】【工具恢复】无参数，验证工具显示继承后的实际名称，菜单默认项包含实际值，恢复后仍保留继承关系
#[test]
fn reset_and_default_menu_option_keep_tool_inheritance_live() {
    let mut state = Preferences::new(false).unwrap();
    let common = entry("startup.default_tool");
    let fullscreen = entry("startup.fullscreen_default_tool");
    state.page = ENTRIES[common].page;
    assert!(state.save(common, "rectangle".into()));
    assert_eq!(state.value(fullscreen), "rectangle");
    state.activate(Control::Row(fullscreen));
    let menu = state.dropdown.as_ref().unwrap();
    let default = menu
        .options
        .iter()
        .find(|option| option.value.is_empty())
        .unwrap();
    assert!(default.label.contains("rectangle") || default.label.contains("矩形"));
    assert!(!default.label.contains("继承"));
    assert_eq!(menu.options[menu.selected.unwrap()].value, "");
    state.dropdown = None;
    assert!(state.save(fullscreen, "arrow".into()));
    state.activate(Control::Reset(fullscreen));
    assert_eq!(state.value(fullscreen), "rectangle");
    assert!(state.save(common, "pen".into()));
    assert_eq!(state.value(fullscreen), "pen");
}
