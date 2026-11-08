//! 【设置窗口】【配置桥接】读取生效值，按字段原子更新原文件并保留注释和未知配置

use serde_json::Value;
use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, RwLock},
};

static CURRENT: OnceLock<RwLock<Arc<super::Config>>> = OnceLock::new();

/// 【应用设置】【业务快照】无参数，返回当前业务配置共享快照，不在指针移动或绘制时读取文件
pub fn current() -> Arc<super::Config> {
    CURRENT
        .get_or_init(|| RwLock::new(Arc::new(super::get().clone())))
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

/// 【应用设置】【保存发布】从 path 读取已保存配置并发布给输出与识别流程，返回错误；界面几何仍沿用启动配置
pub fn reload(path: &Path) -> io::Result<()> {
    let config = load(path)?;
    crate::keys::reload(&config.keys.0);
    *CURRENT
        .get_or_init(|| RwLock::new(Arc::new(super::get().clone())))
        .write()
        .unwrap_or_else(|error| error.into_inner()) = Arc::new(config);
    Ok(())
}

/// 【设置窗口】【当前路径】无参数，返回当前进程实际使用的配置路径，无法确定目录时返回错误
pub fn path() -> io::Result<PathBuf> {
    super::loading::config_path()
        .ok_or_else(|| io::Error::other("configuration directory is unavailable"))
}

/// 【设置窗口】【有效配置】按 path 读取并解析启动配置，返回含默认值的对象，文件缺失使用默认配置
pub fn read(path: &Path) -> io::Result<Value> {
    serde_json::to_value(load(path)?).map_err(io::Error::other)
}

/// 【设置窗口】【启动解析】按 path 返回与应用启动相同的强类型配置，缺失文件使用默认值
pub fn load(path: &Path) -> io::Result<super::Config> {
    let config = match super::persistence::read(path) {
        Ok(text) => {
            super::document::root(path, &text)?;
            super::loading::parse(&text, &path.to_string_lossy())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => super::Config::default(),
        Err(error) => return Err(error),
    };
    Ok(config)
}

/// 【设置窗口】【字段保存】将 value 写入 path 的 section/key，返回保存结果；锁内读取最新版本，其他字段和格式保持原样
pub fn write(path: &Path, section: &str, key: &str, value: Value) -> io::Result<()> {
    validate(section, key, &value)?;
    super::persistence::update(path, |text| {
        if super::document::is_json(path, text) {
            let updated = super::document::update_json(text, |root| {
                super::document::section(root, &[section], section)?
                    .insert(key.into(), value.clone());
                Ok(())
            })?;
            return Ok((updated, ()));
        }
        let mut document = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(io::Error::other)?;
        let replacement = match &value {
            Value::Bool(value) => toml_edit::Value::from(*value),
            Value::String(value) => toml_edit::Value::from(value.as_str()),
            Value::Array(values) if values.iter().all(Value::is_string) => {
                let values = values
                    .iter()
                    .map(|value| value.as_str().unwrap())
                    .collect::<toml_edit::Array>();
                toml_edit::Value::Array(values)
            }
            Value::Number(value) if value.is_i64() => {
                toml_edit::Value::from(value.as_i64().unwrap())
            }
            _ => return Err(io::Error::other("unsupported preference value")),
        };
        let section = document
            .entry(section)
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
        let table = section
            .as_table_mut()
            .ok_or_else(|| io::Error::other("configuration section must be a table"))?;
        let mut replacement = replacement;
        if let Some(previous) = table.get(key).and_then(toml_edit::Item::as_value) {
            *replacement.decor_mut() = previous.decor().clone();
        }
        table.insert(key, toml_edit::Item::Value(replacement));
        Ok((document.to_string(), ()))
    })
}

/// 【设置窗口】【输入校验】校验 section、key 对应的 value，拒绝会导致保存路径错误的日期格式及非法快捷键
fn validate(section: &str, key: &str, value: &Value) -> io::Result<()> {
    if section == "toolbar_layout" && matches!(key, "main" | "actions" | "fullscreen") {
        let mut root = serde_json::Map::new();
        root.insert(key.into(), value.clone());
        serde_json::from_value::<super::toolbar_layout::ToolbarLayout>(root.into())
            .map_err(io::Error::other)?;
    }
    if section == "save" && matches!(key, "filename_format" | "month_format") {
        let pattern = value
            .as_str()
            .ok_or_else(|| io::Error::other("日期格式必须是文字 / Date pattern must be text"))?;
        if (key == "filename_format" && (pattern.is_empty() || pattern.contains(['/', '\\'])))
            || chrono::format::StrftimeItems::new(pattern)
                .any(|item| matches!(item, chrono::format::Item::Error))
        {
            return Err(io::Error::other("无效日期格式 / Invalid date pattern"));
        }
    }
    if section == "keys" {
        let bindings = match value {
            Value::String(text) => vec![text.as_str()],
            Value::Array(list) => list
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .ok_or_else(|| io::Error::other("快捷键必须是文字 / Shortcut must be text"))
                })
                .collect::<io::Result<Vec<_>>>()?,
            _ => return Err(io::Error::other("快捷键必须是文字 / Shortcut must be text")),
        };
        for text in bindings.into_iter().filter(|text| !text.is_empty()) {
            crate::keys::parse(text).map_err(io::Error::other)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// 【设置回归】【无效输入】无参数，无效文件名不能覆盖之前可用的保存配置
    #[test]
    fn invalid_filename_pattern_does_not_replace_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        let original = "[save]\nfilename_format = '%Y-%m-%d'\n";
        std::fs::write(&path, original).unwrap();
        assert!(super::write(&path, "save", "filename_format", "%Q".into()).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
    }
    /// 【设置回归】【原版文件】无参数，设置写入 JSON 后实际启动解析必须采用新工具和鼠标开关
    #[test]
    fn json_preferences_reach_startup_consumers() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        std::fs::write(&path, r#"{"capture":{"includeCursor":false},"custom":7}"#).unwrap();
        super::write(&path, "startup", "include_cursor", true.into()).unwrap();
        super::write(&path, "startup", "normal_tool", "text".into()).unwrap();
        let config = super::read(&path).unwrap();
        assert_eq!(config["startup"]["include_cursor"], true);
        assert_eq!(config["startup"]["normal_tool"], "text");
    }
    /// 【设置回归】【原子字段】无参数，验证设置窗口写入后启动解析可见，保留相邻未知字段、注释和后续更新
    #[test]
    fn saves_effective_values_without_replacing_other_settings() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        std::fs::write(
            &path,
            "# keep\n[notifications]\nenabled = true # keep too\n[custom]\nvalue = 7\n",
        )
        .unwrap();
        super::write(&path, "notifications", "enabled", false.into()).unwrap();
        super::write(&path, "ui", "language", "chinese".into()).unwrap();
        let value = super::read(&path).unwrap();
        assert_eq!(value["notifications"]["enabled"], false);
        assert_eq!(value["ui"]["language"], "chinese");
        let text = std::fs::read_to_string(path).unwrap();
        assert!(
            text.contains("# keep") && text.contains("# keep too") && text.contains("value = 7")
        );
    }
}
