//! 【标注样式测试】【导航独立】持续缩放只改变窗口投影，之后创建和属性编辑仍使用原图单位

use super::*;
use crate::{app::input::handle_axis_scroll, types::ScrollEvent};

/// 【标注样式测试】【真实帧】按state与dirty区域绘制生产帧，临时隐藏工具面板，返回无值
pub(super) fn render(state: &mut EditorState, dirty: Option<Rect>) {
    state.toolbar.opacity = 0.0;
    state.action_toolbar.opacity = 0.0;
    state.settings_panel.visible = false;
    state.selection_size.panel = None;
    let frame = crate::app::frame_render::Frame {
        active_text_id: None,
        scan_badge: None,
        current_color: Color::BLACK,
        outline_color: Color::TRANSPARENT,
        annotations_hidden: false,
    };
    let mut previous = state.selection.zone;
    crate::app::frame_render::render_monitor(state, 0, &frame, dirty, &mut previous);
}

/// 【标注样式测试】【导航创建】滚轮放大后创建的普通和专用工具保留初始文件单位，不保存导航倍率
#[test]
fn tools_created_after_zoom_keep_image_width_without_storing_navigation() {
    for tool in TOOLS {
        let mut state = image_editor((1280, 960));
        let mut dirty = 0;
        pointer(&mut state, 320.0, 240.0, &mut dirty);
        handle_axis_scroll(&mut state, ScrollEvent::wheel(720), &mut dirty);
        let view = state.canvas_transform();
        assert!(!view.is_identity());
        draw(&mut state, tool, 6.0, &mut dirty);
        let annotation = &state.annotations[0];
        close(annotation.style_scale, 0.5, tool);
        close(annotation.stroke_width, 3.0, tool);
        close(annotation.image_stroke_width(), 6.0, tool);
        let start = annotation.shape.start_point();
        let expected = view.to_canvas((220.0, 160.0));
        close(start.0, expected.0 as f32, tool);
        close(start.1, expected.1 as f32, tool);
    }
}

/// 【标注样式测试】【窗口细线】缩小和放大时普通线条遵循一点五窗口像素下限，渲染不再按原图最大笔宽夹紧
#[test]
fn navigated_line_rendering_uses_window_minimum_without_reclamping_image_width() {
    for (device, wheel, width) in [(1.0, -1440, 1.0), (1.5, -1440, 1.0), (2.0, 720, 72.0)] {
        let mut state = image_editor((1280, 960));
        state.placements[0].scale = device;
        state.canvas[0] = Pixmap::new((640.0 * device) as u32, (480.0 * device) as u32).unwrap();
        state.dimmed[0] = state.canvas[0].clone();
        state.annotations_layer[0] = state.canvas[0].clone();
        let mut dirty = 0;
        pointer(&mut state, 320.0, 240.0, &mut dirty);
        handle_axis_scroll(&mut state, ScrollEvent::wheel(wheel), &mut dirty);
        select_tool(&mut state, Tool::Line, &mut dirty);
        state.tool_settings.color = Color::BLACK;
        state.tool_settings.set_stroke(StrokeGroup::Line, width);
        pointer(&mut state, 270.0, 240.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 370.0, 240.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        crate::app::canvas_navigation::preview::clear(&mut state, &mut dirty);
        render(&mut state, None);
        let column = (320.0 * device) as u32;
        let center = (240.0 * device) as u32;
        let expected = (state.annotations[0].stroke_width * state.canvas_transform().scale as f32)
            .max(1.5)
            * device;
        let half = (expected / 2.0 + 8.0 * device).ceil() as u32;
        let darkness: f32 = (center - half..center + half)
            .map(|y| 1.0 - f32::from(state.canvas[0].pixel(column, y).unwrap().red()) / 255.0)
            .sum();
        assert!(
            (darkness - expected).abs() < 0.2,
            "device={device} wheel={wheel} actual={darkness} expected={expected}"
        );
        close(
            state.annotations[0].image_stroke_width(),
            width,
            "unchanged stored width",
        );
    }
}

/// 【标注样式测试】【最小圆框剔除】缩小后的序号仍有十三窗口像素圆框，只覆盖原图选框之外的窄条重绘不得丢失圆框
#[test]
fn number_minimum_radius_remains_visible_in_damage_outside_its_stored_bounds() {
    let mut state = image_editor((1280, 960));
    let mut dirty = 0;
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(-1440), &mut dirty);
    select_tool(&mut state, Tool::Number, &mut dirty);
    state.tool_settings.color = Color::from_rgba8(239, 68, 68, 255);
    state.tool_settings.set_stroke(StrokeGroup::Number, 1.0);
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    crate::app::canvas_navigation::preview::clear(&mut state, &mut dirty);
    render(&mut state, None);
    let whole = state.canvas[0].clone();
    assert!(whole.pixel(310, 240).unwrap().green() < 150);
    for x in [307.25, 309.5, 329.25, 331.5] {
        for _ in 0..3 {
            render(&mut state, Rect::from_xywh(x, 220.25, 1.625, 39.5));
        }
    }
    assert_eq!(whole.data(), state.canvas[0].data());
}

