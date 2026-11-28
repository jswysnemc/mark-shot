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

/// 【取色工具条测试】【色块绘制】无参数，逐次采样后核对色号左侧的实际像素，色块和文本仍属于同一复制按钮，无返回值
#[test]
fn eyedropper_hex_chip_paints_the_current_sample_next_to_the_code() {
    use crate::ui::settings_panel::{SettingsWidget, ValueField, draw_settings_panel};
    let mut state = editor();
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    select_tool(&mut state, Tool::Eyedropper, &mut 0);
    for (offset, rgb) in [(0.0, [108, 99, 88]), (40.0, [32, 170, 67])] {
        let expected = tiny_skia::Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255);
        let mut image = tiny_skia::Pixmap::new(640, 480).unwrap();
        image.fill(expected);
        state.captures[0].pixmap = image.into();
        pointer(&mut state, 181.0 + offset, 172.0, &mut 0);
        crate::app::panels::settings::update_settings_panel(&mut state, &mut 0);
        let current = crate::app::panels::settings::current_color(&state);
        let panel = &mut state.settings_panel;
        let index = panel
            .widgets
            .iter()
            .position(|item| {
                matches!(
                    item,
                    SettingsWidget::Value {
                        field: ValueField::Hex
                    }
                )
            })
            .unwrap();
        let cell = panel.widget_rect(index).unwrap();
        let point = (cell.left() + 15.0, cell.top() + cell.height() * 0.5);
        let mut canvas = tiny_skia::Pixmap::new(640, 480).unwrap();
        draw_settings_panel(
            &mut crate::renderer::Painter::new(&mut canvas).with_text(
                &mut state.text.font_system,
                &mut cosmic_text::SwashCache::new(),
            ),
            panel,
            current,
            tiny_skia::Color::BLACK,
        );
        assert_eq!(
            canvas
                .pixel(point.0 as u32, point.1 as u32)
                .unwrap()
                .demultiply(),
            expected.to_color_u8()
        );
        assert_eq!(
            panel.hit_test((point.0.into(), point.1.into())),
            (true, Some(index))
        );
    }
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

