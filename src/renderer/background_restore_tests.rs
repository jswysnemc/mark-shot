//! 【区域背景测试】【逐像素对照】遮罩、圆角和导航背景与旧缓存入口保持一致

use crate::app::region_rendering_test_support::{editor, equal, render};
use tiny_skia::Rect;

/// 【区域背景测试】【选择损坏】无参数，比较多种设备比例的选区变化及反复局部恢复，无返回值
#[test]
fn identity_background_matches_cached_selection_changes() {
    for scale in [0.75, 1.0, 1.2, 1.5, 2.0] {
        let mut cached = editor(scale, false);
        let mut region = editor(scale, true);
        for selection in [
            None,
            Rect::from_xywh(19.375, 43.625, 559.25, 373.875),
            Rect::from_xywh(-10.375, 43.625, 450.25, 500.875),
            None,
        ] {
            cached.selection.zone = selection;
            region.selection.zone = selection;
            equal(
                &render(&mut region, None, false),
                &render(&mut cached, None, false),
                &format!("identity scale={scale} selection={selection:?}"),
            );
            for dirty in [
                Rect::from_xywh(0.0, 0.0, 20.875, 479.0),
                Rect::from_xywh(19.125, 43.125, 460.25, 350.375),
            ] {
                equal(
                    &render(&mut region, dirty, false),
                    &render(&mut cached, dirty, false),
                    &format!("identity partial scale={scale}"),
                );
            }
        }
    }
}

/// 【区域背景测试】【原图圆角】无参数，通过图片入口状态比较分数选区和圆角局部损坏，无返回值
#[test]
fn image_selection_corners_match_cached_pixels() {
    for scale in [1.0, 1.2, 1.5, 2.0] {
        let mut cached = editor(scale, false);
        let mut region = editor(scale, true);
        for state in [&mut cached, &mut region] {
            let bounds = Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
            state.file_canvas = crate::editor::file_canvas::FileCanvas::new(
                state.captures[0].pixmap.clone().into_pixmap(),
                bounds,
            );
            state.selection.zone = Rect::from_xywh(19.375, 43.625, 559.25, 373.875);
        }
        equal(
            &render(&mut region, None, false),
            &render(&mut cached, None, false),
            &format!("corners scale={scale}"),
        );
        let whole = region.canvas[0].clone();
        for (x, y) in [(18.625, 43.375), (565.5, 401.375)] {
            for _ in 0..3 {
                render(&mut region, Rect::from_xywh(x, y, 15.75, 15.25), false);
                equal(
                    &region.canvas[0],
                    &whole,
                    &format!("corners partial scale={scale}"),
                );
            }
        }
    }
}

/// 【区域背景测试】【导航背景】无参数，比较放大、缩小和移动后固定纹理的每个像素，局部恢复不能改变采样，无返回值
#[test]
fn navigation_background_matches_full_cached_sampling() {
    for scale in [1.0, 1.2, 1.5, 2.0] {
        let mut cached = editor(scale, false);
        let mut region = editor(scale, true);
        for state in [&mut cached, &mut region] {
            state.selection.zone = Rect::from_xywh(19.375, 43.625, 559.25, 373.875);
            assert!(state.ensure_navigation());
        }
        for zoom in [1.2, 0.75, 2.1] {
            for state in [&mut cached, &mut region] {
                let view = state.navigation.view.as_mut().unwrap();
                view.zoom_at(zoom, (277.25, 195.75), true);
            }
            equal(
                &render(&mut region, None, false),
                &render(&mut cached, None, false),
                &format!("navigation scale={scale} zoom={zoom}"),
            );
            let whole = region.canvas[0].clone();
            render(
                &mut region,
                Rect::from_xywh(340.375, 180.625, 180.875, 199.375),
                false,
            );
            equal(
                &region.canvas[0],
                &whole,
                &format!("navigation partial scale={scale}"),
            );
        }
    }
}
