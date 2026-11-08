//! 【设置下拉】【业务适配】将通用选项映射到实际保存事务，保存失败保留当前浮层

use super::{
    catalog::{ENTRIES, Kind},
    dropdown::{Dropdown, OptionItem, Target, layout::Layout},
    window::Preferences,
};

/// 【设置下拉】【通用设置】按 index 为 state 当前字段创建浮层，选项保存值保持原配置协议
pub(super) fn setting(state: &mut Preferences, index: usize) {
    let (Kind::Choice(choices) | Kind::Segments(choices)) = ENTRIES[index].kind else {
        return;
    };
    let layout = super::layout::Layout::new(state);
    let Some(row) = layout.rows.iter().find(|row| row.index == index) else {
        return;
    };
    let options = choices
        .iter()
        .map(|value| {
            OptionItem::new(
                *value,
                ENTRIES[index].option_label(value, state.chinese()),
                "",
            )
        })
        .collect();
    let value = state.value(index);
    let mut menu = Dropdown::new(
        Target::Setting(index),
        row.control,
        options,
        &value,
        &mut state.fonts,
    );
    menu.reveal(Layout::new(&menu, state.size).capacity);
    state.dropdown = Some(menu);
}

/// 【设置下拉】【确认保存】按 target 和 value 写入 state 对应配置，返回成功状态，选择标签不参与持久化
pub(super) fn choose(state: &mut Preferences, target: Target, value: &str) -> bool {
    match target {
        Target::Setting(index) => state.save(index, value.into()),
        Target::Translation(field) => super::translation::input::choose(state, field, value),
    }
}
