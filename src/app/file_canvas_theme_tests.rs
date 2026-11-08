//! 【显示主题回归】【像素隔离】深浅往返只改变显示底色，不改变标注、原件、透明度和导出

use super::*;
use crate::app::{image_export, region_rendering_test_support as support};
use tiny_skia::{Color, Rect};

/// 【显示主题回归】【透明文件】返回带完全透明和半透明像素的文件编辑器，不修改全局主题
fn editor() -> EditorState {
    let mut state = support::editor(1.25, true);
    let mut original = Pixmap::new(160, 80).unwrap();
    original.fill(Color::from_rgba8(90, 130, 170, 117));
    original.pixels_mut()[0] = tiny_skia::PremultipliedColorU8::TRANSPARENT;
    let file = FileCanvas::new(original, Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap()).unwrap();
    state.selection.zone = Some(file.viewport);
    state.file_canvas = Some(file);
    state
}

/// 【显示主题回归】【往返导出】分别验证空白透明原件和已有标注，深浅往返恢复显示字节，PNG及原件始终不变
#[test]
fn light_dark_round_trip_preserves_original_alpha_annotations_and_export() {
    for annotated in [false, true] {
        let mut state = editor();
        if annotated {
            state.annotations.push(support::annotation(
                91,
                0.0,
                crate::types::AnnotationShape::Rectangle {
                    start: (250.0, 180.0),
                    end: (350.0, 240.0),
                    filled: true,
                },
            ));
        }
        state.prepare_region_layers();
        let original = state.file_canvas.as_ref().unwrap().original.data().to_vec();
        let exported = image_export::render_precise(&mut state).unwrap().0;
        if !annotated {
            assert_eq!(Pixmap::decode_png(&exported).unwrap().data(), original);
        }
        let selection = state.selection.zone;
        let annotation_layer = state.annotations_layer[0].data().to_vec();
        let mut first = None;
        for mode in [Mode::Dark, Mode::Light, Mode::Dark] {
            refresh(&mut state, mode);
            assert!(state.selection_translation.background_dirty);
            let pixels = state.captures[0].pixmap.data();
            let color = crate::theme::canvas::background(mode);
            assert_eq!(&pixels[..4], &[color.0, color.1, color.2, 255]);
            match (&first, mode) {
                (None, _) => first = Some(pixels.to_vec()),
                (Some(first), Mode::Light) => assert_eq!(pixels, first),
                (Some(first), Mode::Dark) => assert_eq!(pixels, first),
            }
            assert_eq!(
                state.file_canvas.as_ref().unwrap().original.data(),
                original
            );
            assert_eq!(
                image_export::render_precise(&mut state).unwrap().0,
                exported
            );
            assert_eq!(state.selection.zone, selection);
            assert_eq!(state.annotations_layer[0].data(), annotation_layer);
            assert_eq!(state.annotations.len(), usize::from(annotated));
            if annotated {
                assert_eq!(state.annotations[0].id, 91);
                assert_eq!(
                    state.annotations[0].color,
                    Color::from_rgba8(210, 90, 40, 137)
                );
            }
            // 恒等区域模式不得为了主题切换分配整屏遮罩。
            assert_eq!((state.dimmed[0].width(), state.dimmed[0].height()), (1, 1));
        }
    }
}

/// 【显示主题回归】【导航缓存】缩放视图刷新后下一帧恢复完整背景；截图捕获和导航位置不变，恒等复位仍能识别旧键
#[test]
fn refresh_invalidates_navigation_without_mutating_screenshot_or_reset_transition() {
    let mut state = support::editor(1.0, true);
    state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    assert!(state.ensure_navigation());
    state
        .navigation
        .view
        .as_mut()
        .unwrap()
        .zoom_at(0.8, (320.0, 240.0), true);
    support::paint_current(&mut state, None, false);
    let expected = state.dimmed[0].data().to_vec();
    let capture = state.captures[0].pixmap.share();
    let view = state.canvas_transform();
    state.dimmed[0].fill(Color::from_rgba8(255, 0, 255, 255));
    crate::app::theme_switch::refresh(&mut state, &mut 0);
    assert!(state.navigation.frames.iter().all(Option::is_none));
    assert_eq!(state.canvas_transform(), view);
    assert!(std::sync::Arc::ptr_eq(
        &capture,
        &state.captures[0].pixmap.share()
    ));
    assert!(!state.selection_translation.background_dirty);
    support::paint_current(&mut state, None, false);
    assert_eq!(state.dimmed[0].data(), expected);
    let previous = state.navigation.frames[0];
    state.navigation.view = None;
    refresh(&mut state, Mode::current());
    assert!(state.navigation.frames[0] == previous);
}

/// 【显示主题回归】【翻译退出】跨主题恢复原件后预览片段使用当前底色，不能从保存的旧预览恢复深色补丁
#[test]
fn translation_restore_rebuilds_preview_using_current_theme() {
    let mut state = editor();
    let original = state.file_canvas.as_ref().unwrap().original.data().to_vec();
    refresh(&mut state, Mode::current().opposite());
    let region = crate::features::selection_translation::pixels::Region::capture(
        &state,
        state.selection.zone.unwrap(),
    )
    .unwrap();
    refresh(&mut state, Mode::current());
    let expected = state.captures[0].pixmap.data().to_vec();
    let mut translated = region.original.clone();
    translated.fill(Color::from_rgba8(200, 40, 60, 99));
    region.apply(
        &mut state.captures,
        state.file_canvas.as_mut(),
        Some(&translated),
    );
    region.apply(&mut state.captures, state.file_canvas.as_mut(), None);
    assert_eq!(
        state.file_canvas.as_ref().unwrap().original.data(),
        original
    );
    assert_eq!(state.captures[0].pixmap.data(), expected);
}
