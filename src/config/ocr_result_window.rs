//! 【OCR 窗口配置】【独立置顶】结果窗口与贴图各自保存开关，定向更新保留其他业务刚写入的配置

use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct OcrResultWindow {
    #[serde(alias = "alwaysOnTop")]
    pub always_on_top: bool,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Section {
    #[serde(alias = "ocrResultWindow")]
    ocr_result_window: OcrResultWindow,
}

/// 【OCR 窗口配置】【当前路径】返回命令行覆盖或原生配置位置，目录不可用时报错，无参数
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

/// 【OCR 窗口配置】【独立读取】从 path 读取原生 TOML 或原版 JSON，返回结果窗口参数，不继承贴图开关
pub(crate) fn read_at(path: &Path) -> io::Result<OcrResultWindow> {
    let text = super::persistence::read(path)?;
    let section: Section = super::document::decode(path, &text)?;
    Ok(section.ocr_result_window)
}

/// 【OCR 窗口配置】【最新开关】重新读取结果窗口参数，文件不存在时沿用启动配置，其他错误原样返回，无参数
pub fn current() -> io::Result<OcrResultWindow> {
    match read_at(&path()?) {
        Ok(value) => Ok(value),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(super::get().ocr_result_window.clone())
        }
        Err(error) => Err(error),
    }
}

/// 【OCR 窗口配置】【置顶保存】把 always_on_top 保存到当前配置，返回保存结果，不修改贴图设置
pub fn save(always_on_top: bool) -> io::Result<()> {
    save_at(&path()?, always_on_top)
}

/// 【OCR 窗口配置】【原版更新】按 text、always_on_top 更新 JSON 的独立开关，返回完整 JSON，未知字段和贴图配置不变
fn update_json(text: &str, always_on_top: bool) -> io::Result<String> {
    serde_json::from_str::<Section>(text).map_err(io::Error::other)?;
    let mut document: serde_json::Value = serde_json::from_str(text).map_err(io::Error::other)?;
    let root = document
        .as_object_mut()
        .ok_or_else(|| io::Error::other("config must be an object"))?;
    let section_key = if root.contains_key("ocr_result_window") {
        "ocr_result_window"
    } else {
        "ocrResultWindow"
    };
    let table = root
        .entry(section_key)
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| io::Error::other("ocrResultWindow must be an object"))?;
    let key = if table.contains_key("always_on_top") || section_key == "ocr_result_window" {
        "always_on_top"
    } else {
        "alwaysOnTop"
    };
    table.insert(key.into(), serde_json::Value::Bool(always_on_top));
    Ok(serde_json::to_string_pretty(&document).map_err(io::Error::other)? + "\n")
}

/// 【OCR 窗口配置】【定向保存】在 path 的共享进程锁内更新 always_on_top，保留未知字段和注释；失败不替换原文件
pub(crate) fn save_at(path: &Path, always_on_top: bool) -> io::Result<()> {
    super::persistence::update(path, |text| {
        if super::document::is_json(path, text) {
            let text = if text == super::TEMPLATE && !path.exists() {
                "{}"
            } else {
                text
            };
            return Ok((update_json(text, always_on_top)?, ()));
        }
        toml::from_str::<Section>(text).map_err(io::Error::other)?;
        let mut document = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(io::Error::other)?;
        let section_key = if document.contains_key("ocrResultWindow") {
            "ocrResultWindow"
        } else {
            "ocr_result_window"
        };
        let table = document
            .entry(section_key)
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
            .as_table_like_mut()
            .ok_or_else(|| io::Error::other("ocr_result_window must be a table"))?;
        let key = if table.contains_key("alwaysOnTop") {
            "alwaysOnTop"
        } else {
            "always_on_top"
        };
        let mut value = toml_edit::Value::from(always_on_top);
        if let Some(previous) = table.get(key).and_then(toml_edit::Item::as_value) {
            *value.decor_mut() = previous.decor().clone();
        }
        table.insert(key, toml_edit::Item::Value(value));
        let text = document.to_string();
        toml::from_str::<Section>(&text).map_err(io::Error::other)?;
        Ok((text, ()))
    })
}

#[cfg(test)]
#[path = "tests/ocr_result_window.rs"]
mod tests;
