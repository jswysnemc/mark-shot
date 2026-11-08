//! 【工具栏配置】【布局持久化】三种工作场景分别保存按钮顺序，缺省配置保持现有工具栏

use crate::ui::toolbar::{ACTION_ITEMS, FULLSCREEN_ITEMS, ITEMS, ToolbarItem, catalog};
use serde::{Deserialize, Serialize};

pub const MAX_ITEMS: usize = 64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolbarLayout {
    #[serde(with = "items")]
    pub main: Vec<ToolbarItem>,
    #[serde(with = "items")]
    pub actions: Vec<ToolbarItem>,
    #[serde(with = "items")]
    pub fullscreen: Vec<ToolbarItem>,
    pub main_vertical: bool,
    pub actions_vertical: bool,
    pub fullscreen_vertical: bool,
}

impl Default for ToolbarLayout {
    /// 【工具栏配置】【默认布局】无参数，返回现有主栏、副栏和全屏栏，不改变首次启动行为
    fn default() -> Self {
        Self {
            main: ITEMS.to_vec(),
            actions: ACTION_ITEMS.to_vec(),
            fullscreen: FULLSCREEN_ITEMS.to_vec(),
            main_vertical: false,
            actions_vertical: true,
            fullscreen_vertical: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Profile {
    Main,
    Actions,
    #[default]
    Fullscreen,
}

impl Profile {
    pub const ALL: [Self; 3] = [Self::Main, Self::Actions, Self::Fullscreen];

    /// 【工具栏配置】【场景标识】无参数，返回当前场景的配置字段
    pub fn key(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Actions => "actions",
            Self::Fullscreen => "fullscreen",
        }
    }

    /// 【工具栏配置】【方向字段】无参数，返回当前场景的排列方向配置字段
    pub fn direction_key(self) -> &'static str {
        match self {
            Self::Main => "main_vertical",
            Self::Actions => "actions_vertical",
            Self::Fullscreen => "fullscreen_vertical",
        }
    }

    /// 【工具栏配置】【场景名称】按 chinese 返回对应工作场景的名称
    pub fn label(self, chinese: bool) -> &'static str {
        match (self, chinese) {
            (Self::Main, true) => "截图主栏",
            (Self::Actions, true) => "截图操作栏",
            (Self::Fullscreen, true) => "图片 / 全屏",
            (Self::Main, false) => "Main",
            (Self::Actions, false) => "Actions",
            (Self::Fullscreen, false) => "Image / full",
        }
    }

    /// 【工具栏配置】【场景读取】从 config 返回当前场景按钮切片，不复制或重新解析配置
    pub fn items(self, config: &ToolbarLayout) -> &[ToolbarItem] {
        match self {
            Self::Main => &config.main,
            Self::Actions => &config.actions,
            Self::Fullscreen => &config.fullscreen,
        }
    }

    /// 【工具栏配置】【排列方向】从 config 返回当前场景是否纵向排列
    pub fn vertical(self, config: &ToolbarLayout) -> bool {
        match self {
            Self::Main => config.main_vertical,
            Self::Actions => config.actions_vertical,
            Self::Fullscreen => config.fullscreen_vertical,
        }
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
