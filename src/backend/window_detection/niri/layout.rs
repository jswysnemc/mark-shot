//! 【niri 窗口识别】【布局配置】有界读取 gaps、struts 与相对 include

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub(super) struct Layout {
    pub gaps: f64,
    pub struts: [f64; 4],
}

impl Default for Layout {
    /// 【niri 窗口识别】【默认布局】无参数，返回原版默认十六逻辑像素间隙及零独占边距
    fn default() -> Self {
        Self {
            gaps: 16.0,
            struts: [0.0; 4],
        }
    }
}

/// 【niri 窗口识别】【配置目录】无参数，返回实际 XDG 配置目录
pub(super) fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
}

/// 【niri 窗口识别】【去除注释】按 text 去除两种注释并保留字符串和换行，返回可解析文本
pub(super) fn uncomment(text: &str) -> String {
    let mut chars = text.chars().peekable();
    let mut output = String::new();
    let mut quoted = false;
    let mut escaped = false;
    while let Some(value) = chars.next() {
        if quoted {
            output.push(value);
            if escaped {
                escaped = false;
            } else if value == '\\' {
                escaped = true;
            } else if value == '"' {
                quoted = false;
            }
        } else if value == '"' {
            quoted = true;
            output.push(value);
        } else if value == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for value in chars.by_ref() {
                if value == '\n' {
                    output.push(value);
                    break;
                }
            }
        } else if value == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(value) = chars.next() {
                if value == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
                if value == '\n' {
                    output.push(value);
                }
            }
        } else {
            output.push(value);
        }
    }
    output
}

/// 【niri 窗口识别】【有限文件】按 path 和 budget 读取文本，返回有效 UTF-8；总预算不足或文件过大时返回空值
pub(super) fn bounded_text(path: &Path, budget: &mut usize) -> Option<String> {
    use std::io::Read;
    if !std::fs::metadata(path).ok()?.is_file() {
        return None;
    }
    let mut file = std::fs::File::open(path).ok()?.take(*budget as u64 + 1);
    let mut text = String::new();
    file.read_to_string(&mut text).ok()?;
    if text.len() > *budget {
        return None;
    }
    *budget -= text.len();
    Some(text)
}

/// 【niri 窗口识别】【配置展开】按 path、seen、budget 和 depth 展开相对 include，返回去除注释的有限文本
pub(super) fn expand(
    path: &Path,
    seen: &mut HashSet<PathBuf>,
    budget: &mut usize,
    depth: usize,
) -> String {
    if depth >= 16 {
        return String::new();
    }
    let Ok(real) = std::fs::canonicalize(path) else {
        return String::new();
    };
    if !seen.insert(real) {
        return String::new();
    }
    let Some(text) = bounded_text(path, budget) else {
        return String::new();
    };
    let text = uncomment(&text);
    let mut output = String::new();
    for line in text.lines() {
        if let Some(target) = line.trim().strip_prefix("include ") {
            let target = target.trim().trim_end_matches(';').trim_matches('"');
            output.push_str(&expand(
                &path.parent().unwrap_or(Path::new(".")).join(target),
                seen,
                budget,
                depth + 1,
            ));
        } else {
            output.push_str(line);
        }
        output.push('\n');
    }
    output
}

/// 【niri 窗口识别】【布局读取】无参数，读取原版布局配置并返回 gaps 与四边 struts
pub(super) fn load() -> Layout {
    let path = std::env::var_os("NIRI_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_home().join("niri/config.kdl"));
    parse(&expand(&path, &mut HashSet::new(), &mut (1024 * 1024), 0))
}

/// 【niri 窗口识别】【布局解析】按 text 提取顶层 layout 的 gaps、struts，返回有限数值配置
pub(super) fn parse(text: &str) -> Layout {
    let mut config = Layout::default();
    let mut depth = 0i32;
    let mut layout_depth = None;
    let mut struts_depth = None;
    for line in text.lines() {
        let tokens: Vec<_> = line.split_whitespace().collect();
        let Some(name) = tokens.first().copied() else {
            continue;
        };
        let opens = line.matches('{').count() as i32;
        if depth == 0 && name == "layout" && opens > 0 {
            layout_depth = Some(depth + 1);
        } else if layout_depth == Some(depth) && name == "struts" && opens > 0 {
            struts_depth = Some(depth + 1);
        } else if let Some(value) = tokens
            .get(1)
            .and_then(|token| token.trim_end_matches(';').parse::<f64>().ok())
            && value.is_finite()
            && value.abs() <= 65536.0
        {
            if layout_depth == Some(depth) && name == "gaps" {
                config.gaps = value;
            } else if struts_depth == Some(depth)
                && let Some(index) = ["top", "bottom", "left", "right"]
                    .iter()
                    .position(|edge| *edge == name)
            {
                config.struts[index] = value;
            }
        }
        depth += opens - line.matches('}').count() as i32;
        if struts_depth.is_some_and(|current| depth < current) {
            struts_depth = None;
        }
        if layout_depth.is_some_and(|current| depth < current) {
            layout_depth = None;
        }
    }
    config
}