/// 【标注样式测试】【乘法正文】荧光笔保留原图黑字，草稿和提交后的局部绘制相同，恒等与导航视图都使用实际底图
#[test]
fn highlighter_multiplies_actual_background_in_drafts_commits_and_partial_redraws() {
    for wheel in [0, 720] {
        let mut state = image_editor((640, 480));
        let mut original = state
            .file_canvas
            .as_ref()
            .unwrap()
            .original
            .as_ref()
            .to_owned();
        for y in 180..280 {
            for x in 300..340 {
                let index = ((y * 640 + x) * 4) as usize;
                original.data_mut()[index..index + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
        state.canvas[0] = original.clone();
        state.dimmed[0] = original.clone();
        state.file_canvas.as_mut().unwrap().original =
            crate::image_source::ImageSource::memory(original);
        let mut dirty = 0;
        pointer(&mut state, 320.0, 240.0, &mut dirty);
        if wheel != 0 {
            handle_axis_scroll(&mut state, ScrollEvent::wheel(wheel), &mut dirty);
        }
        select_tool(&mut state, Tool::Highlighter, &mut dirty);
        state.tool_settings.color = Color::from_rgba8(250, 200, 20, 180);
        state
            .tool_settings
            .set_stroke(StrokeGroup::Highlighter, 1.0);
        pointer(&mut state, 240.0, 240.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        for x in [270.0, 300.0, 330.0, 360.0, 400.0] {
            pointer(&mut state, x, 240.0, &mut dirty);
        }
        crate::app::canvas_navigation::preview::clear(&mut state, &mut dirty);
        render(&mut state, None);
        let draft = state.canvas[0].clone();
        assert_eq!(draft.pixel(320, 240).unwrap().red(), 0, "wheel={wheel}");
        assert!(draft.pixel(260, 240).unwrap().blue() < 200);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        render(&mut state, None);
        assert_eq!(draft.data(), state.canvas[0].data(), "commit wheel={wheel}");
        for x in [257.25, 307.5, 329.75] {
            render(&mut state, Rect::from_xywh(x, 230.5, 4.625, 19.25));
        }
        assert_eq!(
            draft.data(),
            state.canvas[0].data(),
            "partial wheel={wheel}"
        );
        let png = crate::app::image_export::render_precise(&mut state)
            .unwrap()
            .0;
        let exported = Pixmap::decode_png(&png).unwrap();
        assert_eq!(exported.pixel(320, 240).unwrap().red(), 0);
    }
}

/// 【标注样式测试】【专用命中】原图放大比例不扩大序号、放大镜和滤镜的选中控制点容差，绘制尺寸仍为九个窗口像素
#[test]
fn specialized_selected_controls_keep_window_sizes_and_hit_tolerances_after_file_fit() {
    use crate::types::{AnnotationShape, SelectionHandle};
    for tool in [Tool::Number, Tool::Magnifier, Tool::Redact] {
        let mut state = image_editor((320, 240));
        let mut dirty = 0;
        draw(&mut state, tool, 6.0, &mut dirty);
        let annotation = &state.annotations[0];
        let controls = match annotation.shape {
            AnnotationShape::Number { mark } => {
                crate::features::number::drag::controls(mark, annotation.stroke_width)
            }
            AnnotationShape::Magnifier { magnifier } => {
                crate::features::magnifier::drag::controls(magnifier)
            }
            AnnotationShape::Redact { filter } => crate::features::filter::drag::controls(filter),
            _ => unreachable!(),
        };
        let (handle, at) = controls[0];
        assert_eq!(
            state.canvas_handle(annotation, (f64::from(at.0), f64::from(at.1))),
            handle,
            "{tool:?}"
        );
        assert_ne!(
            state.canvas_handle(annotation, (f64::from(at.0 - 12.0), f64::from(at.1 - 12.0))),
            handle,
            "{tool:?}"
        );
        let mut pixels = Pixmap::new(640, 480).unwrap();
        crate::renderer::draw_annotation(
            &mut crate::renderer::Painter::new(&mut pixels),
            annotation,
            true,
            &mut std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
            None,
        );
        let left = (at.0 - 10.0).floor().max(0.0) as u32;
        let right = (at.0 + 10.0).ceil().min(640.0) as u32;
        let y = (at.1 + 2.0).floor().clamp(0.0, 479.0) as u32;
        let painted = (left..right)
            .filter(|x| {
                let pixel = pixels.pixel(*x, y).unwrap();
                pixel.alpha() > 220
                    && pixel.red() > 200
                    && pixel.green() > 80
                    && pixel.green() < 200
                    && pixel.blue() < 100
            })
            .count();
        assert!(
            (8..=10).contains(&painted),
            "tool={tool:?} handle={handle:?} painted={painted}"
        );
        assert_ne!(handle, SelectionHandle::None);
    }
}
