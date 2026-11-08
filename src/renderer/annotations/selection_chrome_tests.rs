//! 【标注选框测试】【实际绘制】验证通用选框使用原版完整虚线与橙色方块，不通过独立替身绘图

use super::*;

/// 【标注选框测试】【矩形标注】返回固定逻辑矩形，不预先扩大通用选框范围
fn rectangle() -> Annotation {
    Annotation {
        style_scale: 1.0,
        rotation_degrees: 0.0,
        frozen_bounds: None,
        id: 7,
        shape: AnnotationShape::Rectangle {
            radius: 0.0,
            fixed_ratio: false,
            start: (80.0, 80.0),
            end: (180.0, 160.0),
            filled: false,
        },
        color: Color::from_rgba8(30, 40, 50, 255),
        shadow_color: Color::TRANSPARENT,
        stroke_width: 3.0,
        bbox: Rect::from_xywh(80.0, 80.0, 100.0, 80.0).unwrap(),
    }
}

/// 【标注选框测试】【原版缩放点】实际绘制的上边中点应为9像素橙色方块，四角分段线不能替代缩放点
#[test]
fn ordinary_annotation_uses_orange_square_handle_on_its_actual_bounds() {
    let mut canvas = Pixmap::new(260, 220).unwrap();
    draw_annotation_handles_only(&mut Painter::new(&mut canvas), &rectangle());
    let pixel = canvas.pixel(130, 80).unwrap().demultiply();
    assert_eq!(
        (pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()),
        (251, 146, 60, 255)
    );
}

/// 【标注选框测试】【分数局部恢复】实际画笔选框在完整画布与部分损坏缓冲必须逐字节一致，不涉及历史或正文重采样
#[test]
fn fractional_pen_controls_are_identical_after_partial_repaint() {
    for scale in [1.0, 1.5, 1280.0 / 853.0, 2.0] {
        for phase in [0.0, 0.125, 0.37, 0.625] {
            let mut annotation = rectangle();
            annotation.shape = AnnotationShape::Pen {
                points: vec![(180.0 + phase, 180.0 + phase), (260.8562, 244.62823)],
            };
            annotation.stroke_width = 12.0;
            let mut whole = Pixmap::new((420.0 * scale) as u32, (350.0 * scale) as u32).unwrap();
            whole.fill(Color::from_rgba8(30, 34, 38, 255));
            draw_annotation_handles_only(
                &mut Painter::new(&mut whole).with_scale(scale),
                &annotation,
            );
            for dirty in [
                Rect::from_xywh(140.0, 130.0, 200.0, 190.0).unwrap(),
                Rect::from_xywh(30.666_666, 164.666_67, 350.0, 140.0).unwrap(),
            ] {
                let mut partial = whole.clone();
                Painter::new(&mut partial)
                    .with_scale(scale)
                    .clipped(Some(dirty), |painter| {
                        // 1. 【标注选框测试】【真实损坏恢复】先恢复本次缓冲内全部背景，其余像素保留之前完整绘制结果
                        painter.canvas().fill(Color::from_rgba8(30, 34, 38, 255));
                        draw_annotation_handles_only(painter, &annotation);
                    });
                let mismatch = whole
                    .pixels()
                    .iter()
                    .zip(partial.pixels())
                    .filter(|(a, b)| a != b)
                    .count();
                assert_eq!(mismatch, 0, "scale={scale}, phase={phase}, dirty={dirty:?}");
            }
        }
    }
}

/// 【标注选框测试】【生产参考】读取真实 Qt 生产选框函数返回的边界、手柄和删除坐标，不使用当前绘制构造预期
fn reference() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/qt-annotation-selection/reference.json"
    ))
    .unwrap()
}

/// 【标注选框测试】【坐标解码】按 value 返回一个原始二维坐标，供生产参考传输使用
fn point(value: &serde_json::Value) -> (f32, f32) {
    (
        value[0].as_f64().unwrap() as f32,
        value[1].as_f64().unwrap() as f32,
    )
}

/// 【标注选框测试】【删除几何】72张原版样本逐个核对旋转外移与窗口边缘限制，三种像素比例不改变逻辑位置
#[test]
fn deletion_geometry_matches_production_qt_function_at_rotated_edges() {
    use crate::features::annotation_selection::chrome;
    let data = reference();
    let samples = data["samples"].as_array().unwrap();
    assert_eq!(samples.len(), 72);
    for sample in samples {
        let values = &sample["bounds"];
        let rect = Rect::from_xywh(
            values[0].as_f64().unwrap() as f32,
            values[1].as_f64().unwrap() as f32,
            values[2].as_f64().unwrap() as f32,
            values[3].as_f64().unwrap() as f32,
        )
        .unwrap();
        let expected = &sample["delete"];
        let actual = chrome::delete_rect(rect, point(&sample["corners"][1]), true, (600.0, 360.0));
        for (index, value) in [actual.x(), actual.y(), actual.width(), actual.height()]
            .into_iter()
            .enumerate()
        {
            assert!(
                (f64::from(value) - expected[index].as_f64().unwrap()).abs() < 0.0001,
                "{sample}: {actual:?}"
            );
        }
    }
}

