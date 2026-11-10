//! 【应用设置】【统一写入】通过字段路径更新 JSON 和 TOML，保留注释、未知键与原子事务

use serde_json::Value;
use std::{io, path::Path};

/// 【应用设置】【字段读取】按 dotted 从 root 读取任意层级字段，缺失时返回空值
pub fn value<'a>(root: &'a Value, dotted: &str) -> &'a Value {
    dotted
        .split('.')
        .fold(root, |value, key| value.get(key).unwrap_or(&Value::Null))
}

/// 【应用设置】【来源读取】按 path 返回未经默认值填充的文档，缺失时返回空对象，不写文件
pub fn source(path: &Path) -> io::Result<Value> {
    match super::persistence::read(path) {
        Ok(text) => super::document::root(path, &text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(error) => Err(error),
    }
}

/// 【应用设置】【显式来源】按 root、path 返回该字段是否在原配置或兼容路径中存在
pub fn explicit(root: &Value, path: &str) -> bool {
    let keys = path_in(root, path, true);
    !keys
        .iter()
        .fold(root, |value, key| value.get(key).unwrap_or(&Value::Null))
        .is_null()
}

/// 【应用设置】【规范键名】将 key 转换为下划线形式，返回稳定存储名
pub fn snake(key: &str) -> String {
    key.chars()
        .flat_map(|character| {
            if character.is_ascii_uppercase() {
                vec!['_', character.to_ascii_lowercase()]
            } else {
                vec![character]
            }
        })
        .collect()
}

/// 【应用设置】【旧键兼容】按 key 返回已支持的旧别名，供两种文档格式共用
fn alias(key: &str) -> String {
    let mut uppercase = false;
    key.chars()
        .filter_map(|character| {
            if character == '_' {
                uppercase = true;
                None
            } else if uppercase {
                uppercase = false;
                Some(character.to_ascii_uppercase())
            } else {
                Some(character)
            }
        })
        .collect()
}

/// 【应用设置】【文档路径】按 root、path 和 json 返回原文档采用的路径，旧 JSON 优先更新原结构
fn path_in(root: &Value, path: &str, json: bool) -> Vec<String> {
    let mut keys: Vec<String> = path.split('.').map(snake).collect();
    if json && let Some(field) = path.strip_prefix("capture.") {
        let section = super::legacy::capture::capture_key(root);
        if field == "freeze_scope"
            && root[section].get("freezeScope").is_none()
            && root[section].get(field).is_none()
        {
            for key in ["freezeScope", "freezeDisplayScope"] {
                if root.get(key).is_some() {
                    return vec![key.into()];
                }
            }
        }
        return if field == "kde_kwin_screenshot" && root[section].get(field).is_none() {
            vec![
                section.into(),
                "wayland".into(),
                "kde".into(),
                "kwinScreenshot".into(),
                "enabled".into(),
            ]
        } else {
            let alternate = match field {
                "hide_own_windows" => Some("hideOwnWindowsDuringCapture"),
                "freeze_scope" => Some("freezeDisplayScope"),
                _ => None,
            };
            let key = if root[section].get(field).is_some() {
                field.to_owned()
            } else if root[section].get(alias(field)).is_some() {
                alias(field)
            } else {
                alternate
                    .filter(|key| root[section].get(*key).is_some())
                    .map_or_else(|| alias(field), str::to_owned)
            };
            vec![section.into(), key]
        };
    }

    if json && matches!(path, "toolbar.icon_size" | "toolbar.font_size") {
        return vec!["toolbar".into(), alias(&keys[1])];
    }
    if json
        && let Some(key) = path.strip_prefix("keys.")
        && root["keys"].get(key).is_none()
    {
        if root["shortcuts"]["actions"].get(key).is_some() {
            return vec!["shortcuts".into(), "actions".into(), key.into()];
        }
        if let Some(tools) = root["shortcuts"]["tools"].as_object() {
            for name in tools.keys() {
                if let Ok(tool) = crate::tools::names::parse(name)
                    && crate::keys::ACTIONS.iter().any(|(action, value, _)| {
                        *action == key && *value == crate::keys::Action::Tool(tool)
                    })
                {
                    return vec!["shortcuts".into(), "tools".into(), name.into()];
                }
            }
        }
    }
    if json && path == "ocr_result_window.always_on_top" && root.get("ocr_result_window").is_none()
    {
        keys = vec!["ocrResultWindow".into(), "alwaysOnTop".into()];
    }
    if json && root.get("startup").is_none() {
        let capture = super::legacy::capture::capture_key(root);
        let replacement = match path {
            "startup.include_cursor" => Some(vec![capture, "includeCursor"]),
            "startup.selection_loupe" => Some(if root[capture]["selectionLoupe"].is_boolean() {
                vec![capture, "selectionLoupe"]
            } else {
                vec![capture, "selectionLoupe", "enabled"]
            }),
            "startup.default_tool" => Some(vec!["annotation", "defaultTool"]),
            "startup.normal_tool" => Some(vec!["annotation", "defaultTools", "normal"]),
            "startup.fullscreen_default_tool" => {
                Some(vec!["annotation", "defaultTools", "fullscreen"])
            }
            "startup.file_default_tool" => Some(vec!["annotation", "fileDefaultTool"]),
            "startup.default_color" => Some(vec!["annotation", "defaultColor"]),
            _ => None,
        };
        if let Some(replacement) = replacement {
            keys = replacement.into_iter().map(str::to_owned).collect();
        }
    }
    let mut current = root;
    for key in &mut keys {
        if current.get(&*key).is_none() {
            let alternate = match key.as_str() {
                "history" => "captureHistory".to_owned(),
                "upload" => "imageUpload".to_owned(),
                "pinned" => "pinnedWindow".to_owned(),
                _ => alias(key),
            };
            if current.get(&alternate).is_some() {
                *key = alternate;
            }
        }
        current = current.get(&*key).unwrap_or(&Value::Null);
    }
    if json && let Some(field) = path.strip_prefix("pinned.border_") {
        let section = &root[&keys[0]];
        if section.get(&keys[1]).is_none() && section.get("border").is_some() {
            keys = if field == "enabled" && section["border"].is_boolean() {
                vec![keys[0].clone(), "border".into()]
            } else {
                vec![
                    keys[0].clone(),
                    "border".into(),
                    if field == "enabled" {
                        "enabled".into()
                    } else {
                        field.into()
                    },
                ]
            };
        }
    }
    keys
}

/// 【应用设置】【JSON 修改】沿 keys 更新 root 的 value，空值表示恢复继承，返回类型错误
fn set_json(root: &mut Value, keys: &[String], value: &Value) -> io::Result<()> {
    let (key, tail) = keys
        .split_first()
        .ok_or_else(|| io::Error::other("empty setting path"))?;
    if root.is_boolean() {
        *root = serde_json::json!({"enabled": root.clone()});
    } else if root.is_null() {
        *root = serde_json::json!({});
    }
    let table = root
        .as_object_mut()
        .ok_or_else(|| io::Error::other("setting parent must be an object"))?;
    if tail.is_empty() {
        if value.is_null() {
            table.remove(key);
        } else {
            table.insert(key.clone(), value.clone());
        }
        let alternate = if key.contains('_') {
            alias(key)
        } else {
            snake(key)
        };
        if alternate != *key {
            table.remove(&alternate);
        }
    } else {
        set_json(
            table.entry(key).or_insert_with(|| serde_json::json!({})),
            tail,
            value,
        )?;
    }
    Ok(())
}

/// 【应用设置】【TOML 转换】将 value 转为同类型 TOML，支持有限浮点、数组和内联对象，返回转换错误
fn toml_value(value: &Value) -> io::Result<toml_edit::Value> {
    match value {
        Value::String(text) => Ok(text.as_str().into()),
        Value::Bool(value) => Ok((*value).into()),
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Ok(integer.into())
            } else {
                number
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .map(Into::into)
                    .ok_or_else(|| io::Error::other("invalid number"))
            }
        }
        Value::Array(values) => {
            let mut array = toml_edit::Array::new();
            for value in values {
                array.push(toml_value(value)?);
            }
            Ok(array.into())
        }
        Value::Object(values) => {
            let mut table = toml_edit::InlineTable::new();
            for (key, value) in values {
                table.insert(key, toml_value(value)?);
            }
            Ok(table.into())
        }
        Value::Null => Err(io::Error::other("null is only valid for resetting a field")),
    }
}

