//! 【聚焦回归】【像素与编辑】覆盖旋转、区域渲染、导出以及撤销

use crate::features::filter::{FilterMode, RegionFilter};
use crate::image_source::PixelView;
use crate::{app::region_rendering_test_support as support, types::AnnotationShape};
use tiny_skia::{Pixmap, Rect};

/// 【聚焦回归】【场景创建】按 ellipse、rotation 和 region 创建完整选区和聚焦标注，返回编辑器
fn scene(ellipse: bool, rotation: f64, region: bool) -> crate::editor::EditorState {
    let mut state = support::editor_size((320, 240), 1.0, region);
    let bounds = Rect::from_xywh(0.0, 0.0, 320.0, 240.0).unwrap();
    state.selection.zone = Some(bounds);
    let mut filter = RegionFilter::new(
        (100.0, 70.0),
        (220.0, 170.0),
        14.0,
        if ellipse {
            FilterMode::FocusEllipse
        } else {
            FilterMode::Focus
        },
    )
    .unwrap();
    filter.radius = 12.0;
    filter.rotation = (rotation as f32).to_radians();
    let mut annotation = support::annotation(1, 0.0, AnnotationShape::Redact { filter });
    annotation.frozen_bounds = Some(bounds);
    state.annotations.push(annotation);
    state
}

/// 【聚焦回归】【像素一致】无参数，两种形状及旋转下内部保留原像素，周围变暗，区域和缓存帧一致
#[test]
fn focus_preserves_center_and_dims_every_surrounding_tile() {
    for ellipse in [false, true] {
        for rotation in [0.0, 37.0] {
            let mut state = scene(ellipse, rotation, true);
            let (png, _, _) = crate::app::render_final(&mut state).unwrap();
            let exported = Pixmap::decode_png(&png).unwrap();
            let source = state.captures[0].pixmap.pixels_view();
            assert_eq!(exported.pixel(160, 120), source.pixel(160, 120));
            for (x, y) in [(15, 15), (300, 15), (15, 220), (300, 220)] {
                let before = source.pixel(x, y).unwrap();
                let after = exported.pixel(x, y).unwrap();
                assert_eq!(after.alpha(), before.alpha());
                assert!(after.red() < before.red() || after.green() < before.green());
            }
            let actual = support::render(&mut state, None, false);
            let expected = support::render(&mut scene(ellipse, rotation, false), None, false);
            support::equal(&actual, &expected, "focus preview");
        }
    }
}

/// 【聚焦回归】【撤销恢复】无参数，属性关闭聚焦后可撤销和重做，周围重绘范围覆盖完整原图
#[test]
fn focus_property_participates_in_undo_and_full_canvas_damage() {
    let mut state = scene(false, 0.0, true);
    state.selected_tool = crate::tools::Tool::Pick;
    state.select_annotation_ids([1]);
    let bounds = state.selection.zone.unwrap();
    assert_eq!(state.annotations[0].damage_bbox(false), bounds);
    crate::editor::filter_edits::set_mode(&mut state, FilterMode::Pixelate, &mut 0);
    assert!(!super::enabled(&state.annotations[0]));
    state.undo(&mut 0);
    assert!(super::enabled(&state.annotations[0]));
    state.redo(&mut 0);
    assert!(!super::enabled(&state.annotations[0]));
}

/// 【聚焦回归】【真实拖动】无参数，移动与缩放只更新孔洞附近，反复局部帧必须逐像素等于完整帧
#[test]
fn focus_drag_redraws_only_the_hole_and_matches_whole_frames() {
    use crate::{
        tools::Tool,
        types::{MouseButton, SelectionHandle},
    };
    for ellipse in [false, true] {
        for resize in [false, true] {
            for rotation in [0.0, 37.0] {
                let mut state = scene(ellipse, rotation, true);
                state.ui_hidden = true;
                state.selected_tool = Tool::Pick;
                state.select_annotation_ids([1]);
                let AnnotationShape::Redact { filter } = state.annotations[0].shape else {
                    panic!("filter expected")
                };
                let point = if resize {
                    crate::features::filter::drag::controls(filter)
                        .into_iter()
                        .find(|(handle, _)| *handle == SelectionHandle::Right)
                        .unwrap()
                        .1
                } else {
                    filter.center()
                };
                support::paint(&mut state, None, false);
                support::finish_frame(&mut state);
                support::pointer(
                    &mut state,
                    (f64::from(point.0), f64::from(point.1)),
                    Some((MouseButton::Left, true)),
                    &mut 0,
                );
                assert!(state.ann_drag.is_some());
                support::finish_frame(&mut state);
                for index in 1..=4 {
                    support::pointer(
                        &mut state,
                        (
                            f64::from(point.0) + f64::from(index) * 3.0,
                            f64::from(point.1) + f64::from(index),
                        ),
                        None,
                        &mut 0,
                    );
                    let dirty = state.monitor_dirty_rect(0).unwrap();
                    assert!(
                        dirty.width() * dirty.height() < 320.0 * 240.0 * 0.60,
                        "unexpected full focus redraw: {dirty:?}"
                    );
                    let actual = support::render(&mut state, Some(dirty), false);
                    let expected = support::render(&mut state, None, false);
                    support::equal(&actual, &expected, "focus drag partial frame");
                    support::finish_frame(&mut state);
                }
                state.refresh_filters();
                assert_eq!(state.filters.retained_bytes(), 0);
                assert!(!state.region_rendering.cached_annotations);
            }
        }
    }
}

