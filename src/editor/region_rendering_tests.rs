//! 【区域绘制测试】【状态恢复】验证缓存按需创建、画笔撤销、文字原件排除及密集标注的实际帧耗时

use crate::{
    app::region_rendering_test_support::{
        annotation, editor, editor_size, equal, key, paint, paint_current, pointer, render,
    },
    types::{AnnotationShape, MouseButton},
};
use tiny_skia::{Color, Rect};

/// 【区域绘制测试】【真实画笔】无参数，对照增量草稿、提交、撤销和重做，移除画笔后释放完整缓存，无返回值
#[test]
fn real_pen_drafts_and_history_preserve_pixels_across_cache_transitions() {
    for scale in [1.0, 1.2, 1.5, 2.0] {
        let mut cached = editor(scale, false);
        let mut region = editor(scale, true);
        for state in [&mut cached, &mut region] {
            state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
            state.tool_settings.color = Color::from_rgba8(240, 80, 30, 128);
            paint(state, None, false);
            state.selected_tool = crate::tools::Tool::Pen;
        }
        for (position, button) in [
            ((140.375, 130.625), Some((MouseButton::Left, true))),
            ((240.625, 190.375), None),
            ((380.25, 225.875), None),
            ((480.625, 380.375), None),
            ((540.25, 391.875), None),
            ((540.25, 391.875), Some((MouseButton::Left, false))),
        ] {
            for state in [&mut cached, &mut region] {
                pointer(state, position, button, &mut 0);
                paint_current(state, None, false);
            }
            equal(
                &region.canvas[0],
                &cached.canvas[0],
                &format!("pen scale={scale} position={position:?} button={button:?}"),
            );
        }
        assert!(region.pen_backup.is_empty());
        for command in ["Ctrl+Z", "Ctrl+Y", "Ctrl+Z"] {
            for state in [&mut cached, &mut region] {
                key(state, command, &mut 0);
                paint(state, None, false);
            }
            equal(
                &region.canvas[0],
                &cached.canvas[0],
                &format!("pen history scale={scale} key={command}"),
            );
        }
        assert!(!region.region_rendering.cached_annotations);
        assert_eq!(region.annotations_layer[0].data().len(), 4);
    }
}

/// 【区域绘制测试】【文字原件】无参数，编辑已有旋转文字时只显示预览，正文原件和历史保持完整，无返回值
#[test]
fn existing_text_preview_excludes_the_original_annotation() {
    for scale in [1.0, 1.2, 1.5, 2.0] {
        let mut cached = editor(scale, false);
        let mut region = editor(scale, true);
        for state in [&mut cached, &mut region] {
            state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
            state.annotations.push(annotation(
                99,
                13.0,
                AnnotationShape::Text {
                    frame: Default::default(),
                    appearance: Default::default(),
                    start: (330.375, 180.625),
                    content: "Mark Shot".into(),
                    font_size: 24.0,
                    bold: false,
                    italic: false,
                    outline: None,
                },
            ));
            assert!(crate::editor::text_session::begin_existing(
                state, 99, &mut 0
            ));
        }
        equal(
            &render(&mut region, None, false),
            &render(&mut cached, None, false),
            &format!("text original scale={scale}"),
        );
        assert!(region.region_rendering.cached_annotations);
        assert_eq!(region.annotations.len(), 1);
        assert_eq!(region.text_session.as_ref().unwrap().original_id, Some(99));
    }
}

/// 【区域绘制测试】【密集耗时】无参数，记录八百个固定图形下完整帧和窄损坏的耗时，可通过环境目录保存数据，无返回值
#[test]
fn dense_scene_frame_cost_is_measured_through_the_editor_entry() {
    let mut observations = Vec::new();
    for region in [false, true] {
        let mut state = editor_size((3840, 2160), 1.0, region);
        state.selection.zone = Rect::from_xywh(0.0, 0.0, 3840.0, 2160.0);
        for index in 0..800 {
            let (x, y) = (
                40.375 + (index % 40) as f32 * 93.25,
                40.625 + (index / 40) as f32 * 102.25,
            );
            state.annotations.push(annotation(
                index,
                0.0,
                AnnotationShape::Rectangle {
                    start: (x, y),
                    end: (x + 57.375, y + 53.875),
                    filled: true,
                },
            ));
        }
        let started = std::time::Instant::now();
        paint(&mut state, None, false);
        let whole_ms = started.elapsed().as_secs_f64() * 1000.0;
        let mut costs = Vec::new();
        for _ in 0..20 {
            let started = std::time::Instant::now();
            paint_current(
                &mut state,
                Rect::from_xywh(1775.375, 830.625, 40.875, 180.375),
                false,
            );
            costs.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        costs.sort_by(f64::total_cmp);
        observations.push(serde_json::json!({"region":region,"annotations":800,"whole_ms":whole_ms,"partial_median_ms":costs[10],"partial_max_ms":costs[19]}));
    }
    if let Ok(directory) = std::env::var("MARK_SHOT_REGION_RENDER_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join("frame-cost.json"),
            serde_json::to_string_pretty(&observations).unwrap(),
        )
        .unwrap();
    }
}
