//! 【图片窗口测试】【尺寸隔离】调整窗口不能改变原图、标注几何和导出内容

use super::*;

/// 【图片窗口测试】【连续调整】窗口往返调整同时保留原图透明度、选区、历史与原生分辨率导出，无参数、无返回值
#[test]
fn resize_preserves_source_annotations_and_export() {
    let mut state = super::super::region_rendering_test_support::editor(1.25, true);
    let mut original = tiny_skia::Pixmap::new(320, 180).unwrap();
    original.fill(tiny_skia::Color::from_rgba8(120, 50, 90, 130));
    let file = FileCanvas::new(original, Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap()).unwrap();
    state.selection.zone = Some(file.viewport);
    state.file_canvas = Some(file);
    state
        .annotations
        .push(super::super::region_rendering_test_support::annotation(
            41,
            0.0,
            crate::types::AnnotationShape::Rectangle {
                radius: 0.0,
                fixed_ratio: false,
                start: (80.0, 120.0),
                end: (150.0, 160.0),
                filled: true,
            },
        ));
    let selection = state.selection.zone;
    let annotations = state.annotations.clone();
    let original = state.file_canvas.as_ref().unwrap().original.data().to_vec();
    let exported = super::super::image_export::render_precise(&mut state)
        .unwrap()
        .0;
    for size in [(900, 600), (480, 320), (640, 480)] {
        resize(&mut state, size, &mut 0);
        super::super::region_rendering_test_support::paint(&mut state, None, false);
        assert_eq!(state.placements[0].size, (size.0 as i32, size.1 as i32));
        assert_eq!(state.canvas[0].width(), (size.0 as f32 * 1.25) as u32);
        assert_eq!(state.selection.zone, selection);
        assert_eq!(state.annotations, annotations);
        assert!(state.undo_stack.is_empty());
        assert_eq!(
            state.file_canvas.as_ref().unwrap().original.data(),
            original
        );
        assert_eq!(
            super::super::image_export::render_precise(&mut state)
                .unwrap()
                .0,
            exported
        );
    }
}
