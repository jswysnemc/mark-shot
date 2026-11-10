//! 【取色回归】【标注生命周期】真实输入创建颜色标注，检查撤销、导出映射与颜色稳定性

use super::{editor, pointer};
use crate::{
    app::input::{handle_pointer_button, select_tool},
    tools::Tool,
    types::{AnnotationShape, MouseButton},
};

/// 【取色回归】【格式面板】无参数，真实单击打开八种格式，面板内移动不再取色，返回后继续采样且保留标注
#[test]
fn inspector_freezes_the_sample_and_exposes_every_format() {
    use crate::features::color_sample::{formats::Format, inspector::Hit, inspector_draw};
    let mut state = editor();
    select_tool(&mut state, Tool::Eyedropper, &mut 0);
    pointer(&mut state, 180.0, 170.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    let (monitor, panel) = state
        .color_inspector
        .panel
        .expect("single sample must open formats");
    let sampled = state.color_inspector.live;
    for (index, format) in Format::ALL.into_iter().enumerate() {
        let button = inspector_draw::copy_rect(panel, index);
        let point = (
            f64::from(button.left() + 2.0),
            f64::from(button.top() + 2.0),
        );
        pointer(&mut state, point.0, point.1, &mut 0);
        assert_eq!(
            state.color_inspector.hit(monitor, point),
            Some(Hit::Copy(format))
        );
        assert_eq!(state.color_inspector.live, sampled);
    }
    super::key(&mut state, "Escape", &mut 0);
    assert!(state.color_inspector.panel.is_none());
    assert_eq!(state.annotations.len(), 1);
    assert!(!state.cancel);
}

/// 【取色回归】【悬停更新】无参数，真实指针移动必须更新颜色值，不能污染绘图颜色或创建标注
#[test]
fn hover_updates_the_sample_without_changing_drawing_color() {
    let mut state = editor();
    let original = state.tool_settings.color;
    let mut image = tiny_skia::Pixmap::new(640, 480).unwrap();
    image.fill(tiny_skia::Color::from_rgba8(32, 67, 92, 255));
    image.data_mut()[(200 * 640 + 200) * 4..(200 * 640 + 201) * 4]
        .copy_from_slice(&[120, 180, 40, 255]);
    state.captures[0].pixmap = image.into();
    select_tool(&mut state, Tool::Eyedropper, &mut 0);
    pointer(&mut state, 180.0, 170.0, &mut 0);
    assert_eq!(
        crate::app::panels::settings::current_color(&state),
        tiny_skia::Color::from_rgba8(32, 67, 92, 255)
    );
    pointer(&mut state, 200.0, 200.0, &mut 0);
    assert_eq!(
        crate::app::panels::settings::current_color(&state),
        tiny_skia::Color::from_rgba8(120, 180, 40, 255)
    );
    assert_eq!(state.tool_settings.color, original);
    assert!(state.annotations.is_empty() && state.pending.is_none());
}

/// 【取色回归】【点位标注】无参数，点击后产生带标签的标注，取色颜色不受后续指针影响，撤销可恢复
#[test]
fn sample_is_an_undoable_annotation_with_stable_color() {
    let mut state = editor();
    select_tool(&mut state, Tool::Eyedropper, &mut 0);
    pointer(&mut state, 180.0, 170.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 300.0, 240.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    assert_eq!(state.annotations.len(), 1);
    let annotation = &state.annotations[0];
    let AnnotationShape::ColorSample { sample } = annotation.shape else {
        panic!("missing color sample")
    };
    assert_eq!(sample.source, (180.0, 170.0));
    assert_eq!(sample.label, (300.0, 240.0));
    assert_eq!(sample.color, tiny_skia::Color::from_rgba8(30, 34, 38, 255));
    let AnnotationShape::ColorSample { sample: scaled } = annotation.scaled(2.0).shape else {
        unreachable!()
    };
    assert_eq!(scaled.source, (360.0, 340.0));
    assert_eq!(scaled.color, sample.color);
    let png = crate::app::render_final(&mut state).unwrap().0;
    let output = image::load_from_memory(&png).unwrap().into_rgba8();
    assert!(
        output.pixels().any(|pixel| pixel.0 == [235, 239, 245, 255]),
        "export omitted the color marker and label"
    );
    crate::app::input::handle_axis_scroll(
        &mut state,
        crate::types::ScrollEvent::wheel(120),
        &mut 0,
    );
    let AnnotationShape::ColorSample { sample: larger } = state.annotations[0].shape else {
        unreachable!()
    };
    assert!(larger.size > sample.size);
    assert_eq!(larger.color, sample.color);
    state.undo(&mut 0);
    state.undo(&mut 0);
    assert!(state.annotations.is_empty());
    state.redo(&mut 0);
    assert_eq!(state.annotations.len(), 1);
}

/// 【取色回归】【移动采样点】无参数，移动颜色标注后实时读取新点原图，撤销恢复旧点及旧颜色
#[test]
fn moving_sample_refreshes_label_and_property_color_and_undo_restores_both() {
    let mut state = editor();
    let mut image = tiny_skia::Pixmap::new(640, 480).unwrap();
    image.fill(tiny_skia::Color::from_rgba8(32, 67, 92, 255));
    for y in 0..480 {
        for x in 240..640 {
            image.data_mut()[(y * 640 + x) * 4..(y * 640 + x + 1) * 4]
                .copy_from_slice(&[120, 180, 40, 255]);
        }
    }
    state.captures[0].pixmap = image.into();
    select_tool(&mut state, Tool::Eyedropper, &mut 0);
    pointer(&mut state, 180.0, 170.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 200.0, 200.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    select_tool(&mut state, Tool::Pick, &mut 0);
    pointer(&mut state, 250.0, 215.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 330.0, 215.0, &mut 0);
    let expected = tiny_skia::Color::from_rgba8(120, 180, 40, 255);
    assert_eq!(
        crate::app::panels::settings::current_color(&state),
        expected
    );
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    let AnnotationShape::ColorSample { sample } = state.annotations[0].shape else {
        panic!("missing sample")
    };
    assert_eq!(sample.color, expected);
    assert_eq!(state.annotations[0].color, expected);
    state.undo(&mut 0);
    assert_eq!(
        state.annotations[0].color,
        tiny_skia::Color::from_rgba8(32, 67, 92, 255)
    );
}
