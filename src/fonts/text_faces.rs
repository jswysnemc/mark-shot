//! 【文字字体】【定向匹配】只匹配当前字体与样式，不替换其他界面的内置字体

use cosmic_text::{FontSystem, Style, Weight};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

type MatchCache = HashMap<String, Option<(String, String)>>;

/// 【文字字体】【匹配缓存】查询 pattern 并缓存路径和家族；失败也缓存，避免每帧创建外部进程
fn matched(pattern: String) -> Option<(String, String)> {
    static MATCHES: OnceLock<Mutex<MatchCache>> = OnceLock::new();
    let mut cache = MATCHES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    cache
        .entry(pattern.clone())
        .or_insert_with(|| {
            let output = std::process::Command::new("fc-match")
                .args(["-f", "%{file}\n%{family[0]}", &pattern])
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let text = std::str::from_utf8(&output.stdout).ok()?;
            let mut lines = text.lines();
            Some((lines.next()?.to_owned(), lines.next()?.to_owned()))
        })
        .clone()
}

/// 【文字字体】【样式匹配】按 family、bold、italic 查询并加载字体，返回可用于排版的实际家族；失败时采用已加载回退
pub fn family(fonts: &mut FontSystem, family: &str, bold: bool, italic: bool) -> String {
    resolve(fonts, family, bold, italic, None)
}

/// 【文字字体】【缺字回退】按 family、样式和 character 查询系统首选字体，加载必要文件并返回家族；只用于主字体缺少字形的片段
pub fn fallback(
    fonts: &mut FontSystem,
    family: &str,
    bold: bool,
    italic: bool,
    character: char,
) -> String {
    resolve(fonts, family, bold, italic, Some(character))
}

/// 【文字字体】【定向加载】用 family、样式与可选 character 匹配字体；返回已有或新加载家族，不重复读取字体文件
fn resolve(
    fonts: &mut FontSystem,
    family: &str,
    bold: bool,
    italic: bool,
    character: Option<char>,
) -> String {
    let generic = family.is_empty()
        || family.eq_ignore_ascii_case("Sans Serif")
        || family.eq_ignore_ascii_case("sans-serif");
    let mut pattern = format!(
        "{}:weight={}:slant={}",
        if generic { "sans-serif" } else { family },
        if bold { "demibold" } else { "regular" },
        if italic { "italic" } else { "roman" }
    );
    if let Some(character) = character {
        pattern.push_str(&format!(":charset={:x}", u32::from(character)));
    }
    if let Some((file, resolved)) = matched(pattern) {
        if !generic
            && character.is_none()
            && !resolved.eq_ignore_ascii_case(family)
            && let Some(available) = loaded_family(fonts, family)
        {
            return available;
        }
        // 1. 【文字字体】【文件缓存】按实际字体文件判断是否加载，常规字体不能代替同家族的半粗体
        if fonts.db().faces().any(|face| match &face.source {
            cosmic_text::fontdb::Source::File(path)
            | cosmic_text::fontdb::Source::SharedFile(path, _) => {
                path == std::path::Path::new(&file)
            }
            _ => false,
        }) {
            return resolved.to_owned();
        }
        if fonts.db_mut().load_font_file(&file).is_ok() {
            return resolved.to_owned();
        }
    }
    if let Some(available) = loaded_family(fonts, family) {
        return available;
    }
    fonts
        .db()
        .faces()
        .find_map(|face| face.families.first().map(|item| item.0.clone()))
        .unwrap_or_else(|| "Inter".into())
}

/// 【文字字体】【已有家族】在 fonts 中按 family 查找已经加载的家族，返回标准名称；内置字体不要求系统安装
fn loaded_family(fonts: &FontSystem, family: &str) -> Option<String> {
    fonts.db().faces().find_map(|face| {
        face.families
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(family))
            .map(|(name, _)| name.clone())
    })
}

/// 【文字字体】【行高】由 fonts、family、pixels 与样式返回字体上升部、下降部和行间距之和，不使用旧固定倍数
pub fn line_height(
    fonts: &mut FontSystem,
    family: &str,
    pixels: f32,
    bold: bool,
    italic: bool,
) -> f32 {
    let weight = if bold { Weight(600) } else { Weight::NORMAL };
    let id = fonts.db().query(&cosmic_text::fontdb::Query {
        families: &[cosmic_text::fontdb::Family::Name(family)],
        weight,
        stretch: cosmic_text::fontdb::Stretch::Normal,
        style: if italic { Style::Italic } else { Style::Normal },
    });
    if let Some(font) = id.and_then(|id| fonts.get_font(id, weight)) {
        let metrics = font.as_swash().metrics(&[]).scale(pixels);
        return (metrics.ascent + metrics.descent + metrics.leading)
            .ceil()
            .max(pixels);
    }
    pixels * 1.4
}
