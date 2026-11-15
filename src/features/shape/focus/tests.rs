//! 【聚焦回归】【像素与编辑】覆盖旋转、区域渲染、导出以及撤销

use crate::image_source::PixelView;
use crate::{app::region_rendering_test_support as support, types::AnnotationShape};
use tiny_skia::{Pixmap, Rect};

/// 【聚焦回归】【场景创建】按 ellipse、rotation 和 region 创建完整选区和聚焦标注，返回编辑器
fn scene(ellipse: bool, rotation: f64, region: bool) -> crate::editor::EditorState {
    let mut state = support::editor_size((320, 240), 1.0, region);
    let bounds = Rect::from_xywh(0.0, 0.0, 320.0, 240.0).unwrap();
    state.selection.zone = Some(bounds);
    let shape = if ellipse {
        AnnotationShape::Circle {
            start: (100.0, 70.0),
            end: (220.0, 170.0),
            filled: false,
            fixed_ratio: false,
            focus: true,
        }
    } else {
        AnnotationShape::Rectangle {
            start: (100.0, 70.0),
            end: (220.0, 170.0),
            filled: false,
            radius: 12.0,
            fixed_ratio: false,
            focus: true,
        }
    };
    let mut annotation = support::annotation(1, rotation, shape);
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
    crate::editor::edits::commit_settings_change(
        &mut state,
        true,
        true,
        |tools| tools.focus = false,
        |annotation| {
            if let AnnotationShape::Rectangle { focus, .. } = &mut annotation.shape {
                *focus = false;
            }
        },
        &mut 0,
    );
    assert!(!super::enabled(&state.annotations[0]));
    state.undo(&mut 0);
    assert!(super::enabled(&state.annotations[0]));
    state.redo(&mut 0);
    assert!(!super::enabled(&state.annotations[0]));
}
