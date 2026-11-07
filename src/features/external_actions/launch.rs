//! 【外部动作】【启动交付】原版图片与区域占位符通过独立参数或引用后的 shell 值交付，启动失败删除临时图片

use super::{Action, desktop_exec, extensions::ExtensionCommand};
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// 【扩展命令】【参数引用】按 value 返回 POSIX shell 单参数引用，特殊字符不能触发额外命令
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

/// 【扩展命令】【命令展开】用 extension、image 和 geometry 替换原版四类占位符，返回 shell 命令；仅需区域时不附加图片
pub fn expand_extension(
    extension: &ExtensionCommand,
    image: Option<&Path>,
    geometry: &str,
) -> io::Result<String> {
    let file = image
        .map(|path| {
            path.to_str()
                .ok_or_else(|| io::Error::other("image path is not UTF-8"))
        })
        .transpose()?;
    let url = image.map(desktop_exec::file_url).transpose()?;
    let replacements = [
        ("{image}", file),
        ("{imagePath}", file),
        ("{imageUrl}", url.as_deref()),
        ("{slurp}", Some(geometry)),
    ];
    let mut remaining = extension.command.as_str();
    let mut result = String::new();
    let mut used_image = false;
    // 1. 【扩展命令】【单次替换】占位符只从原始命令扫描，文件名包含花括号时也不会再次替换
    while let Some((offset, field, value)) = replacements
        .iter()
        .filter_map(|(field, value)| remaining.find(field).map(|offset| (offset, *field, *value)))
        .min_by_key(|item| item.0)
    {
        result.push_str(&remaining[..offset]);
        let value = value.ok_or_else(|| io::Error::other("extension requires an image"))?;
        if field == "{slurp}" && value.is_empty() {
            return Err(io::Error::other("selection geometry is empty"));
        }
        result.push_str(&shell_quote(value));
        used_image |= field != "{slurp}";
        remaining = &remaining[offset + field.len()..];
    }
    result.push_str(remaining);
    if extension.save_image && !used_image {
        result.push(' ');
        result.push_str(&shell_quote(
            file.ok_or_else(|| io::Error::other("extension requires an image"))?,
        ));
    }
    Ok(result)
}

/// 【扩展命令】【工作目录】按 raw 展开当前用户的波浪号目录并返回路径，不展开 shell 或环境变量
fn working_directory(raw: &str) -> io::Result<PathBuf> {
    if raw == "~" || raw.starts_with("~/") {
        let directory =
            dirs::home_dir().ok_or_else(|| io::Error::other("home directory is unavailable"))?;
        Ok(if raw == "~" {
            directory
        } else {
            directory.join(&raw[2..])
        })
    } else {
        Ok(PathBuf::from(raw))
    }
}

/// 【外部动作】【图片暂存】把 png 写入 directory 下私有随机 PNG，返回自动删除句柄；路径直到启动成功才保留
pub fn temporary_image(png: &[u8], directory: &Path) -> io::Result<tempfile::NamedTempFile> {
    let mut file = tempfile::Builder::new()
        .prefix("mark-shot-open-")
        .suffix(".png")
        .tempfile_in(directory)?;
    file.write_all(png)?;
    file.flush()?;
    Ok(file)
}

/// 【外部动作】【独立启动】为 action 交付 image 与 geometry，返回启动结果；成功只表示程序完成 exec，不等待外部业务结束
pub fn start(action: &Action, image: Option<&Path>, geometry: &str) -> io::Result<()> {
    let mut command = match action {
        Action::Desktop(app) => {
            let args = desktop_exec::expand(
                app,
                image.ok_or_else(|| io::Error::other("desktop application requires an image"))?,
            )?;
            let mut command = Command::new(&args[0]);
            command.args(&args[1..]);
            command
        }
        Action::Extension(extension) => {
            let mut command = Command::new("/bin/sh");
            command.args(["-c", &expand_extension(extension, image, geometry)?]);
            if !extension.working_directory.is_empty() {
                command.current_dir(working_directory(&extension.working_directory)?);
            }
            command
        }
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::utils::helper_process::spawn(command)?;
    Ok(())
}
