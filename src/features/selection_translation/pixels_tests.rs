//! 【框选翻译回归】【缓存原件】实际区域显示写回与退出恢复必须支持独立文件原件

use super::{Patch, Region};
use crate::{editor::file_canvas::FileCanvas, image_source::ImageSource};
use tiny_skia::{Color, IntRect, Pixmap, Rect};

/// 【框选翻译回归】【真实写回】无参数，通过生产Region.apply修改大缓存原件并退出恢复，区域外像素及全部透明度保持一致，无返回值
#[test]
fn translated_cached_original_restores_the_exact_source_region() {
    let root = std::env::current_dir()
        .unwrap()
        .join("target/image-source-tests");
    std::fs::create_dir_all(&root).unwrap();
    let directory = tempfile::tempdir_in(root).unwrap();
    let mut original = Pixmap::new(1100, 1100).unwrap();
    original.fill(Color::from_rgba8(29, 73, 131, 137));
    let baseline = original.clone();
    let mut expected = baseline.clone();
    let source = ImageSource::store_in(original, directory.path());
    assert_eq!(source.cached_bytes(), baseline.data().len());
    let mut file = FileCanvas {
        original: source,
        viewport: Rect::from_xywh(0.0, 0.0, 1100.0, 1100.0).unwrap(),
    };
    let rect = IntRect::from_xywh(100, 200, 20, 30).unwrap();
    let saved = file.original.clone_rect(rect).unwrap();
    let region = Region {
        original: saved.clone(),
        bounds: Rect::from_xywh(100.0, 200.0, 20.0, 30.0).unwrap(),
        origin: (100.0, 200.0),
        scale: 1.0,
        patches: vec![Patch {
            index: None,
            rect,
            original: saved,
            origin: (100.0, 200.0),
            scale: (1.0, 1.0),
        }],
    };
    let mut translated = Pixmap::new(20, 30).unwrap();
    translated.fill(Color::from_rgba8(177, 41, 73, 94));
    // 1. 【框选翻译回归】【显示版本】真实生产入口写回原件，不借用或修改旧文件映射
    region.apply(&mut [], Some(&mut file), Some(&translated));
    crate::renderer::paste(&mut expected, &translated, 100, 200);
    assert_eq!(file.original.data(), expected.data());
    assert_eq!(file.original.cached_bytes(), baseline.data().len());
    // 2. 【框选翻译回归】【退出恢复】退出恢复只写回原始区域，不丢失原件分辨率或透明像素
    region.apply(&mut [], Some(&mut file), None);
    assert_eq!(file.original.data(), baseline.data());
    assert_eq!(
        (file.original.width(), file.original.height()),
        (1100, 1100)
    );
    drop(file);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}
