//! 【标注样式测试】【创建单位】文件适配和截图比例只转换一次，移动不得覆盖标注自身单位

use super::*;
use crate::types::AnnotationShape;

/// 【标注样式测试】【文件创建】小图放大与大图缩小都保存六个原图像素，全部移动更新保留创建比例
#[test]
fn every_persistent_tool_stores_image_width_after_file_fit_and_movement() {
    for size in [(320, 240), (1280, 960)] {
        for tool in TOOLS {
            let mut state = image_editor(size);
            let mut dirty = 0;
            draw(&mut state, tool, 6.0, &mut dirty);
            let annotation = &state.annotations[0];
            let scale = 640.0 / size.0 as f32;
            close(annotation.style_scale, scale, (size, tool));
            close(annotation.stroke_width, 6.0 * scale, (size, tool));
            close(annotation.image_stroke_width(), 6.0, (size, tool));
            assert_eq!(annotation.shadow_color, Color::TRANSPARENT, "{tool:?}");
            match annotation.shape {
                AnnotationShape::Number { mark } => close(mark.style_scale, scale, tool),
                AnnotationShape::Magnifier { magnifier } => {
                    close(magnifier.style_scale, scale, tool)
                }
                AnnotationShape::Ruler { measurement } => {
                    close(measurement.style_scale as f32, scale, tool);
                    close(
                        measurement.unit(annotation.stroke_width),
                        scale * (1.0 + 3.0 / 9.0),
                        tool,
                    );
                }
                AnnotationShape::Arrow { ref line, .. }
                | AnnotationShape::Line { ref line, .. } => {
                    close(line.style_scale, scale, tool);
                }
                _ => {}
            }
        }
    }
}

/// 【标注样式测试】【截图创建】分数输出比例缩小持久笔宽，尺子刻度保留截图补偿但使用原图宽度计算粗细
#[test]
fn fractional_capture_scales_width_but_compensates_ruler_ticks() {
    for tool in TOOLS {
        let mut state = capture_editor(1.5);
        let mut dirty = 0;
        draw(&mut state, tool, 6.0, &mut dirty);
        let annotation = &state.annotations[0];
        close(annotation.style_scale, 2.0 / 3.0, tool);
        close(annotation.stroke_width, 4.0, tool);
        close(annotation.image_stroke_width(), 6.0, tool);
        if let AnnotationShape::Ruler { measurement } = annotation.shape {
            close(measurement.style_scale as f32, 1.0, tool);
            close(
                measurement.unit(annotation.stroke_width),
                1.0 + 3.0 / 9.0,
                tool,
            );
        }
    }
}

/// 【标注样式测试】【分数文件比例】真实文件创建保留视口与原图之比的双精度，不先对比例做单精度除法
#[test]
fn real_file_ruler_creation_preserves_fractional_style_ratio() {
    let mut state = image_editor((641, 479));
    let mut dirty = 0;
    let file = state.file_canvas.as_ref().unwrap();
    let expected = f64::from(file.viewport.width()) / f64::from(file.original.width());
    let rounded = f64::from(file.viewport.width() / file.original.width() as f32);
    assert_ne!(expected, rounded);
    draw(&mut state, Tool::Ruler, 6.0, &mut dirty);
    let AnnotationShape::Ruler { measurement } = state.annotations[0].shape else {
        panic!("ruler expected")
    };
    assert!((measurement.style_scale - expected).abs() < 1e-15);
}

/// 【标注样式测试】【独立语义】滤镜强度和文字点字号保持原图数值，创建不能二次缩小字号或强度
#[test]
fn filter_strength_and_text_points_keep_their_existing_image_semantics() {
    let mut state = image_editor((1280, 960));
    let mut dirty = 0;
    state.tool_settings.strength = 16.0;
    draw(&mut state, Tool::Redact, 0.0, &mut dirty);
    let annotation = &state.annotations[0];
    close(annotation.style_scale, 0.5, "filter annotation");
    let AnnotationShape::Redact { filter } = annotation.shape else {
        panic!("filter expected")
    };
    close(filter.strength, 16.0, "filter strength");
    close(filter.style_scale, 0.5, "filter scale");
    select_tool(&mut state, Tool::Text, &mut dirty);
    state.tool_settings.font_size = 30.0;
    pointer(&mut state, 180.0, 120.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    crate::app::input::handle_key(&mut state, None, Some("原图字号".into()), &mut dirty);
    crate::editor::text_session::commit(&mut state, &mut dirty);
    let annotation = &state.annotations[1];
    let AnnotationShape::Text {
        font_size,
        ref appearance,
        ..
    } = annotation.shape
    else {
        panic!("text expected")
    };
    close(font_size, 30.0, "text points");
    close(appearance.size_scale, 0.5, "text scale");
    close(annotation.style_scale, 0.5, "text annotation");
}

/// 【标注样式测试】【激光采样】小图放大后低于原图抖动阈值的移动不追加采样，大图缩小后的同一窗口移动达到阈值
#[test]
fn laser_width_and_sampling_distance_use_image_units_without_persistent_annotations() {
    for (size, expected) in [((320, 240), 1), ((1280, 960), 2)] {
        let mut state = image_editor(size);
        let mut dirty = 0;
        select_tool(&mut state, Tool::Laser, &mut dirty);
        state.tool_settings.set_stroke(StrokeGroup::Laser, 1.0);
        pointer(&mut state, 220.0, 160.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 220.5, 160.0, &mut dirty);
        assert_eq!(state.laser.point_count(), expected, "size={size:?}");
        close(
            state.laser.strokes().next().unwrap().image_stroke_width(),
            1.0,
            size,
        );
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
    }
}
