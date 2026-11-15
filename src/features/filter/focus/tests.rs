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