/// 【标注选框测试】【实际栅格】完整共用组件与72张原版生产绘制图比较轮廓及实色，抗锯齿边缘单独容许微小差异
#[test]
fn common_chrome_rasters_match_production_qt_controls_and_delete_button() {
    use crate::ui::annotation_selection::draw;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let artifacts = root.join("target/annotation-chrome-render-r1");
    let save = std::env::var_os("MARK_SHOT_RENDER_SELECTION_CHROME").is_some();
    if save {
        std::fs::create_dir_all(&artifacts).unwrap();
    }
    for sample in reference()["samples"].as_array().unwrap() {
        let dpr = sample["dpr"].as_f64().unwrap() as f32;
        let name = sample["render"].as_str().unwrap();
        let mut canvas = Pixmap::new((600.0 * dpr) as u32, (360.0 * dpr) as u32).unwrap();
        let mut painter = Painter::new(&mut canvas).with_scale(dpr);
        let corners = [
            point(&sample["corners"][0]),
            point(&sample["corners"][1]),
            point(&sample["corners"][2]),
            point(&sample["corners"][3]),
        ];
        draw::frame(&mut painter, corners, 1.0);
        for handle in sample["handles"].as_array().unwrap() {
            draw::square(&mut painter, point(handle), 1.0);
        }
        draw::rotation(
            &mut painter,
            (
                (corners[0].0 + corners[1].0) * 0.5,
                (corners[0].1 + corners[1].1) * 0.5,
            ),
            point(&sample["rotation"]),
            1.0,
        );
        let button = &sample["delete"];
        draw::delete(
            &mut painter,
            Rect::from_xywh(
                button[0].as_f64().unwrap() as f32,
                button[1].as_f64().unwrap() as f32,
                20.0,
                20.0,
            )
            .unwrap(),
        );
        if save {
            canvas.save_png(artifacts.join(name)).unwrap();
        }
        let expected = Pixmap::load_png(
            root.join("tests/fixtures/qt-annotation-selection/render")
                .join(name),
        )
        .unwrap();
        let mut union = 0;
        let mut mismatch = 0;
        let mut solid = 0;
        let mut wrong_color = 0;
        let mut alpha_error = 0usize;
        let mut expected_alpha = 0usize;
        let expected_pixels = expected.pixels();
        let stride = expected.width() as usize;
        for (index, (a, b)) in canvas.pixels().iter().zip(expected_pixels).enumerate() {
            let (a_mask, b_mask) = (a.alpha() >= 128, b.alpha() >= 128);
            alpha_error += usize::from(a.alpha().abs_diff(b.alpha()));
            expected_alpha += usize::from(b.alpha());
            if a_mask || b_mask {
                union += 1;
                mismatch += usize::from(a_mask != b_mask);
            }
            // 1. 【标注选框测试】【实色内部】白色边缘叠在不透明底色上时仍然不透明，不能仅按 alpha 排除抗锯齿混色
            let interior = index > stride
                && index + stride < expected_pixels.len()
                && index % stride > 0
                && index % stride + 1 < stride
                && [index - 1, index + 1, index - stride, index + stride]
                    .into_iter()
                    .all(|neighbor| expected_pixels[neighbor] == *b);
            if b.alpha() == 255 && interior {
                solid += 1;
                wrong_color += usize::from(
                    a.alpha().abs_diff(b.alpha()) > 2
                        || (i32::from(a.red()) - i32::from(b.red())).abs() > 2
                        || (i32::from(a.green()) - i32::from(b.green())).abs() > 2
                        || (i32::from(a.blue()) - i32::from(b.blue())).abs() > 2,
                );
            }
        }
        assert!(union > 100 && solid > 50);
        assert!(
            alpha_error * 100 < expected_alpha * 5,
            "{name}: alpha error {alpha_error}/{expected_alpha}; threshold mask differed at {mismatch}/{union} pixels"
        );
        assert_eq!(
            wrong_color, 0,
            "{name}: {wrong_color}/{solid} interior color pixels differed"
        );
    }
}
