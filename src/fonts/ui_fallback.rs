//! 【界面字体】【按需缺字】仅加载当前菜单或提示实际缺失的字体，不扫描整个系统目录

use cosmic_text::{FontSystem, Weight};

/// 【界面字体】【文字准备】按fonts与text补充缺失字形，无返回值；内置西文字体和序号字体保持独立
pub fn ensure_text(fonts: &mut FontSystem, text: &str) {
    if text.is_ascii() {
        return;
    }
    let ids = |fonts: &FontSystem| {
        fonts
            .db()
            .faces()
            .filter(|face| {
                face.families
                    .iter()
                    .all(|(name, _)| name != super::NUMERALS_FAMILY)
            })
            .map(|face| face.id)
            .collect::<Vec<_>>()
    };
    let mut loaded = ids(fonts);
    let mut faces = loaded
        .iter()
        .filter_map(|id| fonts.get_font(*id, Weight::NORMAL))
        .collect::<Vec<_>>();
    // 1. 【界面字体】【按需回退】只查询当前缺字，新加载的文件覆盖后续字符，不重复读取已有文件
    for character in text.chars().filter(|ch| !ch.is_ascii() && !ch.is_control()) {
        if faces
            .iter()
            .any(|font| font.as_swash().charmap().map(character) != 0)
        {
            continue;
        }
        super::text_faces::fallback(fonts, "Sans Serif", false, false, character);
        for id in ids(fonts) {
            if !loaded.contains(&id) {
                loaded.push(id);
                if let Some(font) = fonts.get_font(id, Weight::NORMAL) {
                    faces.push(font);
                }
            }
        }
    }
}
