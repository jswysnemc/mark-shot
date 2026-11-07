//! 【截图历史】【配置】记录策略与保留其他配置的原子更新

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct History {
    pub enabled: bool,
    pub index_annotations: bool,
    pub limit: usize,
    pub max_size_mib: u64,
    pub directory: String,
}

impl Default for History {
    /// 【截图历史】【默认策略】返回启用记录、保留五十项和二百五十六 MiB 的配置
    fn default() -> Self {
        Self {
            enabled: true,
            index_annotations: false,
            limit: 50,
            max_size_mib: 256,
            directory: String::new(),
        }
    }
}

impl History {
    /// 【截图历史】【配置约束】约束数量和字节容量，避免无效值与乘法溢出；无返回值
    pub(crate) fn normalize(&mut self) {
        self.limit = self.limit.clamp(1, 200);
        self.max_size_mib = self.max_size_mib.clamp(1, 4096);
    }

    /// 【截图历史】【目录解析】将自定义路径转为绝对路径；未配置时返回用户应用数据目录
    pub fn path(&self) -> io::Result<PathBuf> {
        if !self.directory.is_empty() {
            return std::path::absolute(&self.directory);
        }
        dirs::data_dir()
            .map(|path| path.join("mark-shot/capture-history"))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "user data directory is unavailable",
                )
            })
    }

    /// 【截图历史】【字节容量】返回最大保存字节数；配置已限制在安全范围内
    pub fn byte_limit(&self) -> u64 {
        self.max_size_mib.clamp(1, 4096) * 1024 * 1024
    }
}

#[derive(Clone, Copy)]
pub enum Update {
    Enabled(bool),
    Limit(usize),
}

/// 【截图历史】【配置路径】返回命令行指定路径或默认路径；缺少用户配置目录时返回错误
fn path() -> io::Result<PathBuf> {
    super::OVERRIDE_PATH
        .get()
        .cloned()
        .or_else(super::loading::config_path)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "user config directory is unavailable",
            )
        })
}

/// 【截图历史】【策略读取】从 text 单独读取历史配置并约束范围；无效类型返回错误
fn parse_at(path: &Path, text: &str) -> io::Result<History> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Section {
        history: History,
    }
    let mut settings = if super::document::is_json(path, text) {
        let root = super::document::root(path, text)?;
        serde_json::from_value(
            super::legacy::history::section(&root).unwrap_or_else(|| serde_json::json!({})),
        )
        .map_err(io::Error::other)?
    } else {
        super::document::decode::<Section>(path, text)?.history
    };
    settings.normalize();
    Ok(settings)
}

/// 【截图历史】【最新策略】重新读取历史配置，使其他窗口的开关立即影响导出；读取失败保留启动配置
pub fn current() -> History {
    path()
        .and_then(|path| read_config(&path).and_then(|text| parse_at(&path, &text)))
        .unwrap_or_else(|_| super::get().history.clone())
}

/// 【截图历史】【安全读取】读取 path 的普通配置文件；拒绝链接及超过一 MiB 的内容
fn read_config(path: &Path) -> io::Result<String> {
    super::persistence::read(path)
}

/// 【截图历史】【策略持久化】更新一项 update，原子保存到当前配置；返回实际保存的历史策略
pub fn update(update: Update) -> io::Result<History> {
    update_at(&path()?, update)
}

/// 【截图历史】【配置更新】在 path 上串行化 update，保留未知字段与注释；失败不改变原文件
pub(crate) fn update_at(path: &Path, update: Update) -> io::Result<History> {
    super::persistence::update(path, |text| {
        if super::document::is_json(path, text) {
            parse_at(path, text)?;
            let updated = super::document::update_json(text, |root| {
                let table = super::document::section(
                    root,
                    &["history", "captureHistory"],
                    "captureHistory",
                )?;
                let (key, value) = match update {
                    Update::Enabled(enabled) => ("enabled", serde_json::Value::Bool(enabled)),
                    Update::Limit(limit) => ("limit", (limit.clamp(1, 200) as u64).into()),
                };
                table.insert(key.into(), value);
                Ok(())
            })?;
            let settings = parse_at(path, &updated)?;
            return Ok((updated, settings));
        }
        let mut document = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(io::Error::other)?;
        parse_at(path, text)?;
        // 1. 【截图历史】【字段更新】在共享配置锁内只改变用户操作的字段，保留其他窗口的设置
        let history = document
            .entry("history")
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
        let history = history
            .as_table_like_mut()
            .ok_or_else(|| io::Error::other("history must be a table"))?;
        let (key, value) = match update {
            Update::Enabled(enabled) => ("enabled", toml_edit::Value::from(enabled)),
            Update::Limit(limit) => ("limit", toml_edit::Value::from(limit.clamp(1, 200) as i64)),
        };
        if let Some(previous) = history.get(key).and_then(toml_edit::Item::as_value) {
            let mut value = value;
            *value.decor_mut() = previous.decor().clone();
            history.insert(key, toml_edit::Item::Value(value));
        } else {
            history.insert(key, toml_edit::Item::Value(value));
        }
        let updated = document.to_string();
        let result = parse_at(path, &updated)?;
        Ok((updated, result))
    })
}

#[cfg(test)]
#[path = "tests/history.rs"]
mod tests;
