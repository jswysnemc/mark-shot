//! 【界面字体测试】【真实配置】用隔离 Fontconfig 配置与真实字体验证抗锯齿、微调、子像素和缓存隔离

use super::*;
use cosmic_text::{
    SwashCache,
    fontdb::{Family, Query, Stretch, Style, Weight},
};
use std::path::Path;

/// 【界面字体测试】【目标字体】无参数，返回实际系统字体字库、文件键及家族，不要求固定发行版字体名称
fn font() -> (FontSystem, FaceKey, String) {
    let fonts = crate::fonts::ui_system::new();
    let id = fonts
        .db()
        .query(&Query {
            families: &[Family::SansSerif],
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            style: Style::Normal,
        })
        .unwrap();
    let face = fonts.db().face(id).unwrap();
    let file = match &face.source {
        Source::File(path) | Source::SharedFile(path, _) => path.clone(),
        _ => panic!("系统字体未匹配到文件"),
    };
    let key = FaceKey {
        file,
        index: face.index,
    };
    let family = face.families[0].0.clone();
    (fonts, key, family)
}

/// 【界面字体测试】【配置文件】按 directory、file 与 rules 创建隔离字体配置，返回配置对象，不修改环境变量
fn config(directory: &Path, file: &Path, rules: &str) -> Config {
    let path = directory.join("fonts.conf");
    let parent = file
        .parent()
        .unwrap()
        .to_string_lossy()
        .replace('&', "&amp;")
        .replace('<', "&lt;");
    let cache = directory.join("cache").to_string_lossy().into_owned();
    std::fs::write(&path, format!("<?xml version=\"1.0\"?><!DOCTYPE fontconfig SYSTEM \"fonts.dtd\"><fontconfig><dir>{parent}</dir><cachedir>{cache}</cachedir>{rules}</fontconfig>")).unwrap();
    Config::from_file(&path).unwrap()
}

/// 【界面字体测试】【渲染器】按 config 创建独立测试缓存和 FreeType 库，返回绘制器，不访问线程缓存
fn raster(config: Config) -> Raster {
    Raster {
        faces: HashMap::new(),
        settings: HashMap::new(),
        glyphs: HashMap::new(),
        bytes: 0,
        library: Library::init().unwrap(),
        config,
    }
}

/// 【界面字体测试】【字形键】按 face 与 library 返回字符 a 的十八像素字形键，供比较真实轮廓
fn key(face: FaceKey, library: &Library) -> GlyphKey {
    let glyph = library
        .new_face(&face.file, face.index as isize)
        .unwrap()
        .get_char_index('a' as usize)
        .unwrap() as u16;
    GlyphKey {
        face,
        glyph,
        size: 18.0_f32.to_bits(),
        weight: 400,
        flags: CacheKeyFlags::empty(),
        x: SubpixelBin::One,
        y: SubpixelBin::Zero,
        lcd: true,
    }
}

/// 【界面字体测试】【规则生效】无参数，读取实际字体的全部渲染选项，并确认关闭抗锯齿产生二值遮罩
#[test]
fn fontconfig_font_rules_change_real_glyph_rasterization() {
    let (_, face, family) = font();
    let directory = tempfile::tempdir().unwrap();
    let config = config(
        directory.path(),
        &face.file,
        r#"
        <match target="font">
          <edit name="antialias" mode="assign"><bool>false</bool></edit>
          <edit name="hinting" mode="assign"><bool>false</bool></edit>
          <edit name="hintstyle" mode="assign"><const>hintnone</const></edit>
          <edit name="autohint" mode="assign"><bool>true</bool></edit>
          <edit name="rgba" mode="assign"><const>bgr</const></edit>
          <edit name="lcdfilter" mode="assign"><const>lcdlight</const></edit>
          <edit name="embeddedbitmap" mode="assign"><bool>false</bool></edit>
        </match>"#,
    );
    let settings = config.settings(&face.file, face.index, &family, 18.0, 400);
    assert_eq!(
        settings,
        Settings {
            antialias: false,
            hinting: false,
            hintstyle: 0,
            autohint: true,
            rgba: 2,
            lcd_filter: 2,
            embedded_bitmap: false
        }
    );
    let mut raster = raster(config);
    let key = key(face, &raster.library);
    let mono = raster
        .image(
            key.clone(),
            &family,
            Some(Arc::new(std::fs::read(&key.face.file).unwrap())),
        )
        .unwrap();
    assert_eq!(mono.content, Content::Mask);
    assert!(mono.data.iter().all(|value| *value == 0 || *value == 255));
    assert!(mono.data.contains(&255));
    let face = &raster.faces[&key.face];
    let gray = render(
        &raster.library,
        face,
        &key,
        Settings {
            rgba: 0,
            ..Settings::default()
        },
    )
    .unwrap();
    assert!(gray.data.iter().any(|value| *value > 0 && *value < 255));
    assert_ne!(gray.data, raster.glyphs[&key].data);
}

