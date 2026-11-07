//! 【贴图菜单测试】【真实绘制】验证菜单自身准备中文字体，无需主编辑器或后台字库

use super::*;
use crate::ui::language::Language;

/// 【贴图菜单测试】【中文排版】通过实际菜单绘制入口后，每个中文菜单字形都必须有效，不使用序号字体代替正文
#[test]
fn chinese_pin_menu_draw_prepares_every_localized_glyph() {
    let mut fonts = crate::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    let rows = menu::ITEMS
        .iter()
        .map(|item| Row {
            label: Language::Chinese.text(item.label).into(),
            enabled: item.action.is_some(),
            checked: false,
            danger: false,
            submenu: false,
        })
        .collect::<Vec<_>>();
    for row in rows.iter().filter(|row| !row.label.is_empty()) {
        assert!(
            !row.label.is_ascii(),
            "untranslated menu label: {}",
            row.label
        );
    }
    let mut canvas = Pixmap::new(menu::WIDTH, menu::height()).unwrap();
    draw(
        &mut canvas,
        &rows,
        None,
        (menu::WIDTH, menu::height()),
        1.0,
        &mut fonts,
        &mut cache,
        &Default::default(),
    );
    if let Ok(root) = std::env::var("MARK_SHOT_PIN_MENU_FRAMES") {
        canvas
            .save_png(std::path::Path::new(&root).join("chinese-menu.png"))
            .unwrap();
    }
    for row in rows.iter().filter(|row| !row.label.is_empty()) {
        let (buffer, _, _) = crate::renderer::text::shape_single_line(
            &mut fonts,
            &row.label,
            12.0,
            cosmic_text::Weight::NORMAL,
            cosmic_text::Style::Normal,
        );
        for glyph in buffer.layout_runs().flat_map(|run| run.glyphs.iter()) {
            assert_ne!(
                glyph.glyph_id, 0,
                "missing glyph in menu label: {}",
                row.label
            );
            let family = &fonts.db().face(glyph.font_id).unwrap().families[0].0;
            assert_ne!(family, crate::fonts::NUMERALS_FAMILY);
        }
    }
    let count = fonts.db().len();
    draw(
        &mut canvas,
        &rows,
        Some(7),
        (menu::WIDTH, menu::height()),
        1.0,
        &mut fonts,
        &mut cache,
        &Default::default(),
    );
    assert_eq!(
        fonts.db().len(),
        count,
        "reopening the menu must reuse loaded font files"
    );
}

/// 【贴图菜单测试】【语言列表】子菜单中的全部目标语言正常排版，只有英文菜单时不补充系统字库
#[test]
fn language_submenu_has_no_missing_glyphs_and_ascii_menu_stays_bundled() {
    let mut fonts = crate::fonts::bundled();
    let initial = fonts.db().len();
    crate::fonts::ui_fallback::ensure_text(&mut fonts, "Rotate Left / Copy Selected Text");
    assert_eq!(fonts.db().len(), initial);
    for language in [Language::Chinese, Language::English] {
        let labels = crate::translation::language::labels(language);
        for label in labels {
            crate::fonts::ui_fallback::ensure_text(&mut fonts, label);
            let (buffer, _, _) = crate::renderer::text::shape_single_line(
                &mut fonts,
                label,
                12.0,
                cosmic_text::Weight::NORMAL,
                cosmic_text::Style::Normal,
            );
            assert!(
                buffer
                    .layout_runs()
                    .flat_map(|run| run.glyphs.iter())
                    .all(|glyph| glyph.glyph_id != 0),
                "missing target language glyph: {label}"
            );
        }
    }
}
