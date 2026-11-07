//! 【界面语言配置】【兼容读取】读取原生 ui.language 和原版 JSON 配置，旧版顶层 language 只在新键缺失时生效

use crate::ui::language::Mode;
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Ui {
    pub language: String,
    pub theme: crate::theme::Mode,
}

impl Ui {
    /// 【界面语言配置】【启动回退】按 legacy 兼容旧顶层键，返回当前原生配置的有效模式，非空新键优先
    pub(crate) fn mode(&self, legacy: Option<&str>) -> Mode {
        Mode::parse(if self.language.trim().is_empty() {
            legacy.unwrap_or_default()
        } else {
            &self.language
        })
    }
}

/// 【界面语言配置】【原版字段】按 current、legacy 返回原版优先级模式；空值和错误类型等同缺失，未知非空新值仍优先
fn mode(current: Option<&str>, legacy: Option<&str>) -> Mode {
    Mode::parse(
        current
            .filter(|value| !value.trim().is_empty())
            .or(legacy)
            .unwrap_or_default(),
    )
}

/// 【界面语言配置】【独立读取】从 path 安全读取原生 TOML 或原版 JSON，返回有效模式；不改写任何字段
pub(crate) fn read_at(path: &Path) -> io::Result<Mode> {
    let text = super::persistence::read(path)?;
    let value = super::document::root(path, &text)?;
    Ok(mode(
        value
            .get("ui")
            .and_then(|ui| ui.get("language"))
            .and_then(|value| value.as_str()),
        value.get("language").and_then(|value| value.as_str()),
    ))
}

/// 【界面语言配置】【当前读取】返回命令行指定或原生配置中的模式，文件缺失沿用启动配置，其他读取错误原样返回，无参数
pub fn current() -> io::Result<Mode> {
    let path = super::OVERRIDE_PATH
        .get()
        .cloned()
        .or_else(super::loading::config_path);
    match path.map(|path| read_at(&path)).transpose() {
        Ok(Some(mode)) => Ok(mode),
        Ok(None) => Ok(super::get().ui.mode(super::get().language.as_deref())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(super::get().ui.mode(super::get().language.as_deref()))
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "tests/interface_language.rs"]
mod tests;
