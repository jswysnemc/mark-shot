//! 【设置下拉】【选项状态】统一短选项和可搜索长列表，展示标签与保存值独立

pub(super) mod draw;
pub(super) mod input;
pub(super) mod layout;
#[cfg(test)]
mod tests;

use crate::config::settings::translation::Field;
use crate::ui::{fonts::UiFonts, scrollbar, single_line::SingleLine};
use tiny_skia::Rect;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    Setting(usize),
    Translation(Field),
}

pub(super) struct OptionItem {
    pub value: String,
    pub label: String,
    pub detail: String,
}

impl OptionItem {
    /// 【设置下拉】【选项创建】按 value、label 和 detail 创建保存值与双语检索标签，返回选项
    pub fn new(
        value: impl Into<String>,
        label: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            detail: detail.into(),
        }
    }
}

pub(super) struct Dropdown {
    pub target: Target,
    pub anchor: Rect,
    pub options: Vec<OptionItem>,
    pub selected: Option<usize>,
    pub cursor: Option<usize>,
    pub filtered: Vec<usize>,
    pub offset: usize,
    pub search: Option<SingleLine>,
    pub pressed: Option<layout::Hit>,
    pub drag: Option<scrollbar::Drag>,
}

impl Dropdown {
    /// 【设置下拉】【浮层创建】按 target、anchor、options、value 与 fonts 初始化选中状态，超过八项启用搜索
    pub fn new(
        target: Target,
        anchor: Rect,
        options: Vec<OptionItem>,
        value: &str,
        fonts: &mut UiFonts,
    ) -> Self {
        let selected = options.iter().position(|option| option.value == value);
        let filtered: Vec<_> = (0..options.len()).collect();
        let search = (options.len() > 8).then(|| SingleLine::new("", fonts, 128));
        Self {
            target,
            anchor,
            options,
            selected,
            cursor: selected.or_else(|| filtered.first().copied()),
            filtered,
            offset: 0,
            search,
            pressed: None,
            drag: None,
        }
    }

    /// 【设置下拉】【搜索过滤】按当前输入匹配保存值、标题及说明，返回空值；空结果不保留旧候选
    pub fn filter(&mut self) {
        let query = self
            .search
            .as_ref()
            .map(SingleLine::text)
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        self.filtered = self
            .options
            .iter()
            .enumerate()
            .filter(|(_, option)| {
                format!("{} {} {}", option.value, option.label, option.detail)
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        self.cursor = self
            .selected
            .filter(|index| self.filtered.contains(index))
            .or_else(|| self.filtered.first().copied());
        self.offset = 0;
    }

    /// 【设置下拉】【候选可见】按 capacity 将键盘候选保持在可见条目内，无返回值
    pub fn reveal(&mut self, capacity: usize) {
        if let Some(position) = self
            .cursor
            .and_then(|cursor| self.filtered.iter().position(|index| *index == cursor))
        {
            if position < self.offset {
                self.offset = position;
            }
            if position >= self.offset + capacity {
                self.offset = position + 1 - capacity;
            }
        }
        self.offset = self
            .offset
            .min(self.filtered.len().saturating_sub(capacity));
    }
}