/// 【应用设置】【TOML 修改】沿 keys 更新 table，保留字段注释，value 为空时删除覆盖
fn set_toml(
    table: &mut dyn toml_edit::TableLike,
    keys: &[String],
    value: &Value,
) -> io::Result<()> {
    let (key, tail) = keys
        .split_first()
        .ok_or_else(|| io::Error::other("empty setting path"))?;
    if tail.is_empty() {
        if value.is_null() {
            table.remove(key);
            let alternate = if key.contains('_') {
                alias(key)
            } else {
                snake(key)
            };
            if alternate != *key {
                table.remove(&alternate);
            }
            return Ok(());
        }
        let mut replacement = toml_value(value)?;
        if let Some(previous) = table.get(key).and_then(toml_edit::Item::as_value) {
            *replacement.decor_mut() = previous.decor().clone();
        }
        table.insert(key, toml_edit::Item::Value(replacement));
        let alternate = if key.contains('_') {
            alias(key)
        } else {
            snake(key)
        };
        if alternate != *key {
            table.remove(&alternate);
        }
    } else {
        let child = table
            .entry(key)
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
        if let Some(enabled) = child.as_bool() {
            let mut replacement = toml_edit::Table::new();
            replacement.insert("enabled", toml_edit::value(enabled));
            *child = toml_edit::Item::Table(replacement);
        }
        let child = child
            .as_table_like_mut()
            .ok_or_else(|| io::Error::other("setting parent must be a table"))?;
        set_toml(child, tail, value)?;
    }
    Ok(())
}

