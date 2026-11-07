//! 【Windows字体】【元数据目录】只在首次字体请求时读取系统目录，按家族、样式和实际缺字选择文件

use cosmic_text::fontdb::{Database, Family, Query, Source, Stretch, Style, Weight};
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub(super) struct Record {
    pub families: Vec<String>,
    pub languages: Vec<String>,
    pub styles: Vec<String>,
    pub foundry: String,
    pub file: String,
    pub index: u32,
}

/// 【Windows字体】【目录缓存】无参数，返回系统字体元数据，启动时不调用，不保留所有字体的字形缓存
fn database() -> &'static Database {
    static FONTS: OnceLock<Database> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut fonts = Database::new();
        fonts.load_system_fonts();
        fonts
    })
}

/// 【Windows字体】【家族记录】无参数，返回文件家族与索引，实际字形仍按预览需求加载
pub(super) fn records() -> Vec<Record> {
    database()
        .faces()
        .filter_map(|face| {
            let path = match &face.source {
                Source::File(path) | Source::SharedFile(path, _) => path,
                _ => return None,
            };
            Some(Record {
                families: face.families.iter().map(|entry| entry.0.clone()).collect(),
                languages: Vec::new(),
                styles: vec![format!("{:?}", face.style)],
                foundry: String::new(),
                file: path.to_string_lossy().into_owned(),
                index: face.index,
            })
        })
        .collect()
}

/// 【Windows字体】【定向匹配】按pattern家族、样式及可选Unicode字符返回实际文件与家族，未知家族回退系统无衬线
pub(super) fn matched(pattern: &str) -> Option<(String, String)> {
    let mut parts = pattern.split(':');
    let name = parts.next()?;
    let mut weight = Weight::NORMAL;
    let mut style = Style::Normal;
    let mut character = None;
    for part in parts {
        if part == "slant=italic" {
            style = Style::Italic;
        }
        if let Some(value) = part.strip_prefix("weight=") {
            weight = Weight(match value {
                "thin" => 100,
                "extralight" => 200,
                "light" => 300,
                "medium" => 500,
                "demibold" => 600,
                "bold" => 700,
                "extrabold" => 800,
                "black" => 900,
                _ => 400,
            });
        }
        if let Some(value) = part.strip_prefix("charset=") {
            character = u32::from_str_radix(value, 16).ok().and_then(char::from_u32);
        }
    }
    let fonts = database();
    let generic =
        name.eq_ignore_ascii_case("sans-serif") || name.eq_ignore_ascii_case("Sans Serif");
    let families = if generic {
        vec![Family::Name("Segoe UI"), Family::SansSerif]
    } else {
        vec![
            Family::Name(name),
            Family::Name("Segoe UI"),
            Family::SansSerif,
        ]
    };
    let query = fonts.query(&Query {
        families: &families,
        weight,
        stretch: Stretch::Normal,
        style,
    });
    let mut candidates = query.into_iter().collect::<Vec<_>>();
    if character.is_some() {
        for name in [
            "Microsoft YaHei",
            "Yu Gothic",
            "Meiryo",
            "SimSun",
            "Malgun Gothic",
            "Segoe UI Symbol",
        ] {
            if let Some(id) = fonts.query(&Query {
                families: &[Family::Name(name)],
                weight,
                stretch: Stretch::Normal,
                style,
            }) {
                candidates.push(id);
            }
        }
        candidates.extend(fonts.faces().map(|face| face.id));
    }
    for id in candidates {
        if let Some(character) = character
            && !fonts
                .with_face_data(id, |data, index| {
                    ttf_parser::Face::parse(data, index)
                        .ok()
                        .is_some_and(|face| face.glyph_index(character).is_some())
                })
                .unwrap_or(false)
        {
            continue;
        }
        let face = fonts.face(id)?;
        let path = match &face.source {
            Source::File(path) | Source::SharedFile(path, _) => path,
            _ => continue,
        };
        return Some((
            path.to_string_lossy().into_owned(),
            face.families.first()?.0.clone(),
        ));
    }
    None
}
