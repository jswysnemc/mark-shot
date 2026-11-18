//! 【应用设置】【实际取值】解析默认工具继承、工具颜色和用户目录，不修改配置文档

use super::Entry;
use serde_json::Value;

/// 【应用设置】【工具继承】按 values 和 key 读取启动工具优先级，返回规范工具名；无覆盖时采用对应入口的内置工具
fn tool(values: &Value, key: &str) -> String {
    let keys: &[&str] = match key {
        "normal_tool" => &["normal_tool", "default_tool"],
        "fullscreen_default_tool" => &["fullscreen_default_tool", "default_tool", "normal_tool"],
        "file_default_tool" => &[
            "file_default_tool",
            "fullscreen_default_tool",
            "default_tool",
            "normal_tool",
        ],
        _ => &["default_tool"],
    };
    keys.iter()
        .find_map(|key| {
            let text = values["startup"][*key].as_str()?.trim();
            let tool = crate::tools::names::parse(text).ok()?;
            Some(
                super::extra::TOOLS
                    .iter()
                    .copied()
                    .find(|name| crate::tools::names::parse(name).ok() == Some(tool))
                    .unwrap_or(text)
                    .to_owned(),
            )
        })
        .unwrap_or_else(|| {
            if key == "file_default_tool" {
                "pick"
            } else {
                "selection"
            }
            .into()
        })
}

/// 【应用设置】【目录文字】按 path 返回实际绝对目录，缺少系统目录时返回空值，不创建目录或文件
fn directory(path: Option<std::path::PathBuf>) -> Option<String> {
    path.and_then(|path| std::path::absolute(path).ok())
        .map(|path| path.to_string_lossy().into_owned())
}

/// 【应用设置】【派生字段】按 entry、values、configured 解析当前有效值，返回显示文本；普通字段保持原配置值
pub(super) fn resolve(entry: &Entry, values: &Value, configured: String) -> String {
    match (entry.section, entry.key) {
        (
            "startup",
            "default_tool" | "normal_tool" | "fullscreen_default_tool" | "file_default_tool",
        ) => tool(values, entry.key),
        ("startup", "default_color") if configured.is_empty() => {
            let color = crate::editor::saved::get()
                .color
                .or_else(|| {
                    values["tools"]["color"]
                        .as_str()
                        .and_then(crate::theme::Rgba::parse_hex)
                })
                .unwrap_or_else(crate::types::tool_settings::default_color);
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                color.0, color.1, color.2, color.3
            )
        }
        ("save", "directory") => directory(
            dirs::picture_dir()
                .or_else(|| dirs::home_dir().map(|home| home.join("Pictures")))
                .map(|root| root.join(&configured)),
        )
        .unwrap_or(configured),
        ("history", "directory") if configured.is_empty() => {
            directory(dirs::data_dir().map(|root| root.join("mark-shot/capture-history")))
                .unwrap_or(configured)
        }
        ("recording", "directory") if configured.trim().is_empty() => directory(
            dirs::video_dir()
                .or_else(dirs::home_dir)
                .map(|root| root.join("Mark Shot")),
        )
        .unwrap_or(configured),
        ("history" | "recording", "directory") => {
            directory(Some(configured.clone().into())).unwrap_or(configured)
        }
        ("log", "path") if configured.is_empty() => {
            directory(crate::logging::default_log_file_path()).unwrap_or(configured)
        }
        _ => configured,
    }
}
