//! 【翻译配置】【读取边界】保存稳定界面参数和厂商原始字段，凭据不参与调试输出

use serde::Deserialize;
use serde_json::{Map, Value};
use std::{
    fmt, io,
    path::{Path, PathBuf},
};

mod decode;
mod encode;

#[derive(Clone, PartialEq)]
pub struct Translation {
    pub target_language: String,
    pub provider: String,
    pub command: String,
    pub timeout_ms: u64,
    pub(crate) timeout_explicit: bool,
    pub settings: Map<String, Value>,
}

impl Default for Translation {
    /// 【翻译配置】【默认参数】返回原版简体中文目标与自动服务选择，无参数，不提供或读取默认凭据
    fn default() -> Self {
        Self {
            target_language: "Simplified Chinese".into(),
            provider: "auto".into(),
            command: String::new(),
            timeout_ms: 60000,
            timeout_explicit: false,
            settings: Map::new(),
        }
    }
}

impl fmt::Debug for Translation {
    /// 【翻译配置】【调试脱敏】把非敏感界面参数写入 formatter，返回格式结果，命令和厂商配置只显示存在性
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Translation")
            .field("target_language", &self.target_language)
            .field("provider", &self.provider)
            .field("timeout_ms", &self.timeout_ms)
            .field("has_command", &!self.command.is_empty())
            .field("settings", &"[redacted]")
            .finish()
    }
}

impl Translation {
    /// 【翻译配置】【字段规范】约束 timeout 并规范目标语言，无参数、无返回值；服务凭据留给服务解析层校验
    pub(crate) fn normalize(&mut self) {
        self.timeout_ms = self.timeout_ms.clamp(1000, 300000);
        self.target_language = crate::translation::language::value(&self.target_language);
        if self.target_language.is_empty() {
            self.target_language = Self::default().target_language;
        }
    }

    /// 【翻译配置】【服务对象】返回包括厂商字段的完整服务配置，无参数，不改变持久配置
    pub(crate) fn root(&self) -> Value {
        let mut root = self.settings.clone();
        root.insert(
            "target_language".into(),
            self.target_language.clone().into(),
        );
        root.insert("provider".into(), self.provider.clone().into());
        root.insert("command".into(), self.command.clone().into());
        if self.timeout_explicit || self.timeout_ms != Self::default().timeout_ms {
            root.insert("timeout_ms".into(), self.timeout_ms.into());
        }
        Value::Object(root)
    }
}

/// 【翻译配置】【当前路径】返回命令行覆盖或原生配置路径，无参数，缺少用户目录时返回错误
pub(crate) fn path() -> io::Result<PathBuf> {
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

/// 【翻译配置】【安全读取】按 path 读取独立翻译节，兼容原版 JSON，拒绝链接、多重硬链接和超过一 MiB 的文件
pub(crate) fn read_at(path: &Path) -> io::Result<Translation> {
    let text = super::persistence::read(path)?;
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Section {
        translation: Translation,
    }
    let mut config = super::document::decode::<Section>(path, &text)?.translation;
    config.normalize();
    Ok(config)
}

/// 【翻译配置】【最新参数】重新读取当前翻译配置，无参数，文件不存在时沿用启动配置，格式错误不静默覆盖
pub fn current() -> io::Result<Translation> {
    match read_at(&path()?) {
        Ok(config) => Ok(config),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(super::get().translation.clone())
        }
        Err(error) => Err(error),
    }
}

/// 【翻译配置】【语言保存】将 target 保存到当前原生配置，返回规范化名称；单项更新保留所有凭据、未知字段和注释
pub fn save_target(target: &str) -> io::Result<String> {
    save_target_at(&path()?, target)
}

/// 【翻译配置】【定向保存】在 path 的共享锁内保存 target，返回规范化名称；拒绝空目标或非法配置，失败不修改文件
pub(crate) fn save_target_at(path: &Path, target: &str) -> io::Result<String> {
    let target = crate::translation::language::value(target);
    if target.is_empty() {
        return Err(io::Error::other("Target language is empty"));
    }
    super::persistence::update(path, |text| {
        if super::document::is_json(path, text) {
            #[derive(Deserialize, Default)]
            #[serde(default)]
            struct Section {
                translation: Translation,
            }
            super::document::decode::<Section>(path, text)?;
            let updated = super::document::update_json(text, |root| {
                let table = super::document::section(root, &["translation"], "translation")?;
                let key = if table.contains_key("target_language") {
                    "target_language"
                } else {
                    "targetLanguage"
                };
                table.insert(key.into(), target.clone().into());
                Ok(())
            })?;
            return Ok((updated, target.clone()));
        }
        let mut document = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(io::Error::other)?;
        let translation = document
            .entry("translation")
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
            .as_table_like_mut()
            .ok_or_else(|| io::Error::other("translation must be a table"))?;
        let key = if translation.contains_key("targetLanguage") {
            "targetLanguage"
        } else {
            "target_language"
        };
        let mut value = toml_edit::Value::from(target.as_str());
        if let Some(previous) = translation.get(key).and_then(toml_edit::Item::as_value) {
            *value.decor_mut() = previous.decor().clone();
        }
        translation.insert(key, toml_edit::Item::Value(value));
        let text = document.to_string();
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct Section {
            translation: Translation,
        }
        toml::from_str::<Section>(&text).map_err(io::Error::other)?;
        Ok((text, target.clone()))
    })
}

#[cfg(test)]
#[path = "tests/translation.rs"]
mod tests;
