//! 【贴图测试】【剪贴板比例】验证 DPR、文件图片回退、文本缩放与 Wayland 重复检测

use super::super::{content, html, render, rich_text, source::Clipboard};

/// 【贴图测试】【重复检测】无参数，验证 Wayland 缺少序号时不启用重复贴图，无返回值
#[test]
fn clipboard_without_sequence_does_not_repeat() {
    assert!(!Clipboard::default().has_sequence());
    assert!(
        Clipboard {
            sequence: 7,
            ..Default::default()
        }
        .has_sequence()
    );
}

/// 【贴图测试】【剪贴板DPR】无参数，验证原始图片保留内嵌 DPR 且普通 PNG 不伪造比例，无返回值
#[test]
fn clipboard_image_keeps_only_real_embedded_density() {
    let png = tiny_skia::Pixmap::new(2, 2).unwrap().encode_png().unwrap();
    let plain = Clipboard {
        image: Some(png.clone()),
        ..Default::default()
    };
    let content = content::image(&plain, false, false, 1.0).unwrap();
    assert_eq!(
        content.source,
        crate::features::pinned_image::density::Source::ClipboardImage
    );
    assert_eq!(content.density, None);
    let marked = crate::features::capture_history::png_metadata::with_scale(&png, 1.25).unwrap();
    let with_dpr = Clipboard {
        image: Some(marked),
        ..Default::default()
    };
    let content = content::image(&with_dpr, false, false, 1.0).unwrap();
    assert_eq!(
        content.source,
        crate::features::pinned_image::density::Source::ClipboardImage
    );
    assert_eq!(content.density, Some(1.25));
}

/// 【贴图测试】【PNG/JPG文件】无参数，验证复制的 PNG 与 JPG 文件都按图片贴图而不是路径文字，无返回值
#[test]
fn copied_png_and_jpg_files_both_resolve_as_images() {
    use image::ImageEncoder;
    let directory = tempfile::tempdir().unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(4, 4).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(10, 20, 30, 255));
    let png = directory.path().join("sample.png");
    pixmap.save_png(&png).unwrap();
    let jpg = directory.path().join("sample.jpg");
    let mut file = std::fs::File::create(&jpg).unwrap();
    let rgb: Vec<u8> = pixmap
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    image::codecs::jpeg::JpegEncoder::new(&mut file)
        .write_image(&rgb, 4, 4, image::ExtendedColorType::Rgb8)
        .unwrap();
    for path in [png, jpg] {
        let clipboard = Clipboard {
            files: vec![path.clone()],
            ..Default::default()
        };
        let content = content::image(&clipboard, false, false, 1.0).unwrap();
        assert_eq!(
            content.source,
            crate::features::pinned_image::density::Source::FileImage,
            "{path:?}"
        );
    }
}

/// 【贴图测试】【文件图片回退】无参数，验证文件不可解码时优先使用剪贴板图片预览，无返回值
#[test]
fn copied_file_with_image_preview_falls_back_to_the_image() {
    let png = tiny_skia::Pixmap::new(2, 2).unwrap().encode_png().unwrap();
    let clipboard = Clipboard {
        files: vec![std::path::PathBuf::from("/definitely/missing/image.png")],
        image: Some(png),
        ..Default::default()
    };
    for repeated in [false, true] {
        let content = content::image(&clipboard, repeated, false, 1.0).unwrap();
        assert_eq!(
            content.source,
            crate::features::pinned_image::density::Source::ClipboardImage
        );
    }
}

/// 【贴图测试】【文本比例】无参数，验证文本与富文本按 1.25 渲染并返回对应密度，无返回值
#[test]
fn text_and_rich_text_follow_the_requested_scale() {
    let base = tiny_skia::Pixmap::decode_png(&render::text_image("hello", 1.0).unwrap()).unwrap();
    let scaled =
        tiny_skia::Pixmap::decode_png(&render::text_image("hello", 1.25).unwrap()).unwrap();
    assert!(scaled.width() > base.width());
    assert!(scaled.height() > base.height());

    let rich = html::parse("<p>hello</p>");
    let base = tiny_skia::Pixmap::decode_png(&rich_text::image(&rich, 1.0).unwrap()).unwrap();
    let scaled = tiny_skia::Pixmap::decode_png(&rich_text::image(&rich, 1.25).unwrap()).unwrap();
    assert!(scaled.width() > base.width());
    assert!(scaled.height() > base.height());

    let clipboard = Clipboard {
        text: Some("hello".into()),
        ..Default::default()
    };
    let content = content::image(&clipboard, false, false, 1.25).unwrap();
    assert_eq!(content.density, Some(1.25));
    assert_eq!(content.pin_scale(), 1.25);
}
