//! 【打开方式】【命令展开】桌面命令直接传递参数，沿用原版双引号拆分，不通过 shell 执行

use super::desktop_apps::DesktopApp;
use std::{io, path::Path};

/// 【打开方式】【命令拆分】按 text 返回原版 QProcess 双引号参数；三个连续双引号表示字面双引号，单引号及反斜杠保持普通字符
pub fn split(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut token = String::new();
    let mut quoted = false;
    let mut quotes = 0;
    for ch in text.chars() {
        if ch == '"' {
            quotes += 1;
            if quotes == 3 {
                token.push('"');
                quotes = 0;
            }
            continue;
        }
        if quotes == 1 {
            quoted = !quoted;
        }
        quotes = 0;
        if ch.is_whitespace() && !quoted {
            if !token.is_empty() {
                result.push(std::mem::take(&mut token));
            }
        } else {
            token.push(ch);
        }
    }
    if !token.is_empty() {
        result.push(token);
    }
    result
}

/// 【打开方式】【文件地址】把 absolute path 转换为原版可读 file URL，保留空格与 Unicode，控制字符和地址分隔符保持编码；相对路径返回错误
pub fn file_url(path: &Path) -> io::Result<String> {
    let encoded = String::from(
        url::Url::from_file_path(path)
            .map_err(|()| io::Error::other("image path is not absolute"))?,
    );
    let bytes = encoded.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&encoded[index + 1..index + 3], 16)
            && byte >= 32
            && !matches!(byte, 127 | b'%' | b'#' | b'?')
        {
            output.push(byte);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).map_err(io::Error::other)
}

/// 【打开方式】【参数展开】用 app 和 image 替换文件、地址、名称与入口字段，返回程序及参数；不递归解释替换后的用户数据
pub fn expand(app: &DesktopApp, image: &Path) -> io::Result<Vec<String>> {
    let file = image
        .to_str()
        .ok_or_else(|| io::Error::other("image path is not UTF-8"))?;
    let url = file_url(image)?;
    let desktop = app.desktop_path.to_string_lossy();
    let mut used_file = false;
    let mut args = Vec::new();
    for argument in split(&app.exec) {
        if argument == "%i" {
            continue;
        }
        let mut expanded = String::new();
        let mut chars = argument.chars();
        while let Some(ch) = chars.next() {
            if ch != '%' {
                expanded.push(ch);
                continue;
            }
            match chars.next() {
                Some('f' | 'F') => {
                    expanded.push_str(file);
                    used_file = true;
                }
                Some('u' | 'U') => {
                    expanded.push_str(&url);
                    used_file = true;
                }
                Some('c') => expanded.push_str(&app.name),
                Some('k') => expanded.push_str(&desktop),
                Some('%') => expanded.push('%'),
                Some('d' | 'D' | 'n' | 'N' | 'v' | 'm') => {}
                Some(other) => {
                    expanded.push('%');
                    expanded.push(other);
                }
                None => expanded.push('%'),
            }
        }
        if !expanded.trim().is_empty() {
            args.push(expanded);
        }
    }
    if args.is_empty() {
        return Err(io::Error::other("desktop command is empty"));
    }
    if !used_file {
        args.push(file.to_owned());
    }
    Ok(args)
}