/// 【应用设置】【原子提交】将 changes 中的路径和值共同写入 path，失败保持原文件，返回保存结果
pub fn write(path: &Path, changes: &[(String, Value)]) -> io::Result<()> {
    for (path, value) in changes {
        if !value.is_null()
            && let Some((section, key)) = path.rsplit_once('.')
        {
            super::preferences::validate(section, key, value)?;
        }
    }
    super::persistence::update(path, |text| {
        let mut root = super::document::root(path, text)?;
        let json = super::document::is_json(path, text);
        if json {
            super::legacy::decode(text)?;
        } else {
            toml::from_str::<super::Config>(text).map_err(io::Error::other)?;
        }
        let mut changes = changes.to_vec();
        let resets: Vec<_> = changes
            .iter()
            .flat_map(|(path, _)| {
                path.strip_prefix("export.image_frame.")
                    .into_iter()
                    .flat_map(|key| {
                        super::export::ImageFrame::aliases(key)
                            .iter()
                            .filter(move |alias| snake(alias) != key)
                            .map(|alias| (format!("export.image_frame.{alias}"), Value::Null))
                    })
            })
            .collect();
        changes.extend(resets);
        if changes.iter().any(|(path, _)| path == "ui.theme")
            && let Some(colors) = root.get("theme").and_then(Value::as_object)
        {
            for (name, value) in colors {
                if value
                    .as_str()
                    .and_then(crate::theme::Rgba::parse_hex)
                    .is_some_and(|color| super::interface_theme::is_builtin(name, color))
                {
                    changes.push((format!("theme.{name}"), Value::Null));
                }
            }
        }
        let changes: Vec<_> = changes
            .iter()
            .map(|(path, value)| {
                let mut keys = path_in(&root, path, json);
                if path.starts_with("translation.") && keys.len() == 3 {
                    keys[2] = snake(&keys[2]);
                }
                (keys, value)
            })
            .collect();
        for (keys, value) in &changes {
            set_json(&mut root, keys, value)?;
        }
        if json {
            let text = serde_json::to_string_pretty(&root).map_err(io::Error::other)? + "\n";
            super::legacy::decode(&text)?;
            return Ok((text, ()));
        }
        let mut document = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(io::Error::other)?;
        for (keys, value) in &changes {
            set_toml(document.as_table_mut(), keys, value)?;
        }
        toml::from_str::<super::Config>(&document.to_string()).map_err(io::Error::other)?;
        Ok((document.to_string(), ()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【应用设置】【旧文档回归】旧快捷键与边框在原路径更新，布尔外框升级为对象后保留启用状态
    #[test]
    fn legacy_paths_remain_authoritative_and_boolean_sections_expand() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        std::fs::write(&path, r#"{"shortcuts":{"tools":{"rectangle":["R"]}},"pinnedWindow":{"border":true},"export":{"imageFrame":false},"unknown":17}"#).unwrap();
        write(
            &path,
            &[
                ("keys.tool_rectangle".into(), serde_json::json!(["Ctrl+R"])),
                ("pinned.border_width".into(), serde_json::json!(2.5)),
                ("export.image_frame.padding".into(), 24.into()),
            ],
        )
        .unwrap();
        let root = source(&path).unwrap();
        assert_eq!(
            root["shortcuts"]["tools"]["rectangle"],
            serde_json::json!(["Ctrl+R"])
        );
        assert!(root.get("keys").is_none());
        assert_eq!(root["pinnedWindow"]["border"]["enabled"], true);
        assert_eq!(root["pinnedWindow"]["border"]["width"], 2.5);
        assert_eq!(root["export"]["imageFrame"]["enabled"], false);
        assert_eq!(root["unknown"], 17);
        write(&path, &[("pinned.border_width".into(), Value::Null)]).unwrap();
        assert!(
            source(&path).unwrap()["pinnedWindow"]["border"]
                .get("width")
                .is_none()
        );
    }

    /// 【应用设置】【保存拒绝】语法损坏时写入返回错误且原文件逐字节保留，修复必须经恢复入口
    #[test]
    fn invalid_document_is_never_partially_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        let original = "[ui]\nlanguage = \"unfinished\n";
        std::fs::write(&path, original).unwrap();
        assert!(write(&path, &[("ui.theme".into(), "dark".into())]).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
    }

    /// 【应用设置】【兼容分组】修改旧捕获分组保留同组鼠标、识别与放大区配置，外框恢复删除所有旧半径别名
    #[test]
    fn capture_aliases_and_frame_reset_preserve_effective_values() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        std::fs::write(&path, r#"{"screenshot":{"includeCursor":true,"selectionLoupe":true,"regionDetection":false},"export":{"imageFrame":{"enabled":true,"radius":36}}}"#).unwrap();
        write(
            &path,
            &[
                ("startup.selection_loupe".into(), false.into()),
                ("export.image_frame.corner_radius".into(), Value::Null),
            ],
        )
        .unwrap();
        let config = super::super::preferences::load(&path).unwrap();
        assert!(config.startup.include_cursor);
        assert_eq!(config.startup.selection_loupe, Some(false));
        assert!(!config.region_detection.enabled);
        assert_eq!(config.export.image_frame.corner_radius, 18.0);
        assert!(source(&path).unwrap().get("capture").is_none());
    }
    /// 【应用设置】【嵌套保存回归】无参数，浮点与嵌套配置可往返，两种格式保留未知值且支持恢复继承
    #[test]
    fn nested_numbers_preserve_unknown_fields() {
        for extension in ["toml", "json"] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join(format!("config.{extension}"));
            std::fs::write(
                &path,
                if extension == "toml" {
                    "# keep\n[custom]\nvalue=7\n"
                } else {
                    "{\"custom\":{\"value\":7}}"
                },
            )
            .unwrap();
            write(
                &path,
                &[
                    ("pinned.border_width".into(), 1.5.into()),
                    ("export.image_frame.shadow_opacity".into(), 0.4.into()),
                ],
            )
            .unwrap();
            let loaded = super::super::preferences::read(&path).unwrap();
            assert_eq!(
                value(&loaded, "pinned.border_width"),
                &serde_json::json!(1.5)
            );
            assert_eq!(
                value(&loaded, "export.image_frame.shadow_opacity"),
                &serde_json::json!(0.4)
            );
            write(&path, &[("pinned.border_width".into(), Value::Null)]).unwrap();
            let root =
                super::super::document::root(&path, &std::fs::read_to_string(&path).unwrap())
                    .unwrap();
            assert_eq!(root["custom"]["value"], 7);
            assert!(value(&root, "pinned.border_width").is_null());
        }
    }
}