/// 【取色回归】【移动标签】单独拖动标签不改变原图取色点、读数和标签尺寸，撤销恢复标签位置
#[test]
fn moving_the_sample_label_preserves_the_source_and_its_color() {
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
    let expected = tiny_skia::Color::from_rgba8(32, 67, 92, 255);
    assert_eq!(
        crate::app::panels::settings::current_color(&state),
        expected
    );
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    let AnnotationShape::ColorSample { sample } = state.annotations[0].shape else {
        panic!("missing sample")
    };
    assert_eq!(sample.color, expected);
    assert_eq!(
        (sample.source, sample.label),
        ((180.0, 170.0), (280.0, 200.0))
    );
    assert_eq!(state.annotations[0].color, expected);
    state.undo(&mut 0);
    let AnnotationShape::ColorSample { sample } = state.annotations[0].shape else {
        panic!("expected sample")
    };
    assert_eq!(sample.label, (200.0, 200.0));
    assert_eq!(
        state.annotations[0].color,
        tiny_skia::Color::from_rgba8(32, 67, 92, 255)
    );
}
/// 【取色测试】【独立指针】取色与选择工具都可以拖动已有取样点；圆镜随指针显示，标签不动，松开只产生一次撤销
#[test]
fn dragging_sample_tip_keeps_its_label_and_resamples_with_a_loupe() {
    for tool in [Tool::Eyedropper, Tool::Pick] {
        let mut state = editor();
        let mut dirty = 0;
        select_tool(&mut state, Tool::Eyedropper, &mut dirty);
        pointer(&mut state, 160.0, 150.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 280.0, 250.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        let original = state.annotations[0].clone();
        let mut image = tiny_skia::Pixmap::new(640, 480).unwrap();
        let expected = tiny_skia::Color::from_rgba8(210, 83, 60, 255);
        image.fill(expected);
        state.captures[0].pixmap = image.into();
        if tool == Tool::Pick {
            select_tool(&mut state, tool, &mut dirty);
        }
        pointer(&mut state, 160.0, 150.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 354.0, 192.0, &mut dirty);
        assert!(state.magnifier_shown());
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert_eq!(state.annotations.len(), 1);
        let AnnotationShape::ColorSample { sample } = state.annotations[0].shape else {
            panic!("expected sample")
        };
        assert_eq!(
            (sample.source, sample.label, sample.color),
            ((354.0, 192.0), (280.0, 250.0), expected)
        );
        assert_eq!(state.magnifier_shown(), tool == Tool::Eyedropper);
        assert_eq!(state.undo_stack.len(), 2);
        state.undo(&mut dirty);
        assert_eq!(state.annotations[0], original);
    }
}

/// 【取色测试】【连续拖动】单击创建标注后，直接按住面板外的原取色点可以移动指针，无需先额外点击关闭面板
#[test]
fn dragging_a_tip_outside_the_open_inspector_uses_the_first_press() {
    let mut state = editor();
    select_tool(&mut state, Tool::Eyedropper, &mut 0);
    pointer(&mut state, 530.0, 330.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    let (_, panel) = state.color_inspector.panel.unwrap();
    assert!(
        !crate::features::color_sample::inspector::contains(panel, state.input.pointer.local),
        "{panel:?}"
    );
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    assert!(state.ann_drag.is_some(), "tip press was consumed");
    pointer(&mut state, 210.0, 288.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    let AnnotationShape::ColorSample { sample } = state.annotations[0].shape else {
        panic!("expected sample")
    };
    assert_eq!(sample.source, (210.0, 288.0));
    assert_eq!(state.annotations.len(), 1);
}

/// 【取色测试】【选框分离】选框只包围标签，悬空指针仍可命中，引线与选框边的延长线不截获画布操作
#[test]
fn sample_frame_and_hit_area_exclude_the_detached_pointer() {
    use crate::types::SelectionHandle;
    let mut state = editor();
    select_tool(&mut state, Tool::Eyedropper, &mut 0);
    pointer(&mut state, 150.0, 130.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 330.0, 280.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    let annotation = &state.annotations[0];
    let AnnotationShape::ColorSample { sample } = annotation.shape else {
        panic!("expected sample")
    };
    let frame =
        crate::features::annotation_selection::frame::bounds(annotation, state.hit_window_units())
            .unwrap();
    assert_eq!(Some(frame), sample.label_rect().outset(6.0, 6.0));
    assert_eq!(
        state.canvas_handle(annotation, (150.0, 130.0)),
        SelectionHandle::ColorSampleTip
    );
    assert!(!state.canvas_body_hit(annotation, (245.0, 200.0)));
    assert_eq!(
        state.canvas_handle(annotation, (330.0, 180.0)),
        SelectionHandle::None
    );
    let damage = annotation.damage_bbox(false);
    assert!(damage.left() < 150.0 && damage.right() >= sample.label_rect().right());
    // 1. 【取色测试】【专用工具拖动】在取色工具下拖标签不新建采样，不显示取色圆镜
    pointer(&mut state, 365.0, 295.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 385.0, 305.0, &mut 0);
    assert!(!state.magnifier_shown());
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    assert_eq!(state.annotations.len(), 1);
    let AnnotationShape::ColorSample { sample: moved } = state.annotations[0].shape else {
        panic!("expected sample")
    };
    assert_eq!(moved.source, sample.source);
    assert_eq!(moved.label, (350.0, 290.0));
}

/// 【取色测试】【旋转与缩放】标签的旋转和尺寸操作不应让指针转动或改变色值，圆镜只跟随取色点手势
#[test]
fn rotating_and_resizing_color_label_keeps_visual_source_fixed() {
    use crate::{
        editor::annotation_drag::{
            apply_annotation_drag, begin_drag_with_handle, commit_drag_if_changed,
        },
        types::SelectionHandle,
    };
    let mut state = editor();
    select_tool(&mut state, Tool::Eyedropper, &mut 0);
    pointer(&mut state, 160.0, 150.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 300.0, 245.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    select_tool(&mut state, Tool::Pick, &mut 0);
    let original = state.annotations[0].clone();
    for (handle, start, end) in [
        (SelectionHandle::Rotate, (363.0, 210.0), (405.0, 260.0)),
        (SelectionHandle::BottomRight, (370.0, 305.0), (395.0, 325.0)),
    ] {
        pointer(&mut state, start.0, start.1, &mut 0);
        begin_drag_with_handle(&mut state, 0, handle);
        apply_annotation_drag(&mut state, end);
        commit_drag_if_changed(&mut state);
        let annotation = &state.annotations[0];
        let AnnotationShape::ColorSample { sample } = annotation.shape else {
            panic!("expected sample")
        };
        let source = annotation.to_visual(sample.source);
        assert!((source.0 - 160.0).hypot(source.1 - 150.0) < 0.001);
        assert_eq!(annotation.color, original.color);
        assert!(annotation.rotation_degrees.abs() > 1.0);
        if handle == SelectionHandle::BottomRight {
            let AnnotationShape::ColorSample { sample: old } = original.shape else {
                unreachable!()
            };
            assert_ne!(sample.size, old.size);
        }
    }
}
