//! 【扩展命令】【配置兼容】按需读取原版 extensions.json，支持数组、单对象和 commands 对象

use serde_json::Value;
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionCommand {
    pub name: String,
    pub command: String,
    pub working_directory: String,
    pub description: String,
    pub save_image: bool,
    pub close_on_start: bool,
}

/// 【扩展命令】【图片字段】返回 command 是否包含原版图片占位符；仅区域参数不要求保存图片
pub fn uses_image(command: &str) -> bool {
    ["{image}", "{imagePath}", "{imageUrl}"]
        .iter()
        .any(|field| command.contains(field))
}

/// 【扩展命令】【配置路径】返回用户 mark-shot 配置目录下 extensions.json，不创建缺失文件，无参数
pub fn config_path() -> io::Result<PathBuf> {
    dirs::config_dir()
        .map(|directory| directory.join("mark-shot/extensions.json"))
        .ok_or_else(|| io::Error::other("configuration directory is unavailable"))
}

/// 【扩展命令】【配置读取】读取 path 的有界 UTF-8 JSON；不存在时返回空列表，格式或读取错误保留给界面显示
pub fn load(path: &Path) -> io::Result<Vec<ExtensionCommand>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    if !file.metadata()?.is_file() {
        return Err(io::Error::other(
            "extension configuration is not a regular file",
        ));
    }
    let mut text = String::new();
    file.take(1024 * 1024 + 1).read_to_string(&mut text)?;
    if text.len() > 1024 * 1024 {
        return Err(io::Error::other("extension configuration is too large"));
    }
    parse(&text)
}

/// 【扩展命令】【对象解析】按 text 返回有效命令并保留原顺序；错误类型不隐式转换，默认启动后关闭编辑器
pub fn parse(text: &str) -> io::Result<Vec<ExtensionCommand>> {
    let root: Value = serde_json::from_str(text).map_err(io::Error::other)?;
    let values = match &root {
        Value::Array(values) => values.clone(),
        Value::Object(object) => match object.get("commands") {
            Some(Value::Array(values)) => values.clone(),
            _ if object.get("command").is_some_and(Value::is_string) => vec![root.clone()],
            _ => Vec::new(),
        },
        _ => {
            return Err(io::Error::other(
                "Expected a JSON array, a command object, or an object with a commands array",
            ));
        }
    };
    if values.is_empty() {
        return Err(io::Error::other("No extension commands found"));
    }
    let mut commands = Vec::new();
    for value in values {
        let Some(object) = value.as_object() else {
            continue;
        };
        let string = |key: &str| {
            object
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        let name = string("name");
        let command = string("command");
        if name.is_empty() || command.is_empty() {
            continue;
        }
        let working_directory = object
            .get("workingDirectory")
            .and_then(Value::as_str)
            .or_else(|| object.get("cwd").and_then(Value::as_str))
            .unwrap_or_default()
            .trim()
            .to_owned();
        commands.push(ExtensionCommand {
            name,
            description: string("description"),
            working_directory,
            save_image: uses_image(&command)
                || object
                    .get("saveImage")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                || object
                    .get("needsImage")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            close_on_start: object
                .get("closeOnStart")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            command,
        });
    }
    Ok(commands)
}
