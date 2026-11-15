//! 【普通标注旋转验证】【实际正文】矢量正文直接旋转，文字与导出沿用真实绘制入口

use super::*;
use cosmic_text::SwashCache;
use mark_shot::{
    renderer::{Painter, draw_annotation},
    types::MarkerShape,
};
use std::collections::HashMap;
use tiny_skia::Pixmap;

/// 【普通标注旋转验证】【画布输出】按 annotation 和 scale 返回实际正文画布，不绘制选框或原图，不初始化识别模型
fn render(annotation: &Annotation, scale: f32) -> Pixmap {
    render_at(annotation, scale, (256.0 * scale) as u32)
}

/// 【普通标注旋转验证】【物理画布】按 annotation、scale 和 side 返回指定物理边长的实际正文，设备比例独立于图像几何
fn render_at(annotation: &Annotation, scale: f32, side: u32) -> Pixmap {
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = SwashCache::new();
    let mut editors = HashMap::new();
    let mut canvas = Pixmap::new(side, side).unwrap();
    draw_annotation(
        &mut Painter::new(&mut canvas)
            .with_scale(scale)
            .with_text(&mut fonts, &mut cache),
        annotation,
        false,
        &mut editors,
        &HashMap::new(),
        None,
    );
    canvas
}

/// 【普通标注旋转验证】【矢量样本】返回中心位于128、128的五类正文，保证旋转不是对称图像的无效比较
pub(super) fn vectors() -> Vec<Annotation> {
    let (start, end) = ((70.0, 85.0), (186.0, 171.0));
    vec![
        annotation(AnnotationShape::Rectangle {
            radius: 0.0,
            fixed_ratio: false,
            start,
            end,
            filled: true,
        }),
        annotation(AnnotationShape::Circle {
            fixed_ratio: false,
            start,
            end,
            filled: true,
        }),
        annotation(AnnotationShape::Marker {
            radius: 0.0,
            start,
            end,
            kind: MarkerShape::Triangle,
            filled: true,
        }),
        annotation(AnnotationShape::Pen {
            points: vec![start, (105.0, 160.0), end],
        }),
        annotation(AnnotationShape::Highlighter {
            points: vec![start, (105.0, 160.0), end],
            flat_ends: [false; 2],
        }),
    ]
}

/// 【普通标注旋转验证】【内部覆盖】按 source 和 actual 核对90度旋转后的实色内部，排除抗锯齿边缘，不用仅有边界变化代替正文
fn assert_quarter_turn(source: &Pixmap, actual: &Pixmap) {
    let width = source.width() as usize;
    assert_eq!(
        (source.width(), source.height()),
        (actual.width(), actual.height())
    );
    let source_pixels = source.pixels();
    let mut interiors = 0;
    for y in 1..width - 1 {
        for x in 1..width - 1 {
            let index = y * width + x;
            let expected = source_pixels[index];
            if expected.alpha() == 0
                || ![index - 1, index + 1, index - width, index + width]
                    .into_iter()
                    .all(|neighbor| source_pixels[neighbor] == expected)
            {
                continue;
            }
            interiors += 1;
            let actual = actual.pixels()[x * width + (width - 1 - y)];
            for (a, b) in [
                (actual.red(), expected.red()),
                (actual.green(), expected.green()),
                (actual.blue(), expected.blue()),
                (actual.alpha(), expected.alpha()),
            ] {
                assert!(
                    a.abs_diff(b) <= 2,
                    "source ({x},{y}) expected {expected:?}, actual {actual:?}"
                );
            }
        }
    }
    assert!(interiors > 30);
}

/// 【普通标注旋转验证】【真实矢量】五类正文在90度后保留实色与笔宽，非旋转正文或仅旋转选框必须失败，无参数、无返回值
#[test]
fn all_ordinary_vector_bodies_rotate_with_the_annotation_pose() {
    for mut annotation in vectors() {
        let source = render(&annotation, 1.0);
        annotation.rotation_degrees = 90.0;
        let actual = render(&annotation, 1.0);
        assert_ne!(source.data(), actual.data());
        assert_quarter_turn(&source, &actual);
    }
}

/// 【普通标注旋转验证】【实际导出缩放】五类矢量的导出坐标缩放与设备比例映射必须保留同一正文位置，无参数、无返回值
#[test]
fn scaled_export_renders_the_same_rotated_vector_geometry() {
    for mut annotation in vectors() {
        annotation.rotation_degrees = 45.0;
        let physical = render(&annotation, 2.0);
        let exported = render_at(&annotation.scaled(2.0), 1.0, 512);
        assert_eq!(physical.data(), exported.data());
    }
}

/// 【普通标注旋转验证】【文字样本】返回经实际字体排版的文字，平移中心到128、128，供整数像素的四分之一圈验证
pub(super) fn text() -> Annotation {
    let mut annotation = annotation(AnnotationShape::Text {
        frame: Default::default(),
        appearance: Default::default(),
        start: (80.0, 80.0),
        content: "Rotation".into(),
        font_size: 20.0,
        bold: false,
        italic: false,
        outline: None,
    });
    let mut fonts = mark_shot::fonts::bundled();
    let mut editors = HashMap::new();
    let editor = mark_shot::tools::text::ensure_text_editor(&annotation, &mut editors, &mut fonts);
    mark_shot::tools::text::update_text_bbox_inline(&mut annotation, editor, &mut fonts);
    let center = annotation.rotation_center();
    annotation.translate_mut(128.0 - center.0, 128.0 - center.1);
    annotation
}

/// 【普通标注旋转验证】【真实文字】旋转正文保持原始字形，不能只移动文字锚点或旋转外围控制点，无参数、无返回值
#[test]
fn text_body_rotates_its_actual_glyphs() {
    let mut annotation = text();
    let source = render(&annotation, 1.0);
    annotation.rotation_degrees = 90.0;
    let actual = render(&annotation, 1.0);
    assert_ne!(source.data(), actual.data());
    let width = source.width() as usize;
    let mut differences = 0;
    for y in 0..width {
        for x in 0..width {
            differences += usize::from(
                source.pixels()[y * width + x] != actual.pixels()[x * width + width - 1 - y],
            );
        }
    }
    if std::env::var_os("MARK_SHOT_RENDER_ANNOTATION_ROTATION").is_some() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/generic-rotation-r1/render");
        std::fs::create_dir_all(&root).unwrap();
        source.save_png(root.join("text-original.png")).unwrap();
        actual.save_png(root.join("text-quarter-turn.png")).unwrap();
    }
    assert_eq!(
        differences, 0,
        "glyph pixels differed under an integer quarter turn"
    );
}