/// 【聚焦回归】【无彩色描边】无参数，红色和粗描边参数不改变聚焦像素，删除后外围恢复，撤销重新压暗
#[test]
fn focus_ignores_outline_color_and_delete_restores_the_surroundings() {
    use tiny_skia::Color;
    let mut state = scene(false, 0.0, true);
    let reference = crate::app::render_final(&mut state).unwrap().0;
    let alpha = state.annotations[0].color.alpha();
    state.annotations[0].color = Color::from_rgba(1.0, 0.0, 0.0, alpha).unwrap();
    state.annotations[0].stroke_width = 80.0;
    assert_eq!(crate::app::render_final(&mut state).unwrap().0, reference);
    state.selected_tool = crate::tools::Tool::Pick;
    state.select_annotation_ids([1]);
    crate::tools::pick::delete_selected(&mut state, &mut 0);
    let cleared = Pixmap::decode_png(&crate::app::render_final(&mut state).unwrap().0).unwrap();
    assert_eq!(
        cleared.data(),
        state.captures[0].pixmap.pixels_view().data()
    );
    state.undo(&mut 0);
    assert_eq!(crate::app::render_final(&mut state).unwrap().0, reference);
}

/// 【聚焦回归】【多聚焦不叠加压暗】验证多个分离及交叠聚焦框均保留孔洞原亮，且外围暗度等同单个聚焦框
#[test]
fn multiple_focus_regions_preserve_all_holes_without_stacking_dimming() {
    use tiny_skia::PremultipliedColorU8;

    let mut single_ref = scene(false, 0.0, true);
    single_ref.ui_hidden = true;
    let single_export =
        Pixmap::decode_png(&crate::app::render_final(&mut single_ref).unwrap().0).unwrap();
    let single_preview = support::render(&mut single_ref, None, false);
    let single_corner = single_export.pixel(15, 15).unwrap();
    let single_overlay_corner = single_preview.pixel(15, 15).unwrap();

    let build_multi = |region_mode: bool| {
        let mut state = support::editor_size((320, 240), 1.0, region_mode);
        state.ui_hidden = true;
        let bounds = Rect::from_xywh(0.0, 0.0, 320.0, 240.0).unwrap();
        state.selection.zone = Some(bounds);

        let mut f1 =
            RegionFilter::new((40.0, 50.0), (140.0, 170.0), 14.0, FilterMode::Focus).unwrap();
        f1.radius = 8.0;
        let mut a1 = support::annotation(1, 0.0, AnnotationShape::Redact { filter: f1 });
        a1.frozen_bounds = Some(bounds);

        let f2 = RegionFilter::new(
            (120.0, 70.0),
            (260.0, 190.0),
            14.0,
            FilterMode::FocusEllipse,
        )
        .unwrap();
        let mut a2 = support::annotation(2, 0.0, AnnotationShape::Redact { filter: f2 });
        a2.frozen_bounds = Some(bounds);

        state.annotations.push(a1);
        state.annotations.push(a2);
        state.annotations_dirty = true;
        state
    };

    let mut region_state = build_multi(true);
    let mut cached_state = build_multi(false);

    for state in [&mut region_state, &mut cached_state] {
        let exported = Pixmap::decode_png(&crate::app::render_final(state).unwrap().0).unwrap();
        let source = state.captures[0].pixmap.pixels_view();

        // 1. 两个聚焦孔洞中心及交叠区均保持原始像素，不被另一个聚焦遮罩压暗
        assert_eq!(exported.pixel(80, 110), source.pixel(80, 110));
        assert_eq!(exported.pixel(190, 130), source.pixel(190, 130));
        assert_eq!(exported.pixel(130, 120), source.pixel(130, 120));

        // 2. 孔洞外侧像素与单个聚焦标注的亮度完全一致，绝不二次叠加变黑
        assert_eq!(exported.pixel(15, 15).unwrap(), single_corner);
        assert_eq!(
            exported.pixel(305, 225).unwrap(),
            single_export.pixel(305, 225).unwrap()
        );
    }

    // 3. 预览前景层孔洞保持完全透明，外围遮罩透明度与单聚焦一致且双渲染模式像素等价
    let region_preview = support::render(&mut region_state, None, false);
    let cached_preview = support::render(&mut cached_state, None, false);
    support::equal(&region_preview, &cached_preview, "multi-focus preview");
    assert_eq!(
        region_preview.pixel(80, 110),
        Some(PremultipliedColorU8::TRANSPARENT)
    );
    assert_eq!(
        region_preview.pixel(190, 130),
        Some(PremultipliedColorU8::TRANSPARENT)
    );
    assert_eq!(
        region_preview.pixel(130, 120),
        Some(PremultipliedColorU8::TRANSPARENT)
    );
    assert_eq!(region_preview.pixel(15, 15).unwrap(), single_overlay_corner);
}
