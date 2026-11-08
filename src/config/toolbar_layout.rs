//! 【工具栏配置】【布局持久化】只保存主栏和侧栏，全屏按两栏顺序合并

use crate::ui::toolbar::{ACTION_ITEMS, ITEMS, ToolbarItem, catalog};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[cfg(test)]
mod tests;

pub const MAX_ITEMS: usize = 64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolbarLayout {
    #[serde(with = "items")]
    pub main: Vec<ToolbarItem>,
    #[serde(with = "items")]
    pub actions: Vec<ToolbarItem>,
    pub main_vertical: bool,
    pub actions_vertical: bool,
}

impl Default for ToolbarLayout {
    /// 【工具栏配置】【默认布局】无参数，返回横向主栏和纵向侧栏，全屏不另存配置
    fn default() -> Self {
        Self {
            main: ITEMS.to_vec(),
            actions: ACTION_ITEMS.to_vec(),
            main_vertical: false,
            actions_vertical: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Profile {
    #[default]
    Main,
    Actions,
    Fullscreen,
}

impl Profile {
    pub const ALL: [Self; 2] = [Self::Main, Self::Actions];

    /// 【工具栏配置】【场景标识】无参数，返回当前场景的配置字段
    pub fn key(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Actions => "actions",
            Self::Fullscreen => "fullscreen",
        }
    }

    /// 【工具栏配置】【场景名称】按 chinese 返回对应工作场景的名称
    pub fn label(self, chinese: bool) -> &'static str {
        match (self, chinese) {
            (Self::Main, true) => "主工具栏",
            (Self::Actions, true) => "侧工具栏",
            (Self::Fullscreen, true) => "图片 / 全屏",
            (Self::Main, false) => "Main",
            (Self::Actions, false) => "Side toolbar",
            (Self::Fullscreen, false) => "Image / full",
        }
    }

    /// 【工具栏配置】【场景读取】从 config 借用主侧栏；全屏返回去重且整理分隔线后的合并列表
    pub fn items(self, config: &ToolbarLayout) -> Cow<'_, [ToolbarItem]> {
        match self {
            Self::Main => Cow::Borrowed(&config.main),
            Self::Actions => Cow::Borrowed(&config.actions),
            Self::Fullscreen => Cow::Owned(config.merged()),
        }
    }

    /// 【工具栏配置】【排列方向】从 config 返回当前场景是否纵向排列
    pub fn vertical(self, config: &ToolbarLayout) -> bool {
        match self {
            Self::Main => config.main_vertical,
            Self::Actions => config.actions_vertical,
            Self::Fullscreen => config.main_vertical,
        }
    }
}

impl ToolbarLayout {
    /// 【工具栏配置】【全屏合并】无参数，返回主栏在前侧栏在后的唯一按钮列表，去除首尾与连续分隔线
    pub fn merged(&self) -> Vec<ToolbarItem> {
        let mut merged = Vec::with_capacity(self.main.len() + self.actions.len() + 1);
        let mut separator = false;
        for item in self
            .main
            .iter()
            .chain(std::iter::once(&ToolbarItem::Seperator))
            .chain(&self.actions)
        {
            if matches!(item, ToolbarItem::Seperator) {
                separator = !merged.is_empty();
            } else if !merged.contains(item) {
                if separator {
                    merged.push(ToolbarItem::Seperator);
                }
                merged.push(*item);
                separator = false;
            }
        }
        merged
    }
}

/// 【工具栏配置】【布局检查】按 items 验证项目上限、唯一按钮与可操作性，返回错误说明
pub fn validate(items: &[ToolbarItem]) -> Result<(), String> {
    if items.len() > MAX_ITEMS {
        return Err("工具栏最多 64 项 / Maximum 64 items".into());
    }
    if !items
        .iter()
        .any(|item| matches!(item, ToolbarItem::Button(_)))
    {
        return Err("至少保留一个按钮 / Keep at least one button".into());
    }
    for (index, item) in items.iter().enumerate() {
        if matches!(item, ToolbarItem::Button(_)) && items[..index].contains(item) {
            return Err("不能重复添加按钮 / Duplicate button".into());
        }
    }
    Ok(())
}

mod items {
    use super::*;
    /// 【工具栏配置】【数组写入】把 value 按稳定身份写入 serializer，返回序列化结果
    pub fn serialize<S: serde::Serializer>(
        value: &[ToolbarItem],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value
            .iter()
            .map(|item| catalog::id(*item))
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
    /// 【工具栏配置】【数组读取】从 deserializer 解析身份并校验，返回按钮列表或字段错误
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<ToolbarItem>, D::Error> {
        let names = Vec::<String>::deserialize(deserializer)?;
        let items = names
            .iter()
            .map(|name| {
                catalog::parse(name).ok_or_else(|| {
                    serde::de::Error::custom(format!("unknown toolbar item: {name}"))
                })
            })
            .collect::<Result<Vec<_>, D::Error>>()?;
        validate(&items).map_err(serde::de::Error::custom)?;
        Ok(items)
    }
}
