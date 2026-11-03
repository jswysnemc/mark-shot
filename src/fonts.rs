use cosmic_text::FontSystem;
pub mod font_catalog;
mod fontconfig;
pub mod text_faces;
pub mod text_spans;
use cosmic_text::fontdb::{Database, Source};
use std::sync::Arc;

const UI_FAMILY: &str = "Inter";
pub(crate) const NUMERALS_FAMILY: &str = "Mark Shot Numerals";

static BUNDLED: [&[u8]; 5] = [
    include_bytes!("../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../assets/fonts/Inter-Bold.ttf"),
    include_bytes!("../assets/fonts/Inter-Italic.ttf"),
    include_bytes!("../assets/fonts/Inter-BoldItalic.ttf"),
    include_bytes!("../assets/fonts/MarkShotNumerals.otf"),
];

// apparently since FontSystem::new() is reading all fonts in the system,
// it takes around 10ms when they are cached which is already bad, but when
// they are not cached, after reboot for example, it increases up untill
// +- 350 ms which is catastrohpic. Now it uses Inter installed with
// the screenshotter itself, to avoid working with system fonts at launch
/// 【字体】【内置字形】返回只含界面字体和二十三个序号汉字的字体系统，不扫描系统字体
pub fn bundled() -> FontSystem {
    let mut db = Database::new();
    for font in BUNDLED {
        db.load_font_source(Source::Binary(Arc::new(font)));
    }
    db.set_sans_serif_family(UI_FAMILY);
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".into());
    FontSystem::new_with_locale_and_db(locale, db)
}

/// 【字体】【后台字库】读取系统字体并返回字库，供界面初始化完成后的后台线程使用
pub fn system() -> Database {
    let mut db = Database::new();
    db.load_system_fonts();
    db
}

/// 【字体】【字库合并】把 system 的外部字体加入 font_system，不替换内置界面与序号字体；无返回值
pub fn add_system(font_system: &mut FontSystem, system: Database) {
    let db = font_system.db_mut();
    for face in system.faces() {
        if face
            .families
            .iter()
            .all(|(family, _)| family != UI_FAMILY && family != NUMERALS_FAMILY)
        {
            db.push_face_info(face.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_text::fontdb::{Family, Query, Stretch, Style, Weight};

    #[test]
    fn every_ui_style_is_bundled() {
        let fonts = bundled();
        for weight in [Weight::NORMAL, Weight::BOLD] {
            for style in [Style::Normal, Style::Italic] {
                let query = Query {
                    families: &[Family::SansSerif],
                    weight,
                    stretch: Stretch::Normal,
                    style,
                };
                let id = fonts.db().query(&query).expect("no bundled face");
                let face = fonts.db().face(id).unwrap();
                assert_eq!(face.families[0].0, UI_FAMILY);
                assert_eq!((face.weight, face.style), (weight, style));
            }
        }
    }

    #[test]
    fn system_fonts_never_replace_any_bundled_face() {
        let mut fonts = bundled();
        let mut system = Database::new();
        for font in BUNDLED {
            system.load_font_source(Source::Binary(Arc::new(font)));
        }
        add_system(&mut fonts, system);
        assert_eq!(fonts.db().len(), BUNDLED.len());
    }

    /// 【序号字体】【完整字形】内置字体覆盖中文数字、单位和全部天干，不依赖任何系统中文字体
    #[test]
    fn every_numbering_character_has_a_bundled_glyph() {
        let mut fonts = bundled();
        let mut buffer =
            cosmic_text::Buffer::new(&mut fonts, cosmic_text::Metrics::new(16.0, 20.0));
        buffer.set_size(None, None);
        buffer.set_text(
            "零一二三四五六七八九十百千甲乙丙丁戊己庚辛壬癸",
            &cosmic_text::Attrs::new()
                .family(cosmic_text::Family::Name(NUMERALS_FAMILY))
                .weight(cosmic_text::Weight::BLACK),
            cosmic_text::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut fonts, false);
        let glyphs = buffer
            .layout_runs()
            .flat_map(|run| run.glyphs.iter())
            .collect::<Vec<_>>();
        assert_eq!(glyphs.len(), 23);
        for glyph in glyphs {
            assert_ne!(glyph.glyph_id, 0);
            let face = fonts.db().face(glyph.font_id).unwrap();
            assert_eq!(face.families[0].0, NUMERALS_FAMILY);
            assert_eq!(face.weight, Weight::BLACK);
        }
    }
}