/// 【界面字体测试】【子像素顺序】无参数，真实 LCD 遮罩按 RGB/BGR 顺序变化，透明绘制请求使用单通道
#[test]
fn subpixel_order_and_grayscale_requests_are_respected() {
    let (_, face, _) = font();
    let library = Library::init().unwrap();
    for filter in [0, 1, 2, 3] {
        set_lcd_filter(&library, filter).unwrap();
    }
    let key = key(face.clone(), &library);
    let face = library.new_face(&face.file, face.index as isize).unwrap();
    let rgb = render(
        &library,
        &face,
        &key,
        Settings {
            rgba: 1,
            ..Settings::default()
        },
    )
    .unwrap();
    let bgr = render(
        &library,
        &face,
        &key,
        Settings {
            rgba: 2,
            ..Settings::default()
        },
    )
    .unwrap();
    assert_eq!(rgb.content, Content::Subpixel);
    assert_eq!((rgb.width, rgb.height), (bgr.width, bgr.height));
    assert!(
        rgb.data
            .as_chunks::<3>()
            .0
            .iter()
            .any(|pixel| pixel[0] != pixel[2])
    );
    for (rgb, bgr) in rgb
        .data
        .as_chunks::<3>()
        .0
        .iter()
        .zip(bgr.data.as_chunks::<3>().0.iter())
    {
        assert_eq!([rgb[0], rgb[1], rgb[2]], [bgr[2], bgr[1], bgr[0]]);
    }
    let gray = render(
        &library,
        &face,
        &GlyphKey { lcd: false, ..key },
        Settings {
            rgba: 1,
            ..Settings::default()
        },
    )
    .unwrap();
    assert_eq!(gray.content, Content::Mask);
}

/// 【界面字体测试】【缓存隔离】无参数，重复绘制复用同一遮罩与字体对象，界面绘制不修改标注 Swash 字形缓存
#[test]
fn ui_glyphs_reuse_storage_and_leave_annotation_cache_unchanged() {
    let (mut fonts, face, family) = font();
    let directory = tempfile::tempdir().unwrap();
    let mut raster = raster(config(directory.path(), &face.file, ""));
    let key = key(face, &raster.library);
    let pointer = raster
        .image(
            key.clone(),
            &family,
            Some(Arc::new(std::fs::read(&key.face.file).unwrap())),
        )
        .unwrap()
        .data
        .as_ptr();
    let bytes = raster.bytes;
    assert_eq!(
        raster
            .image(
                key.clone(),
                &family,
                Some(Arc::new(std::fs::read(&key.face.file).unwrap()))
            )
            .unwrap()
            .data
            .as_ptr(),
        pointer
    );
    assert_eq!(
        (
            raster.faces.len(),
            raster.glyphs.len(),
            raster.settings.len(),
            raster.bytes
        ),
        (1, 1, 1, bytes)
    );
    let id = fonts
        .db()
        .query(&Query {
            families: &[Family::SansSerif],
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            style: Style::Normal,
        })
        .unwrap();
    let cache_key = CacheKey::new(
        id,
        key.glyph,
        18.0,
        (0.25, 0.0),
        Weight::NORMAL,
        CacheKeyFlags::empty(),
    )
    .0;
    let mut cache = SwashCache::new();
    let original = cache
        .get_image(&mut fonts, cache_key)
        .as_ref()
        .unwrap()
        .data
        .clone();
    assert!(with_image(&fonts, cache_key, true, |_| {}));
    assert_eq!(
        cache
            .get_image(&mut fonts, cache_key)
            .as_ref()
            .unwrap()
            .data,
        original
    );
}

/// 【界面字体测试】【字重规则】无参数，同一文件与字号的常规、半粗体分别匹配 Fontconfig 规则，不共享错误设置
#[test]
fn fontconfig_weight_rules_are_applied_and_cached_per_weight() {
    let (_, face, family) = font();
    let directory = tempfile::tempdir().unwrap();
    let mut raster = raster(config(
        directory.path(),
        &face.file,
        r#"
        <match target="pattern"><test name="weight" compare="more_eq"><const>demibold</const></test>
          <edit name="antialias" mode="assign"><bool>false</bool></edit></match>
        <match target="pattern"><test name="weight" compare="less"><const>demibold</const></test>
          <edit name="antialias" mode="assign"><bool>true</bool></edit></match>"#,
    ));
    let key = key(face.clone(), &raster.library);
    assert!(
        raster
            .config
            .settings(&face.file, face.index, &family, 18.0, 400)
            .antialias
    );
    assert!(
        !raster
            .config
            .settings(&face.file, face.index, &family, 18.0, 600)
            .antialias
    );
    raster
        .image(
            key.clone(),
            &family,
            Some(Arc::new(std::fs::read(&key.face.file).unwrap())),
        )
        .unwrap();
    raster
        .image(
            GlyphKey {
                weight: 600,
                ..key.clone()
            },
            &family,
            Some(Arc::new(std::fs::read(&key.face.file).unwrap())),
        )
        .unwrap();
    assert_eq!(raster.settings.len(), 2);
    assert!(raster.settings[&(face.clone(), 18.0_f32.to_bits(), 400)].antialias);
    assert!(!raster.settings[&(face, 18.0_f32.to_bits(), 600)].antialias);
}

/// 【界面字体测试】【LCD 微调】无参数，轻微和完整微调在相同 RGB 设置下产生不同真实遮罩，输出模式均保持 LCD
#[test]
fn lcd_rendering_keeps_the_requested_hinting_target() {
    let (_, source, _) = font();
    let library = Library::init().unwrap();
    let mut key = key(source.clone(), &library);
    let face = library
        .new_face(&source.file, source.index as isize)
        .unwrap();
    let mut different = false;
    for size in [11.0_f32, 13.0, 17.0, 19.0] {
        key.size = size.to_bits();
        let slight = render(
            &library,
            &face,
            &key,
            Settings {
                rgba: 1,
                hintstyle: 1,
                ..Settings::default()
            },
        )
        .unwrap();
        let full = render(
            &library,
            &face,
            &key,
            Settings {
                rgba: 1,
                hintstyle: 3,
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(
            (slight.content, full.content),
            (Content::Subpixel, Content::Subpixel)
        );
        different |=
            slight.data != full.data || (slight.width, slight.height) != (full.width, full.height);
    }
    assert!(different, "LCD 输出忽略了轻微微调配置");
}
