//! 【选区尺寸】【界面字体】按原版 Sans Serif 定向加载常规与半粗体，不扫描系统字体目录

use cosmic_text::{FontSystem, Weight};

/// 【选区尺寸】【字体创建】返回原版无衬线界面家族的独立字库，无参数，仅在首次显示尺寸面板时调用
pub(crate) fn new() -> FontSystem {
    let mut fonts = super::bundled();
    let family = super::text_faces::family(&mut fonts, "Sans Serif", false, false);
    super::text_faces::family(&mut fonts, &family, true, false);
    fonts.db_mut().set_sans_serif_family(family);
    fonts
}

/// 【选区尺寸】【缺字准备】按 text 在 fonts 检查非 ASCII 字符，仅加载实际缺失的半粗体回退，无返回值
pub(crate) fn prepare(fonts: &mut FontSystem, text: &str) {
    for character in text.chars().filter(|ch| !ch.is_ascii() && !ch.is_control()) {
        let ids = fonts
            .db()
            .faces()
            .filter(|face| {
                face.families
                    .iter()
                    .all(|(name, _)| name != super::NUMERALS_FAMILY)
            })
            .map(|face| face.id)
            .collect::<Vec<_>>();
        if ids.into_iter().any(|id| {
            fonts
                .get_font(id, Weight(600))
                .is_some_and(|font| font.as_swash().charmap().map(character) != 0)
        }) {
            continue;
        }
        super::text_faces::fallback(fonts, "Sans Serif", true, false, character);
    }
}
