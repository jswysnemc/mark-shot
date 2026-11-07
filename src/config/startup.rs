//! 【启动配置】【入口覆盖】保留原版默认工具的共同值、区域独立值和文件覆盖顺序

use crate::{app::Launch, theme::Rgba, tools::Tool};
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Startup {
    pub default_tool: Option<String>,
    pub normal_tool: Option<String>,
    pub fullscreen_default_tool: Option<String>,
    pub file_default_tool: Option<String>,
    pub default_color: Option<Rgba>,
    pub include_cursor: bool,
    pub selection_loupe: Option<bool>,
}

/// 【启动配置】【工具转换】按 value 返回有效配置工具，错误名称只忽略本字段，不输出配置正文
fn tool(value: Option<&str>) -> Option<Tool> {
    value.and_then(|value| crate::tools::names::parse(value.trim()).ok())
}

impl Startup {
    /// 【启动配置】【命令覆盖】按 self 补齐 launch，命令行优先；普通配置的独立工具不覆盖全屏，鼠标包含使用有效开关，无返回值
    pub(crate) fn apply(&self, launch: &mut Launch) {
        let normal = launch.default_tool;
        let fullscreen = launch.fullscreen_default_tool;
        let file = launch.file_default_tool;
        let common = tool(self.default_tool.as_deref());
        let configured_fullscreen = tool(self.fullscreen_default_tool.as_deref()).or(common);
        launch.default_tool = normal.or(tool(self.normal_tool.as_deref())).or(common);
        launch.fullscreen_default_tool = fullscreen.or(normal).or(configured_fullscreen);
        launch.file_default_tool = file
            .or(fullscreen)
            .or(normal)
            .or(tool(self.file_default_tool.as_deref()))
            .or(configured_fullscreen);
        launch.default_color = launch.default_color.or(self.default_color);
        launch.include_cursor |= self.include_cursor;
    }
}

/// 【启动配置】【放大区保存】把 enabled 保存到当前格式对应的选区放大区字段，返回保存结果，不影响取色开关
pub(crate) fn save_loupe(enabled: bool) -> io::Result<()> {
    let path = super::OVERRIDE_PATH
        .get()
        .cloned()
        .or_else(super::loading::config_path)
        .ok_or_else(|| io::Error::other("configuration directory is unavailable"))?;
    super::persistence::update(&path, |text| {
        if super::document::is_json(&path, text) {
            let updated = super::document::update_json(text, |root| {
                let capture = super::document::section(
                    root,
                    &["capture", "screenshot", "screenCapture"],
                    "capture",
                )?;
                let key = if capture.contains_key("selectionLoupe") {
                    "selectionLoupe"
                } else if capture.contains_key("loupe") {
                    "loupe"
                } else {
                    "selectionLoupe"
                };
                let value = capture.entry(key).or_insert_with(|| serde_json::json!({}));
                if let Some(table) = value.as_object_mut() {
                    let key = if table.contains_key("enable") {
                        "enable"
                    } else {
                        "enabled"
                    };
                    table.insert(key.into(), enabled.into());
                } else {
                    *value = enabled.into();
                }
                Ok(())
            })?;
            return Ok((updated, ()));
        }
        let mut document = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(io::Error::other)?;
        let startup = document
            .entry("startup")
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
            .as_table_like_mut()
            .ok_or_else(|| io::Error::other("startup must be a table"))?;
        let mut value = toml_edit::Value::from(enabled);
        if let Some(previous) = startup
            .get("selection_loupe")
            .and_then(toml_edit::Item::as_value)
        {
            *value.decor_mut() = previous.decor().clone();
        }
        startup.insert("selection_loupe", toml_edit::Item::Value(value));
        Ok((document.to_string(), ()))
    })
}
