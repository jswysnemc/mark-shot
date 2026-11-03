use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::arguments::CaptureOptions;
use crate::AppResult;

#[derive(Debug, Serialize)]
pub(super) struct Report {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub output: String,
    pub error: Option<String>,
}

/// 【截图】【文件名】将显示器名称中的非文件名字符替换为下划线，防止路径穿越
fn safe_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

/// 【截图】【输出路径】为单张或多显示器捕获计算文件路径；多张捕获必须写入目录
pub(super) fn destination(
    options: &CaptureOptions,
    output: &str,
    multiple: bool,
) -> AppResult<PathBuf> {
    let directory = options.destination.is_dir()
        || options.destination.extension().is_none()
        || options
            .destination
            .as_os_str()
            .to_string_lossy()
            .ends_with('/');
    if !directory {
        if multiple {
            return Err("multiple displays require an output directory".into());
        }
        return Ok(options.destination.clone());
    }
    let base = options.output_name.as_deref().unwrap_or("mark-shot");
    if base.is_empty()
        || Path::new(base).components().count() != 1
        || matches!(base, "." | "..")
        || base.contains(['/', '\\'])
    {
        return Err("--output-name must be a plain non-empty filename".into());
    }
    let suffix = if multiple {
        format!("-{}", safe_name(output))
    } else {
        String::new()
    };
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%f");
    Ok(options
        .destination
        .join(format!("{base}{suffix}-{stamp}.png")))
}

/// 【截图】【原子写入】把 PNG 写入同目录临时文件并原子替换目标；失败不留下半张截图
pub(super) fn write(path: &Path, png: &[u8]) -> AppResult {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(png)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

/// 【截图】【结果报告】输出一行 JSON；多显示器使用 captures 数组，单张保持原版字段
pub(super) fn print(reports: &[Report], multiple: bool) -> AppResult {
    let mut stdout = std::io::stdout().lock();
    if multiple {
        serde_json::to_writer(&mut stdout, &serde_json::json!({ "captures": reports }))?;
    } else if let Some(report) = reports.first() {
        serde_json::to_writer(&mut stdout, report)?;
    }
    writeln!(stdout)?;
    Ok(())
}
