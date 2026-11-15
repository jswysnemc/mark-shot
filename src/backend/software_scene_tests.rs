//! 【软件场景测试】【像素契约】两种通道、完整透明度和重复损坏均与原合成顺序一致

use super::*;
use crate::image_source::ImageSource;
use tiny_skia::{Color, ColorU8, Pixmap, Rect};

/// 【软件场景测试】【通道与损坏】无参数，对比全部透明度并确保损坏区域之外保持逐字节一致
#[test]
fn sparse_scene_matches_legacy_composition_in_both_channel_orders() {
    let mut original = Pixmap::new(257, 19).unwrap();
    original.fill(Color::from_rgba8(113, 57, 231, 255));
    let background = ImageSource::memory(original);
    let mut foreground = Pixmap::new(257, 19).unwrap();
    for (index, pixel) in foreground.pixels_mut().iter_mut().enumerate() {
        *pixel = ColorU8::from_rgba(29, 71, 131, index as u8).premultiply();
    }
    let selection = Rect::from_xywh(17.0, 3.0, 93.0, 11.0);
    let frame = Frame {
        foreground: &mut foreground,
        background: &background,
        selection,
        hovered: None,
        opaque: false,
        damage: Some((3, 1, 219, 15)),
    };
    let mut rgba = vec![97; frame.foreground.data().len()];
    let mut bgra = rgba.clone();
    compose(&frame, &mut rgba, false).unwrap();
    compose(&frame, &mut bgra, true).unwrap();
    for y in 0..19 {
        for x in 0..257 {
            let offset = (y * 257 + x) * 4;
            if !(3..222).contains(&x) || !(1..16).contains(&y) {
                assert_eq!(&rgba[offset..offset + 4], &[97; 4]);
                assert_eq!(&bgra[offset..offset + 4], &[97; 4]);
                continue;
            }
            let mut legacy = Pixmap::new(1, 1).unwrap();
            if !(17..110).contains(&x) || !(3..14).contains(&y) {
                legacy.fill(crate::config::get().selection.dim.color());
            }
            let pixel = frame.foreground.pixel(x as u32, y as u32).unwrap();
            let mut paint = tiny_skia::Paint::default();
            let color = pixel.demultiply();
            paint.set_color(Color::from_rgba8(
                color.red(),
                color.green(),
                color.blue(),
                color.alpha(),
            ));
            legacy.fill_rect(
                Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
                &paint,
                tiny_skia::Transform::identity(),
                None,
            );
            let expected = over(legacy.data().try_into().unwrap(), [113, 57, 231, 255]);
            // 1. 【软件场景测试】【量化误差】颜色解预乘再预乘最多差一，提交路径本身不执行这一步
            assert!(
                rgba[offset..offset + 4]
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.abs_diff(b) <= 1)
            );
            assert_eq!(
                &bgra[offset..offset + 4],
                &[
                    rgba[offset + 2],
                    rgba[offset + 1],
                    rgba[offset],
                    rgba[offset + 3]
                ]
            );
        }
    }
}

/// 【软件场景测试】【完整编辑器】真实渲染的选框、圆角、候选及半透明标注与旧帧比较，局部重画不能残留旧像素
#[test]
fn sparse_editor_keeps_selection_annotation_and_hover_pixels() {
    use crate::app::region_rendering_test_support as support;
    for scale in [1.0, 1.2, 1.25, 1.5, 2.0] {
        for selection in [
            None,
            Rect::from_xywh(43.0, 37.0, 417.0, 321.0),
            Rect::from_xywh(43.3, 37.7, 417.4, 321.1),
        ] {
            let mut sparse = support::editor(scale, true);
            let mut legacy = support::editor(scale, true);
            sparse.region_rendering.native_scene = true;
            for state in [&mut sparse, &mut legacy] {
                state.selection.zone = selection;
                state.annotations.push(support::annotation(
                    1,
                    0.0,
                    crate::types::AnnotationShape::Rectangle {
                        radius: 7.0,
                        fixed_ratio: false,
                        start: (75.0, 80.0),
                        end: (195.0, 170.0),
                        filled: true,
                    },
                ));
            }
            for hover in [None, Rect::from_xywh(83.0, 91.0, 217.0, 199.0), None] {
                for state in [&mut sparse, &mut legacy] {
                    state.region_detection.hovered = hover;
                    support::paint(state, None, false);
                }
                let mut expected = legacy.canvas[0].clone();
                crate::renderer::opaque_backdrop::compose(
                    &legacy.captures[0].pixmap,
                    &mut expected,
                    None,
                );
                let mut actual = vec![0; expected.data().len()];
                let hovered = hover.filter(|_| sparse.region_rendering.native_hover);
                compose(
                    &Frame {
                        foreground: &mut sparse.canvas[0],
                        background: &sparse.captures[0].pixmap,
                        selection: selection
                            .and_then(|rect| crate::renderer::scale_rect(&rect, scale)),
                        hovered: hovered.and_then(|rect| crate::renderer::scale_rect(&rect, scale)),
                        damage: None,
                        opaque: false,
                    },
                    &mut actual,
                    false,
                )
                .unwrap();
                let differences: Vec<_> = actual
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(expected.data().as_chunks::<4>().0)
                    .enumerate()
                    .filter(|(_, (a, b))| a.iter().zip(*b).any(|(a, b)| a.abs_diff(*b) > 2))
                    .take(10)
                    .collect();
                assert!(
                    differences.is_empty(),
                    "scale={scale} selection={selection:?} hover={hover:?}: {differences:?}"
                );
            }
        }
    }
}

/// 【软件场景测试】【持续局部帧】真实矩形移动只重建旧新范围的并集，提交后与完整重绘保持一致
#[test]
fn partial_sparse_frames_keep_pixels_outside_damage() {
    use crate::app::region_rendering_test_support as support;
    let mut sparse = support::editor(1.0, true);
    let mut legacy = support::editor(1.0, true);
    sparse.region_rendering.native_scene = true;
    let selection = Rect::from_xywh(23.0, 27.0, 510.0, 370.0);
    let mut actual = vec![0; sparse.canvas[0].data().len()];
    for (index, x) in [55.0, 107.0, 150.0, 73.0].into_iter().enumerate() {
        let dirty = (index > 0).then(|| Rect::from_xywh(30.0, 40.0, 300.0, 250.0).unwrap());
        for state in [&mut sparse, &mut legacy] {
            state.selection.zone = selection;
            state.annotations = vec![support::annotation(
                1,
                0.0,
                crate::types::AnnotationShape::Rectangle {
                    radius: 0.0,
                    fixed_ratio: false,
                    start: (x, 80.0),
                    end: (x + 100.0, 170.0),
                    filled: true,
                },
            )];
            state.annotations_dirty = true;
        }
        support::paint(&mut sparse, dirty, false);
        support::paint(&mut legacy, None, false);
        let mut expected = legacy.canvas[0].clone();
        crate::renderer::opaque_backdrop::compose(&legacy.captures[0].pixmap, &mut expected, None);
        let damage = dirty.and_then(|rect| crate::renderer::rect_bounds(&rect, 640, 480));
        compose(
            &Frame {
                foreground: &mut sparse.canvas[0],
                background: &sparse.captures[0].pixmap,
                selection,
                hovered: None,
                damage,
                opaque: false,
            },
            &mut actual,
            false,
        )
        .unwrap();
        let maximum = actual
            .iter()
            .zip(expected.data())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(
            maximum <= 2,
            "frame={index}, maximum channel difference={maximum}"
        );
    }
}
