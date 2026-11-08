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
    if cfg!(test) {
        return Ok(());
    }
    let path = super::preferences::path()?;
    super::settings_store::write(&path, &[("startup.selection_loupe".into(), enabled.into())])?;
    super::preferences::reload(&path)
}

/// 【启动配置】【放大区迁移】按 config、path 将旧工具状态迁移为统一偏好，成功后返回空值，失败保留旧文件
pub(super) fn migrate_loupe(config: &mut super::Config, path: &std::path::Path) -> io::Result<()> {
    if config.startup.selection_loupe.is_none()
        && let Some(enabled) = crate::editor::saved::get().magnifier
    {
        super::settings_store::write(path, &[("startup.selection_loupe".into(), enabled.into())])?;
        config.startup.selection_loupe = Some(enabled);
        if let Some(state) =
            dirs::state_dir().map(|directory| directory.join("mark-shot/tools.toml"))
            && state.exists()
        {
            super::persistence::update(&state, |text| {
                let mut document = text
                    .parse::<toml_edit::DocumentMut>()
                    .map_err(io::Error::other)?;
                document.remove("magnifier");
                Ok((document.to_string(), ()))
            })?;
        }
    }
    Ok(())
}
