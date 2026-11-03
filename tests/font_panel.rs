//! 【字体弹层测试】【列表状态】覆盖过滤、可见字体缓存与独立绘制，真实系统对照按显式环境开关执行

use mark_shot::{
    fonts::font_catalog::{Face, Family, FontCatalog},
    ui::font_panel::{FontPanel, state::Field},
    ui::text_field::CursorInit,
};

/// 【字体弹层测试】【目录样本】返回 count 个共享内置文件的字体，避免测试加载系统字库
fn catalog(count: usize) -> FontCatalog {
    FontCatalog {
        families: (0..count)
            .map(|index| Family {
                name: format!("Sample {index:03}"),
                faces: vec![Face {
                    file: format!(
                        "{}/assets/fonts/Inter-Regular.ttf",
                        env!("CARGO_MANIFEST_DIR")
                    ),
                    index: 0,
                }],
            })
            .collect(),
    }
}

#[test]
fn search_preserves_panel_size_and_limits_visible_rows() {
    let mut panel = FontPanel {
        catalog: Some(catalog(200)),
        family: "Sample 100".into(),
        ..Default::default()
    };
    panel.begin();
    assert!(panel.rows().len() <= 11);
    assert!(panel.offset > 0.0);
    let original = panel.layout.size;
    panel.fields.set_editing_text(Field::Search, "199".into());
    panel.filter();
    assert_eq!(panel.filtered, vec![199]);
    assert_eq!(panel.layout.size, original);
    assert_eq!(panel.offset, 0.0);
    panel
        .fields
        .begin_edit(Field::Search, "no match".into(), CursorInit::End);
    panel.filter();
    assert!(panel.rows().is_empty());
    assert!(panel.thumb().is_none());
}

#[test]
fn preview_replaces_visible_font_cache_instead_of_retaining_all_pages() {
    let catalog = catalog(200);
    let mut preview = mark_shot::ui::font_panel::preview::Preview::default();
    preview.prepare(&catalog, &[0, 1, 2]);
    assert_eq!(preview.key, vec![0, 1, 2]);
    for page in 1..20 {
        let visible = vec![page * 3, page * 3 + 1, page * 3 + 2];
        preview.prepare(&catalog, &visible);
        assert_eq!(preview.key, visible);
    }
    preview.clear();
    assert!(preview.key.is_empty());
}

/// 【字体弹层测试】【实际绘制】按 MARK_SHOT_PROPERTY_RENDER_DIR 保存真实系统目录和两类面板，输出作为原版对照证据
#[test]
#[ignore = "requires explicit system-font render directory"]
fn actual_system_font_panels() {
    use mark_shot::{
        renderer::Painter,
        ui::{icons, settings_panel::SettingsPanel, text_properties::layout::TextLayout, toolbar},
    };
    let root = std::path::PathBuf::from(
        std::env::var("MARK_SHOT_PROPERTY_RENDER_DIR").expect("render directory"),
    );
    std::fs::create_dir_all(&root).unwrap();
    let cache = icons::EXTRA_ICONS
        .iter()
        .chain(toolbar::icons::EXTRA.iter())
        .map(|svg| {
            (
                *svg,
                usvg::Tree::from_str(svg, &Default::default()).unwrap(),
            )
        })
        .collect();
    let mut fonts = mark_shot::fonts::bundled();
    let mut swash = cosmic_text::SwashCache::new();
    let mut panel = SettingsPanel::new();
    panel.text_layout = Some(TextLayout::new(30.0, 11.0, 22.0, false));
    panel.size = (438.0, 44.0);
    panel.visible = true;
    let mut image = tiny_skia::Pixmap::new(438, 44).unwrap();
    mark_shot::ui::settings_panel::draw_settings_panel(
        &mut Painter::new(&mut image)
            .with_text(&mut fonts, &mut swash)
            .with_icons(&cache),
        &mut panel,
        tiny_skia::Color::from_rgba8(255, 77, 77, 255),
        tiny_skia::Color::TRANSPARENT,
    );
    image.save_png(root.join("properties.png")).unwrap();
    let mut panel = FontPanel {
        family: "Sans Serif".into(),
        ..Default::default()
    };
    panel.begin();
    let names = panel
        .catalog
        .as_ref()
        .unwrap()
        .families
        .iter()
        .map(|family| &family.name)
        .collect::<Vec<_>>();
    std::fs::write(
        root.join("families.json"),
        serde_json::to_string_pretty(&names).unwrap(),
    )
    .unwrap();
    let mut image = tiny_skia::Pixmap::new(360, 360).unwrap();
    mark_shot::ui::font_panel::draw::draw(
        &mut Painter::new(&mut image)
            .with_text(&mut fonts, &mut swash)
            .with_icons(&cache),
        &mut panel,
    );
    image.save_png(root.join("font.png")).unwrap();
    assert!(panel.preview.key.len() <= 11);
}
